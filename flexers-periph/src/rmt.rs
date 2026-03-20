use flexers_core::memory::MmioHandler;
use crate::interrupt::{InterruptSource, InterruptRaiser};
use std::sync::{Arc, Mutex};

/// RMT register offsets (per channel)
const RMT_CH0CONF0_REG: u32 = 0x000;      // Channel 0 configuration 0
const RMT_CH0CONF1_REG: u32 = 0x004;      // Channel 0 configuration 1
const RMT_CH0STATUS_REG: u32 = 0x008;     // Channel 0 status
const RMT_CH0ADDR_REG: u32 = 0x00C;       // Channel 0 memory address

const CHANNEL_CONF_OFFSET: u32 = 0x10;    // Offset between channel configs

/// RMT data memory base (64 items × 8 channels = 512 items)
const RMT_DATA_BASE: u32 = 0x800;
const RMT_DATA_SIZE: usize = 64;          // 64 items per channel

/// Global RMT registers
const RMT_INT_RAW_REG: u32 = 0x0A0;       // Interrupt raw status
const RMT_INT_ENA_REG: u32 = 0x0A4;       // Interrupt enable
const RMT_INT_CLR_REG: u32 = 0x0A8;       // Interrupt clear

/// Configuration 0 bits
const RMT_DIV_CNT_MASK: u32 = 0xFF;       // Clock divider (bits 0-7)
const RMT_MEM_SIZE_MASK: u32 = 0xF << 24; // Memory size (bits 24-27)
const RMT_CARRIER_EN: u32 = 1 << 28;      // Carrier enable
const RMT_TX_START: u32 = 1 << 29;        // Start TX
const RMT_MEM_PD: u32 = 1 << 30;          // Memory power down

/// Configuration 1 bits
const RMT_TX_CONTI_MODE: u32 = 1 << 0;    // Continuous mode (loop)
const RMT_REF_ALWAYS_ON: u32 = 1 << 1;    // Reference clock always on
const RMT_IDLE_OUT_LV: u32 = 1 << 2;      // Idle output level
const RMT_IDLE_OUT_EN: u32 = 1 << 3;      // Idle output enable

/// Interrupt bits (per channel)
fn rmt_int_tx_end(ch: u8) -> u32 { 1 << (ch * 3) }
fn rmt_int_rx_end(ch: u8) -> u32 { 1 << (ch * 3 + 1) }
fn rmt_int_err(ch: u8) -> u32 { 1 << (ch * 3 + 2) }

/// RMT item format
/// Each item represents two levels with durations
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RmtItem {
    /// First level (high or low)
    pub level0: bool,

    /// First duration (in clock ticks)
    pub duration0: u16,

    /// Second level (high or low)
    pub level1: bool,

    /// Second duration (in clock ticks)
    pub duration1: u16,
}

impl RmtItem {
    /// Create a new RMT item
    pub fn new(level0: bool, duration0: u16, level1: bool, duration1: u16) -> Self {
        Self {
            level0,
            duration0,
            level1,
            duration1,
        }
    }

    /// Serialize to 32-bit value for memory storage
    pub fn to_u32(&self) -> u32 {
        let level0_bit = if self.level0 { 1u32 << 15 } else { 0 };
        let level1_bit = if self.level1 { 1u32 << 31 } else { 0 };

        (self.duration0 as u32 & 0x7FFF) | level0_bit |
        ((self.duration1 as u32 & 0x7FFF) << 16) | level1_bit
    }

    /// Deserialize from 32-bit value
    pub fn from_u32(val: u32) -> Self {
        Self {
            duration0: (val & 0x7FFF) as u16,
            level0: (val & 0x8000) != 0,
            duration1: ((val >> 16) & 0x7FFF) as u16,
            level1: (val & 0x80000000) != 0,
        }
    }
}

/// RMT mode (transmit or receive)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RmtMode {
    Tx,  // Transmit mode
    Rx,  // Receive mode
}

/// RMT channel
#[derive(Debug, Clone)]
struct RmtChannel {
    /// Channel mode (TX or RX)
    mode: RmtMode,

    /// Configuration register 0
    conf0: u32,

    /// Configuration register 1
    conf1: u32,

    /// Status register
    status: u32,

    /// Memory address pointer
    addr: u32,

    /// Clock divider (1-255)
    clock_div: u8,

    /// Memory size (number of 64-item blocks)
    mem_size: u8,

    /// Carrier enabled
    carrier_en: bool,

    /// Carrier frequency (Hz)
    carrier_freq: u32,

    /// Idle output level
    idle_level: bool,

    /// Continuous/loop mode
    continuous: bool,

    /// TX in progress
    transmitting: bool,

    /// Memory items (64 items)
    memory: Vec<RmtItem>,
}

impl RmtChannel {
    fn new() -> Self {
        Self {
            mode: RmtMode::Tx,
            conf0: 0,
            conf1: 0,
            status: 0,
            addr: 0,
            clock_div: 1,
            mem_size: 1,
            carrier_en: false,
            carrier_freq: 38000, // Default 38kHz for IR
            idle_level: false,
            continuous: false,
            transmitting: false,
            memory: vec![RmtItem::new(false, 0, false, 0); RMT_DATA_SIZE],
        }
    }

    /// Update configuration from conf0 register
    fn update_conf0(&mut self, val: u32) {
        self.conf0 = val;
        self.clock_div = (val & RMT_DIV_CNT_MASK) as u8;
        self.mem_size = ((val & RMT_MEM_SIZE_MASK) >> 24) as u8;
        self.carrier_en = (val & RMT_CARRIER_EN) != 0;

        if (val & RMT_TX_START) != 0 {
            self.start_tx();
        }
    }

    /// Update configuration from conf1 register
    fn update_conf1(&mut self, val: u32) {
        self.conf1 = val;
        self.continuous = (val & RMT_TX_CONTI_MODE) != 0;
        self.idle_level = (val & RMT_IDLE_OUT_LV) != 0;
    }

    /// Start transmission
    fn start_tx(&mut self) {
        self.transmitting = true;
        self.addr = 0;
        // In real hardware, this would trigger actual pin transitions
        // For emulation, we just mark as transmitting
    }

    /// Stop transmission
    fn stop_tx(&mut self) {
        self.transmitting = false;
    }

    /// Write item to memory
    fn write_memory(&mut self, index: usize, item: RmtItem) {
        if index < self.memory.len() {
            self.memory[index] = item;
        }
    }

    /// Read item from memory
    fn read_memory(&self, index: usize) -> RmtItem {
        if index < self.memory.len() {
            self.memory[index]
        } else {
            RmtItem::new(false, 0, false, 0)
        }
    }
}

/// RMT peripheral with 8 channels
pub struct Rmt {
    /// 8 RMT channels
    channels: [RmtChannel; 8],

    /// Interrupt raw register
    int_raw: u32,

    /// Interrupt enable register
    int_ena: u32,

    /// Interrupt raiser
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,
}

impl Rmt {
    /// Create a new RMT peripheral
    pub fn new() -> Self {
        Self {
            channels: [
                RmtChannel::new(), RmtChannel::new(), RmtChannel::new(), RmtChannel::new(),
                RmtChannel::new(), RmtChannel::new(), RmtChannel::new(), RmtChannel::new(),
            ],
            int_raw: 0,
            int_ena: 0,
            int_raiser: None,
        }
    }

    /// Set interrupt raiser
    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Load items into a channel's memory
    pub fn load_items(&mut self, channel: u8, items: &[RmtItem]) {
        if (channel as usize) < self.channels.len() {
            let ch = &mut self.channels[channel as usize];
            for (i, &item) in items.iter().enumerate().take(RMT_DATA_SIZE) {
                ch.write_memory(i, item);
            }
        }
    }

    /// Start transmission on a channel
    pub fn transmit(&mut self, channel: u8) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].start_tx();
        }
    }

    /// Simulate transmission complete (for testing)
    pub fn complete_tx(&mut self, channel: u8) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].stop_tx();
            self.int_raw |= rmt_int_tx_end(channel);
            self.check_and_raise_interrupt();
        }
    }

    /// Check and raise interrupt if needed
    fn check_and_raise_interrupt(&mut self) {
        if (self.int_raw & self.int_ena) != 0 {
            if let Some(ref raiser) = self.int_raiser {
                if let Ok(mut r) = raiser.lock() {
                    r.raise(InterruptSource::Rmt);
                }
            }
        }
    }

    /// Read channel configuration register
    fn read_channel_conf(&self, channel: u8, reg: u32) -> u32 {
        if (channel as usize) >= self.channels.len() {
            return 0;
        }

        let ch = &self.channels[channel as usize];
        match reg {
            0 => ch.conf0,
            4 => ch.conf1,
            8 => ch.status,
            12 => ch.addr,
            _ => 0,
        }
    }

    /// Write channel configuration register
    fn write_channel_conf(&mut self, channel: u8, reg: u32, val: u32) {
        if (channel as usize) >= self.channels.len() {
            return;
        }

        let ch = &mut self.channels[channel as usize];
        match reg {
            0 => ch.update_conf0(val),
            4 => ch.update_conf1(val),
            8 => ch.status = val,
            12 => ch.addr = val,
            _ => {},
        }
    }
}

impl MmioHandler for Rmt {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        let offset = addr & 0xFFF;

        // Check if reading channel configuration
        if offset < 0x080 {
            let channel = (offset / CHANNEL_CONF_OFFSET) as u8;
            let reg = offset % CHANNEL_CONF_OFFSET;
            return self.read_channel_conf(channel, reg);
        }

        // Check if reading interrupt registers
        match offset {
            RMT_INT_RAW_REG => self.int_raw,
            RMT_INT_ENA_REG => self.int_ena,
            _ if offset >= RMT_DATA_BASE && offset < RMT_DATA_BASE + 2048 => {
                // Reading RMT memory (512 items × 4 bytes = 2048 bytes)
                let item_offset = (offset - RMT_DATA_BASE) / 4;
                let channel = (item_offset / RMT_DATA_SIZE as u32) as usize;
                let item_idx = (item_offset % RMT_DATA_SIZE as u32) as usize;

                if channel < self.channels.len() {
                    self.channels[channel].read_memory(item_idx).to_u32()
                } else {
                    0
                }
            },
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        let offset = addr & 0xFFF;

        // Check if writing channel configuration
        if offset < 0x080 {
            let channel = (offset / CHANNEL_CONF_OFFSET) as u8;
            let reg = offset % CHANNEL_CONF_OFFSET;
            self.write_channel_conf(channel, reg, val);
            return;
        }

        // Check if writing interrupt registers
        match offset {
            RMT_INT_RAW_REG => {
                // Writing to int_raw clears the bits
                self.int_raw &= !val;
            },
            RMT_INT_ENA_REG => {
                self.int_ena = val;
            },
            RMT_INT_CLR_REG => {
                self.int_raw &= !val;
            },
            _ if offset >= RMT_DATA_BASE && offset < RMT_DATA_BASE + 2048 => {
                // Writing RMT memory
                let item_offset = (offset - RMT_DATA_BASE) / 4;
                let channel = (item_offset / RMT_DATA_SIZE as u32) as usize;
                let item_idx = (item_offset % RMT_DATA_SIZE as u32) as usize;

                if channel < self.channels.len() {
                    let item = RmtItem::from_u32(val);
                    self.channels[channel].write_memory(item_idx, item);
                }
            },
            _ => {},
        }
    }
}

impl Default for Rmt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rmt_item_creation() {
        let item = RmtItem::new(true, 100, false, 200);
        assert_eq!(item.level0, true);
        assert_eq!(item.duration0, 100);
        assert_eq!(item.level1, false);
        assert_eq!(item.duration1, 200);
    }

    #[test]
    fn test_rmt_item_serialization() {
        let item = RmtItem::new(true, 500, false, 1000);
        let val = item.to_u32();
        let item2 = RmtItem::from_u32(val);

        assert_eq!(item, item2);
    }

    #[test]
    fn test_rmt_creation() {
        let rmt = Rmt::new();
        assert_eq!(rmt.channels.len(), 8);
    }

    #[test]
    fn test_rmt_channel_clock_div() {
        let mut rmt = Rmt::new();

        // Set clock divider to 80 (for 1MHz from 80MHz)
        let conf0 = 80u32; // Clock divider in bits 0-7
        rmt.write(RMT_CH0CONF0_REG, 4, conf0);

        assert_eq!(rmt.channels[0].clock_div, 80);
    }

    #[test]
    fn test_rmt_carrier_enable() {
        let mut rmt = Rmt::new();

        rmt.write(RMT_CH0CONF0_REG, 4, RMT_CARRIER_EN);
        assert!(rmt.channels[0].carrier_en);
    }

    #[test]
    fn test_rmt_idle_level() {
        let mut rmt = Rmt::new();

        rmt.write(RMT_CH0CONF1_REG, 4, RMT_IDLE_OUT_LV);
        assert!(rmt.channels[0].idle_level);
    }

    #[test]
    fn test_rmt_continuous_mode() {
        let mut rmt = Rmt::new();

        rmt.write(RMT_CH0CONF1_REG, 4, RMT_TX_CONTI_MODE);
        assert!(rmt.channels[0].continuous);
    }

    #[test]
    fn test_rmt_memory_write_read() {
        let mut rmt = Rmt::new();

        let item = RmtItem::new(true, 100, false, 200);

        // Write to channel 0, item 0
        rmt.write(RMT_DATA_BASE, 4, item.to_u32());

        // Read back
        let val = rmt.read(RMT_DATA_BASE, 4);
        let item2 = RmtItem::from_u32(val);

        assert_eq!(item, item2);
    }

    #[test]
    fn test_rmt_load_items() {
        let mut rmt = Rmt::new();

        let items = [
            RmtItem::new(true, 100, false, 100),
            RmtItem::new(true, 200, false, 200),
            RmtItem::new(true, 300, false, 300),
        ];

        rmt.load_items(0, &items);

        assert_eq!(rmt.channels[0].memory[0], items[0]);
        assert_eq!(rmt.channels[0].memory[1], items[1]);
        assert_eq!(rmt.channels[0].memory[2], items[2]);
    }

    #[test]
    fn test_rmt_transmit() {
        let mut rmt = Rmt::new();

        rmt.transmit(0);
        assert!(rmt.channels[0].transmitting);
    }

    #[test]
    fn test_rmt_tx_complete() {
        let mut rmt = Rmt::new();
        rmt.int_ena = rmt_int_tx_end(0);

        rmt.transmit(0);
        rmt.complete_tx(0);

        assert!(!rmt.channels[0].transmitting);
        assert_ne!(rmt.int_raw & rmt_int_tx_end(0), 0);
    }

    #[test]
    fn test_rmt_interrupt_enable() {
        let mut rmt = Rmt::new();

        rmt.write(RMT_INT_ENA_REG, 4, rmt_int_tx_end(0) | rmt_int_tx_end(1));
        assert_eq!(rmt.int_ena, rmt_int_tx_end(0) | rmt_int_tx_end(1));
    }

    #[test]
    fn test_rmt_interrupt_clear() {
        let mut rmt = Rmt::new();

        rmt.int_raw = rmt_int_tx_end(0) | rmt_int_rx_end(0);

        // Clear TX end interrupt
        rmt.write(RMT_INT_RAW_REG, 4, rmt_int_tx_end(0));

        assert_eq!(rmt.int_raw, rmt_int_rx_end(0));
    }

    #[test]
    fn test_rmt_multiple_channels() {
        let mut rmt = Rmt::new();

        // Configure different channels
        rmt.write(RMT_CH0CONF0_REG, 4, 80);
        rmt.write(RMT_CH0CONF0_REG + CHANNEL_CONF_OFFSET, 4, 40);

        assert_eq!(rmt.channels[0].clock_div, 80);
        assert_eq!(rmt.channels[1].clock_div, 40);
    }

    #[test]
    fn test_rmt_ws2812_pattern() {
        // WS2812 LED pattern: T0H=400ns, T0L=850ns, T1H=800ns, T1L=450ns
        // At 80MHz with div=1, 1 tick = 12.5ns
        // T0H = 32 ticks, T0L = 68 ticks
        // T1H = 64 ticks, T1L = 36 ticks

        let mut rmt = Rmt::new();

        let bit_0 = RmtItem::new(true, 32, false, 68);
        let bit_1 = RmtItem::new(true, 64, false, 36);

        // Load a byte pattern (e.g., 0b10101010)
        let items = [bit_1, bit_0, bit_1, bit_0, bit_1, bit_0, bit_1, bit_0];

        rmt.load_items(0, &items);

        assert_eq!(rmt.channels[0].memory[0], bit_1);
        assert_eq!(rmt.channels[0].memory[1], bit_0);
    }
}
