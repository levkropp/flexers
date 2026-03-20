use flexers_core::memory::MmioHandler;
use crate::interrupt::{InterruptSource, InterruptRaiser};
use std::sync::{Arc, Mutex};

/// DMA register offsets (per channel)
const DMA_IN_CONF0_CH0: u32 = 0x000;    // RX channel 0 configuration 0
const DMA_IN_CONF1_CH0: u32 = 0x004;    // RX channel 0 configuration 1
const DMA_IN_LINK_CH0: u32 = 0x008;     // RX channel 0 descriptor link
const DMA_IN_STATE_CH0: u32 = 0x00C;    // RX channel 0 state
const DMA_IN_INT_RAW_CH0: u32 = 0x010;  // RX channel 0 interrupt raw
const DMA_IN_INT_ENA_CH0: u32 = 0x014;  // RX channel 0 interrupt enable

const DMA_OUT_CONF0_CH0: u32 = 0x070;   // TX channel 0 configuration 0
const DMA_OUT_CONF1_CH0: u32 = 0x074;   // TX channel 0 configuration 1
const DMA_OUT_LINK_CH0: u32 = 0x078;    // TX channel 0 descriptor link
const DMA_OUT_STATE_CH0: u32 = 0x07C;   // TX channel 0 state
const DMA_OUT_INT_RAW_CH0: u32 = 0x080; // TX channel 0 interrupt raw
const DMA_OUT_INT_ENA_CH0: u32 = 0x084; // TX channel 0 interrupt enable

const CHANNEL_OFFSET: u32 = 0x18;       // Offset between channels

/// Configuration bits
const DMA_IN_RST: u32 = 1 << 0;         // Reset RX channel
const DMA_OUT_RST: u32 = 1 << 0;        // Reset TX channel
const DMA_IN_START: u32 = 1 << 1;       // Start RX transfer
const DMA_OUT_START: u32 = 1 << 1;      // Start TX transfer
const DMA_IN_LOOP_TEST: u32 = 1 << 2;   // Loop test mode
const DMA_OUT_LOOP_TEST: u32 = 1 << 2;  // Loop test mode

/// Link register bits
const DMA_INLINK_ADDR_MASK: u32 = 0xFFFFF;    // Descriptor address (20 bits)
const DMA_INLINK_START: u32 = 1 << 28;        // Start RX descriptor
const DMA_INLINK_STOP: u32 = 1 << 29;         // Stop RX descriptor
const DMA_INLINK_RESTART: u32 = 1 << 30;      // Restart RX descriptor
const DMA_OUTLINK_ADDR_MASK: u32 = 0xFFFFF;   // Descriptor address (20 bits)
const DMA_OUTLINK_START: u32 = 1 << 28;       // Start TX descriptor
const DMA_OUTLINK_STOP: u32 = 1 << 29;        // Stop TX descriptor
const DMA_OUTLINK_RESTART: u32 = 1 << 30;     // Restart TX descriptor

/// Interrupt bits
const DMA_INT_DONE: u32 = 1 << 0;             // Transfer done
const DMA_INT_EOF: u32 = 1 << 1;              // End of frame
const DMA_INT_DSCR_ERR: u32 = 1 << 2;         // Descriptor error
const DMA_INT_DSCR_EMPTY: u32 = 1 << 3;       // Descriptor empty

/// DMA descriptor structure (in memory)
/// Each descriptor is 12 bytes (3 words)
#[derive(Debug, Clone, Copy)]
pub struct DmaDescriptor {
    /// Size of buffer (in bytes)
    pub size: u16,

    /// Actual length of data (in bytes)
    pub length: u16,

    /// Pointer to data buffer
    pub buffer_ptr: u32,

    /// Pointer to next descriptor (0 = end of list)
    pub next_ptr: u32,

    /// Flags (owner, EOF, etc.)
    pub flags: u32,
}

impl DmaDescriptor {
    /// Create a new descriptor
    pub fn new(buffer_ptr: u32, size: u16) -> Self {
        Self {
            size,
            length: 0,
            buffer_ptr,
            next_ptr: 0,
            flags: 0,
        }
    }

    /// Check if this is the end of the descriptor chain
    pub fn is_end(&self) -> bool {
        self.next_ptr == 0
    }

    /// Check if EOF flag is set
    pub fn is_eof(&self) -> bool {
        (self.flags & 0x1) != 0
    }

    /// Set EOF flag
    pub fn set_eof(&mut self, eof: bool) {
        if eof {
            self.flags |= 0x1;
        } else {
            self.flags &= !0x1;
        }
    }

    /// Serialize to bytes for memory storage
    pub fn to_bytes(&self) -> [u8; 12] {
        let mut bytes = [0u8; 12];

        // Word 0: size (2 bytes) + length (2 bytes)
        bytes[0..2].copy_from_slice(&self.size.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.length.to_le_bytes());

        // Word 1: buffer_ptr
        bytes[4..8].copy_from_slice(&self.buffer_ptr.to_le_bytes());

        // Word 2: next_ptr (20 bits) + flags (12 bits)
        let word2 = (self.next_ptr & 0xFFFFF) | ((self.flags & 0xFFF) << 20);
        bytes[8..12].copy_from_slice(&word2.to_le_bytes());

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= 12, "DMA descriptor must be at least 12 bytes");

        let size = u16::from_le_bytes([bytes[0], bytes[1]]);
        let length = u16::from_le_bytes([bytes[2], bytes[3]]);
        let buffer_ptr = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let word2 = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        let next_ptr = word2 & 0xFFFFF;
        let flags = (word2 >> 20) & 0xFFF;

        Self {
            size,
            length,
            buffer_ptr,
            next_ptr,
            flags,
        }
    }
}

/// DMA channel state
#[derive(Debug, Clone)]
struct DmaChannel {
    /// Channel enabled
    enabled: bool,

    /// Configuration register 0
    conf0: u32,

    /// Configuration register 1
    conf1: u32,

    /// Link register (descriptor address + control bits)
    link: u32,

    /// State register
    state: u32,

    /// Interrupt raw register
    int_raw: u32,

    /// Interrupt enable register
    int_ena: u32,

    /// Current descriptor address
    current_desc_addr: u32,

    /// Bytes transferred in current descriptor
    bytes_transferred: usize,

    /// Transfer active
    active: bool,

    /// Circular buffer mode
    circular: bool,
}

impl DmaChannel {
    fn new() -> Self {
        Self {
            enabled: false,
            conf0: 0,
            conf1: 0,
            link: 0,
            state: 0,
            int_raw: 0,
            int_ena: 0,
            current_desc_addr: 0,
            bytes_transferred: 0,
            active: false,
            circular: false,
        }
    }

    /// Reset the channel
    fn reset(&mut self) {
        self.conf0 = 0;
        self.conf1 = 0;
        self.link = 0;
        self.state = 0;
        self.int_raw = 0;
        self.current_desc_addr = 0;
        self.bytes_transferred = 0;
        self.active = false;
    }

    /// Start a transfer
    fn start(&mut self, desc_addr: u32) {
        self.current_desc_addr = desc_addr;
        self.bytes_transferred = 0;
        self.active = true;
        self.state = 1; // FSM in running state
    }

    /// Stop a transfer
    fn stop(&mut self) {
        self.active = false;
        self.state = 0;
    }

    /// Mark transfer as complete
    fn complete(&mut self) {
        self.active = false;
        self.state = 0;
        self.int_raw |= DMA_INT_DONE;
    }

    /// Mark end of frame
    fn eof(&mut self) {
        self.int_raw |= DMA_INT_EOF;
    }

    /// Check if interrupt should be raised
    fn should_raise_interrupt(&self) -> bool {
        (self.int_raw & self.int_ena) != 0
    }
}

/// DMA controller with multiple channels
pub struct Dma {
    /// RX channels (0-7)
    rx_channels: [DmaChannel; 8],

    /// TX channels (0-7)
    tx_channels: [DmaChannel; 8],

    /// Interrupt raiser (shared with interrupt controller)
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,
}

impl Dma {
    /// Create a new DMA controller
    pub fn new() -> Self {
        Self {
            rx_channels: [
                DmaChannel::new(), DmaChannel::new(), DmaChannel::new(), DmaChannel::new(),
                DmaChannel::new(), DmaChannel::new(), DmaChannel::new(), DmaChannel::new(),
            ],
            tx_channels: [
                DmaChannel::new(), DmaChannel::new(), DmaChannel::new(), DmaChannel::new(),
                DmaChannel::new(), DmaChannel::new(), DmaChannel::new(), DmaChannel::new(),
            ],
            int_raiser: None,
        }
    }

    /// Set interrupt raiser
    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Start a RX (receive) transfer on a channel
    pub fn start_rx_transfer(&mut self, channel: u8, desc_addr: u32) {
        if (channel as usize) < self.rx_channels.len() {
            self.rx_channels[channel as usize].start(desc_addr);
        }
    }

    /// Start a TX (transmit) transfer on a channel
    pub fn start_tx_transfer(&mut self, channel: u8, desc_addr: u32) {
        if (channel as usize) < self.tx_channels.len() {
            self.tx_channels[channel as usize].start(desc_addr);
        }
    }

    /// Stop a RX transfer
    pub fn stop_rx_transfer(&mut self, channel: u8) {
        if (channel as usize) < self.rx_channels.len() {
            self.rx_channels[channel as usize].stop();
        }
    }

    /// Stop a TX transfer
    pub fn stop_tx_transfer(&mut self, channel: u8) {
        if (channel as usize) < self.tx_channels.len() {
            self.tx_channels[channel as usize].stop();
        }
    }

    /// Simulate completion of a transfer (for testing)
    pub fn complete_rx_transfer(&mut self, channel: u8, eof: bool) {
        if (channel as usize) < self.rx_channels.len() {
            self.rx_channels[channel as usize].complete();
            if eof {
                self.rx_channels[channel as usize].eof();
            }
            self.check_and_raise_interrupt();
        }
    }

    /// Simulate completion of a TX transfer (for testing)
    pub fn complete_tx_transfer(&mut self, channel: u8, eof: bool) {
        if (channel as usize) < self.tx_channels.len() {
            self.tx_channels[channel as usize].complete();
            if eof {
                self.tx_channels[channel as usize].eof();
            }
            self.check_and_raise_interrupt();
        }
    }

    /// Check if any channel needs an interrupt and raise it
    fn check_and_raise_interrupt(&mut self) {
        let mut should_raise = false;

        for ch in &self.rx_channels {
            if ch.should_raise_interrupt() {
                should_raise = true;
                break;
            }
        }

        if !should_raise {
            for ch in &self.tx_channels {
                if ch.should_raise_interrupt() {
                    should_raise = true;
                    break;
                }
            }
        }

        if should_raise {
            if let Some(ref raiser) = self.int_raiser {
                if let Ok(mut r) = raiser.lock() {
                    r.raise(InterruptSource::Dma);
                }
            }
        }
    }

    /// Get RX channel configuration
    fn read_rx_channel(&self, channel: u8, offset: u32) -> u32 {
        let ch = &self.rx_channels[channel as usize];
        match offset {
            0x00 => ch.conf0,
            0x04 => ch.conf1,
            0x08 => ch.link,
            0x0C => ch.state,
            0x10 => ch.int_raw,
            0x14 => ch.int_ena,
            _ => 0,
        }
    }

    /// Get TX channel configuration
    fn read_tx_channel(&self, channel: u8, offset: u32) -> u32 {
        let ch = &self.tx_channels[channel as usize];
        match offset {
            0x00 => ch.conf0,
            0x04 => ch.conf1,
            0x08 => ch.link,
            0x0C => ch.state,
            0x10 => ch.int_raw,
            0x14 => ch.int_ena,
            _ => 0,
        }
    }

    /// Write RX channel configuration
    fn write_rx_channel(&mut self, channel: u8, offset: u32, val: u32) {
        let ch = &mut self.rx_channels[channel as usize];
        match offset {
            0x00 => {
                ch.conf0 = val;
                if (val & DMA_IN_RST) != 0 {
                    ch.reset();
                }
                if (val & DMA_IN_START) != 0 {
                    ch.enabled = true;
                }
            },
            0x04 => ch.conf1 = val,
            0x08 => {
                ch.link = val;
                if (val & DMA_INLINK_START) != 0 {
                    let addr = val & DMA_INLINK_ADDR_MASK;
                    self.start_rx_transfer(channel, addr);
                }
                if (val & DMA_INLINK_STOP) != 0 {
                    self.stop_rx_transfer(channel);
                }
                if (val & DMA_INLINK_RESTART) != 0 {
                    let addr = val & DMA_INLINK_ADDR_MASK;
                    self.start_rx_transfer(channel, addr);
                }
            },
            0x0C => ch.state = val,
            0x10 => {
                // Writing to int_raw clears the bits
                ch.int_raw &= !val;
            },
            0x14 => ch.int_ena = val,
            _ => {},
        }
    }

    /// Write TX channel configuration
    fn write_tx_channel(&mut self, channel: u8, offset: u32, val: u32) {
        let ch = &mut self.tx_channels[channel as usize];
        match offset {
            0x00 => {
                ch.conf0 = val;
                if (val & DMA_OUT_RST) != 0 {
                    ch.reset();
                }
                if (val & DMA_OUT_START) != 0 {
                    ch.enabled = true;
                }
            },
            0x04 => ch.conf1 = val,
            0x08 => {
                ch.link = val;
                if (val & DMA_OUTLINK_START) != 0 {
                    let addr = val & DMA_OUTLINK_ADDR_MASK;
                    self.start_tx_transfer(channel, addr);
                }
                if (val & DMA_OUTLINK_STOP) != 0 {
                    self.stop_tx_transfer(channel);
                }
                if (val & DMA_OUTLINK_RESTART) != 0 {
                    let addr = val & DMA_OUTLINK_ADDR_MASK;
                    self.start_tx_transfer(channel, addr);
                }
            },
            0x0C => ch.state = val,
            0x10 => {
                // Writing to int_raw clears the bits
                ch.int_raw &= !val;
            },
            0x14 => ch.int_ena = val,
            _ => {},
        }
    }
}

impl MmioHandler for Dma {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        let offset = addr & 0xFFF;

        // Determine if this is RX or TX channel
        if offset < 0x070 {
            // RX channels (0x000 - 0x06F)
            let channel = (offset / CHANNEL_OFFSET) as u8;
            let reg_offset = offset % CHANNEL_OFFSET;

            if (channel as usize) < self.rx_channels.len() {
                self.read_rx_channel(channel, reg_offset)
            } else {
                0
            }
        } else if offset >= 0x070 && offset < 0x0E0 {
            // TX channels (0x070 - 0x0DF)
            let channel = ((offset - 0x070) / CHANNEL_OFFSET) as u8;
            let reg_offset = (offset - 0x070) % CHANNEL_OFFSET;

            if (channel as usize) < self.tx_channels.len() {
                self.read_tx_channel(channel, reg_offset)
            } else {
                0
            }
        } else {
            0
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        let offset = addr & 0xFFF;

        // Determine if this is RX or TX channel
        if offset < 0x070 {
            // RX channels (0x000 - 0x06F)
            let channel = (offset / CHANNEL_OFFSET) as u8;
            let reg_offset = offset % CHANNEL_OFFSET;

            if (channel as usize) < self.rx_channels.len() {
                self.write_rx_channel(channel, reg_offset, val);
            }
        } else if offset >= 0x070 && offset < 0x0E0 {
            // TX channels (0x070 - 0x0DF)
            let channel = ((offset - 0x070) / CHANNEL_OFFSET) as u8;
            let reg_offset = (offset - 0x070) % CHANNEL_OFFSET;

            if (channel as usize) < self.tx_channels.len() {
                self.write_tx_channel(channel, reg_offset, val);
            }
        }
    }
}

impl Default for Dma {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dma_descriptor_creation() {
        let desc = DmaDescriptor::new(0x1000, 256);
        assert_eq!(desc.buffer_ptr, 0x1000);
        assert_eq!(desc.size, 256);
        assert_eq!(desc.length, 0);
        assert_eq!(desc.next_ptr, 0);
        assert!(desc.is_end());
    }

    #[test]
    fn test_dma_descriptor_eof() {
        let mut desc = DmaDescriptor::new(0x1000, 256);
        assert!(!desc.is_eof());

        desc.set_eof(true);
        assert!(desc.is_eof());

        desc.set_eof(false);
        assert!(!desc.is_eof());
    }

    #[test]
    fn test_dma_descriptor_serialization() {
        let mut desc = DmaDescriptor::new(0x3FF00000, 512);
        desc.length = 256;
        desc.next_ptr = 0x100; // 20-bit address (must be < 0xFFFFF)
        desc.set_eof(true);

        let bytes = desc.to_bytes();
        let desc2 = DmaDescriptor::from_bytes(&bytes);

        assert_eq!(desc2.size, desc.size);
        assert_eq!(desc2.length, desc.length);
        assert_eq!(desc2.buffer_ptr, desc.buffer_ptr);
        assert_eq!(desc2.next_ptr, desc.next_ptr & 0xFFFFF); // Mask to 20 bits
        assert_eq!(desc2.is_eof(), desc.is_eof());
    }

    #[test]
    fn test_dma_channel_reset() {
        let mut ch = DmaChannel::new();
        ch.conf0 = 0x1234;
        ch.active = true;

        ch.reset();

        assert_eq!(ch.conf0, 0);
        assert!(!ch.active);
    }

    #[test]
    fn test_dma_channel_start_stop() {
        let mut ch = DmaChannel::new();

        ch.start(0x1000);
        assert!(ch.active);
        assert_eq!(ch.current_desc_addr, 0x1000);
        assert_eq!(ch.state, 1);

        ch.stop();
        assert!(!ch.active);
        assert_eq!(ch.state, 0);
    }

    #[test]
    fn test_dma_channel_complete() {
        let mut ch = DmaChannel::new();
        ch.start(0x1000);

        ch.complete();
        assert!(!ch.active);
        assert_eq!(ch.state, 0);
        assert_ne!(ch.int_raw & DMA_INT_DONE, 0);
    }

    #[test]
    fn test_dma_channel_interrupt() {
        let mut ch = DmaChannel::new();
        ch.int_ena = DMA_INT_DONE;

        assert!(!ch.should_raise_interrupt());

        ch.complete();
        assert!(ch.should_raise_interrupt());
    }

    #[test]
    fn test_dma_controller_creation() {
        let dma = Dma::new();
        assert_eq!(dma.rx_channels.len(), 8);
        assert_eq!(dma.tx_channels.len(), 8);
    }

    #[test]
    fn test_dma_rx_transfer_start() {
        let mut dma = Dma::new();

        dma.start_rx_transfer(0, 0x2000);
        assert!(dma.rx_channels[0].active);
        assert_eq!(dma.rx_channels[0].current_desc_addr, 0x2000);
    }

    #[test]
    fn test_dma_tx_transfer_start() {
        let mut dma = Dma::new();

        dma.start_tx_transfer(1, 0x3000);
        assert!(dma.tx_channels[1].active);
        assert_eq!(dma.tx_channels[1].current_desc_addr, 0x3000);
    }

    #[test]
    fn test_dma_transfer_stop() {
        let mut dma = Dma::new();

        dma.start_rx_transfer(0, 0x2000);
        assert!(dma.rx_channels[0].active);

        dma.stop_rx_transfer(0);
        assert!(!dma.rx_channels[0].active);
    }

    #[test]
    fn test_dma_transfer_complete() {
        let mut dma = Dma::new();
        dma.rx_channels[0].int_ena = DMA_INT_DONE | DMA_INT_EOF;

        dma.start_rx_transfer(0, 0x2000);
        dma.complete_rx_transfer(0, true);

        assert!(!dma.rx_channels[0].active);
        assert_ne!(dma.rx_channels[0].int_raw & DMA_INT_DONE, 0);
        assert_ne!(dma.rx_channels[0].int_raw & DMA_INT_EOF, 0);
    }

    #[test]
    fn test_dma_multiple_channels() {
        let mut dma = Dma::new();

        dma.start_rx_transfer(0, 0x2000);
        dma.start_rx_transfer(1, 0x3000);
        dma.start_tx_transfer(0, 0x4000);

        assert!(dma.rx_channels[0].active);
        assert!(dma.rx_channels[1].active);
        assert!(dma.tx_channels[0].active);
    }

    #[test]
    fn test_dma_mmio_rx_conf_write() {
        let mut dma = Dma::new();

        // Write to RX channel 0 CONF0
        dma.write(DMA_IN_CONF0_CH0, 4, DMA_IN_START);
        assert!(dma.rx_channels[0].enabled);
    }

    #[test]
    fn test_dma_mmio_tx_conf_write() {
        let mut dma = Dma::new();

        // Write to TX channel 0 CONF0
        dma.write(DMA_OUT_CONF0_CH0, 4, DMA_OUT_START);
        assert!(dma.tx_channels[0].enabled);
    }

    #[test]
    fn test_dma_mmio_link_start() {
        let mut dma = Dma::new();

        // Write to RX channel 0 LINK register with start bit
        let link_val = DMA_INLINK_START | 0x2000;
        dma.write(DMA_IN_LINK_CH0, 4, link_val);

        assert!(dma.rx_channels[0].active);
        assert_eq!(dma.rx_channels[0].current_desc_addr, 0x2000);
    }

    #[test]
    fn test_dma_mmio_interrupt_clear() {
        let mut dma = Dma::new();

        dma.rx_channels[0].int_raw = DMA_INT_DONE | DMA_INT_EOF;

        // Clear DONE interrupt
        dma.write(DMA_IN_INT_RAW_CH0, 4, DMA_INT_DONE);

        assert_eq!(dma.rx_channels[0].int_raw, DMA_INT_EOF);
    }

    #[test]
    fn test_dma_mmio_read() {
        let mut dma = Dma::new();
        dma.rx_channels[0].conf0 = 0x1234;

        let val = dma.read(DMA_IN_CONF0_CH0, 4);
        assert_eq!(val, 0x1234);
    }
}
