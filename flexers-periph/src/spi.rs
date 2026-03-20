use flexers_core::memory::MmioHandler;
use crate::interrupt::{InterruptSource, InterruptRaiser};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// SPI register offsets
const SPI_CMD_REG: u32 = 0x000;        // Command register
const SPI_ADDR_REG: u32 = 0x004;       // Address register
const SPI_CTRL_REG: u32 = 0x008;       // Control register
const SPI_CTRL1_REG: u32 = 0x00C;      // Control register 1
const SPI_CTRL2_REG: u32 = 0x010;      // Control register 2
const SPI_CLOCK_REG: u32 = 0x018;      // Clock register
const SPI_USER_REG: u32 = 0x01C;       // User-defined control
const SPI_USER1_REG: u32 = 0x020;      // User-defined control 1
const SPI_USER2_REG: u32 = 0x024;      // User-defined control 2
const SPI_MOSI_DLEN_REG: u32 = 0x028;  // MOSI data length
const SPI_MISO_DLEN_REG: u32 = 0x02C;  // MISO data length
const SPI_W0_REG: u32 = 0x080;         // Data buffer 0 (16 registers, W0-W15)
const SPI_DMA_CONF_REG: u32 = 0x100;   // DMA configuration
const SPI_DMA_OUT_LINK_REG: u32 = 0x104; // DMA TX link
const SPI_DMA_IN_LINK_REG: u32 = 0x108;  // DMA RX link

/// Command register bits
const SPI_CMD_USR: u32 = 1 << 18;      // User command start

/// User register bits
const SPI_USR_MOSI: u32 = 1 << 27;     // Enable MOSI phase
const SPI_USR_MISO: u32 = 1 << 28;     // Enable MISO phase
const SPI_USR_DUMMY: u32 = 1 << 29;    // Enable dummy phase
const SPI_USR_ADDR: u32 = 1 << 30;     // Enable address phase
const SPI_USR_COMMAND: u32 = 1 << 31;  // Enable command phase

/// Control register bits
const SPI_WR_BIT_ORDER: u32 = 1 << 25; // Write bit order (LSB/MSB first)
const SPI_RD_BIT_ORDER: u32 = 1 << 26; // Read bit order
const SPI_FREAD_DUAL: u32 = 1 << 14;   // Dual SPI mode
const SPI_FREAD_QUAD: u32 = 1 << 20;   // Quad SPI mode

/// SPI mode (CPOL/CPHA)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpiMode {
    Mode0, // CPOL=0, CPHA=0
    Mode1, // CPOL=0, CPHA=1
    Mode2, // CPOL=1, CPHA=0
    Mode3, // CPOL=1, CPHA=1
}

/// SPI data mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpiDataMode {
    Single,  // Standard SPI (1 bit)
    Dual,    // Dual SPI (2 bits)
    Quad,    // Quad SPI (4 bits)
}

/// SPI controller (models SPI2 or SPI3)
pub struct Spi {
    /// Command register
    cmd: u32,

    /// Address register
    addr: u32,

    /// Control registers
    ctrl: u32,
    ctrl1: u32,
    ctrl2: u32,

    /// Clock configuration
    clock: u32,

    /// User-defined control
    user: u32,
    user1: u32,
    user2: u32,

    /// Data length registers
    mosi_dlen: u32,
    miso_dlen: u32,

    /// TX buffer (64 bytes = 16 words)
    tx_buffer: [u32; 16],

    /// RX buffer (64 bytes = 16 words)
    rx_buffer: [u32; 16],

    /// TX FIFO for simulation
    tx_fifo: VecDeque<u8>,

    /// RX FIFO for simulation
    rx_fifo: VecDeque<u8>,

    /// DMA configuration
    dma_conf: u32,
    dma_out_link: u32,
    dma_in_link: u32,

    /// Transfer in progress
    transfer_active: bool,

    /// SPI mode
    mode: SpiMode,

    /// Data mode (single/dual/quad)
    data_mode: SpiDataMode,

    /// Interrupt raiser
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,
}

impl Spi {
    /// Create a new SPI controller
    pub fn new() -> Self {
        Self {
            cmd: 0,
            addr: 0,
            ctrl: 0,
            ctrl1: 0,
            ctrl2: 0,
            clock: 0,
            user: 0,
            user1: 0,
            user2: 0,
            mosi_dlen: 0,
            miso_dlen: 0,
            tx_buffer: [0; 16],
            rx_buffer: [0; 16],
            tx_fifo: VecDeque::new(),
            rx_fifo: VecDeque::new(),
            dma_conf: 0,
            dma_out_link: 0,
            dma_in_link: 0,
            transfer_active: false,
            mode: SpiMode::Mode0,
            data_mode: SpiDataMode::Single,
            int_raiser: None,
        }
    }

    /// Set interrupt raiser
    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Start a transfer
    fn start_transfer(&mut self) {
        self.transfer_active = true;

        // Copy TX buffer to FIFO
        let tx_bits = self.mosi_dlen + 1;
        let tx_bytes = ((tx_bits + 7) / 8) as usize;

        for i in 0..tx_bytes {
            let word_idx = i / 4;
            let byte_idx = i % 4;
            if word_idx < 16 {
                let byte = ((self.tx_buffer[word_idx] >> (byte_idx * 8)) & 0xFF) as u8;
                self.tx_fifo.push_back(byte);
            }
        }

        // Simulate transfer - in real hardware, this would clock out MOSI and clock in MISO
        // For emulation, we just copy TX to RX (loopback)
        for (i, &byte) in self.tx_fifo.iter().enumerate().take(tx_bytes) {
            if i < 64 {
                let word_idx = i / 4;
                let byte_idx = i % 4;
                self.rx_buffer[word_idx] &= !(0xFF << (byte_idx * 8));
                self.rx_buffer[word_idx] |= (byte as u32) << (byte_idx * 8);
            }
        }

        self.transfer_active = false;
        self.cmd &= !SPI_CMD_USR; // Clear command bit
    }

    /// Write to TX buffer register
    fn write_tx_buffer(&mut self, offset: u32, val: u32) {
        let idx = ((offset - SPI_W0_REG) / 4) as usize;
        if idx < 16 {
            self.tx_buffer[idx] = val;
            // Also copy to RX buffer (for loopback/testing)
            self.rx_buffer[idx] = val;
        }
    }

    /// Read from buffer register (reads from RX buffer, which mirrors TX for loopback)
    fn read_buffer(&self, offset: u32) -> u32 {
        let idx = ((offset - SPI_W0_REG) / 4) as usize;
        if idx < 16 {
            self.rx_buffer[idx]
        } else {
            0
        }
    }

    /// Set SPI mode from control register
    fn update_mode(&mut self) {
        // SPI mode is determined by clock polarity and phase bits
        // For now, default to Mode0
        self.mode = SpiMode::Mode0;
    }

    /// Set data mode from control register
    fn update_data_mode(&mut self) {
        if (self.ctrl & SPI_FREAD_QUAD) != 0 {
            self.data_mode = SpiDataMode::Quad;
        } else if (self.ctrl & SPI_FREAD_DUAL) != 0 {
            self.data_mode = SpiDataMode::Dual;
        } else {
            self.data_mode = SpiDataMode::Single;
        }
    }
}

impl MmioHandler for Spi {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        let offset = addr & 0xFFF;

        match offset {
            SPI_CMD_REG => self.cmd,
            SPI_ADDR_REG => self.addr,
            SPI_CTRL_REG => self.ctrl,
            SPI_CTRL1_REG => self.ctrl1,
            SPI_CTRL2_REG => self.ctrl2,
            SPI_CLOCK_REG => self.clock,
            SPI_USER_REG => self.user,
            SPI_USER1_REG => self.user1,
            SPI_USER2_REG => self.user2,
            SPI_MOSI_DLEN_REG => self.mosi_dlen,
            SPI_MISO_DLEN_REG => self.miso_dlen,
            SPI_DMA_CONF_REG => self.dma_conf,
            SPI_DMA_OUT_LINK_REG => self.dma_out_link,
            SPI_DMA_IN_LINK_REG => self.dma_in_link,
            _ if offset >= SPI_W0_REG && offset < SPI_W0_REG + 64 => {
                self.read_buffer(offset)
            },
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        let offset = addr & 0xFFF;

        match offset {
            SPI_CMD_REG => {
                self.cmd = val;
                if (val & SPI_CMD_USR) != 0 {
                    self.start_transfer();
                }
            },
            SPI_ADDR_REG => self.addr = val,
            SPI_CTRL_REG => {
                self.ctrl = val;
                self.update_mode();
                self.update_data_mode();
            },
            SPI_CTRL1_REG => self.ctrl1 = val,
            SPI_CTRL2_REG => self.ctrl2 = val,
            SPI_CLOCK_REG => self.clock = val,
            SPI_USER_REG => self.user = val,
            SPI_USER1_REG => self.user1 = val,
            SPI_USER2_REG => self.user2 = val,
            SPI_MOSI_DLEN_REG => self.mosi_dlen = val,
            SPI_MISO_DLEN_REG => self.miso_dlen = val,
            SPI_DMA_CONF_REG => self.dma_conf = val,
            SPI_DMA_OUT_LINK_REG => self.dma_out_link = val,
            SPI_DMA_IN_LINK_REG => self.dma_in_link = val,
            _ if offset >= SPI_W0_REG && offset < SPI_W0_REG + 64 => {
                self.write_tx_buffer(offset, val);
            },
            _ => {},
        }
    }
}

impl Default for Spi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spi_creation() {
        let spi = Spi::new();
        assert_eq!(spi.mode, SpiMode::Mode0);
        assert_eq!(spi.data_mode, SpiDataMode::Single);
    }

    #[test]
    fn test_spi_mode_configuration() {
        let spi = Spi::new();
        // Default mode 0
        assert_eq!(spi.mode, SpiMode::Mode0);
    }

    #[test]
    fn test_spi_data_mode_single() {
        let mut spi = Spi::new();
        spi.write(SPI_CTRL_REG, 4, 0);
        assert_eq!(spi.data_mode, SpiDataMode::Single);
    }

    #[test]
    fn test_spi_data_mode_dual() {
        let mut spi = Spi::new();
        spi.write(SPI_CTRL_REG, 4, SPI_FREAD_DUAL);
        assert_eq!(spi.data_mode, SpiDataMode::Dual);
    }

    #[test]
    fn test_spi_data_mode_quad() {
        let mut spi = Spi::new();
        spi.write(SPI_CTRL_REG, 4, SPI_FREAD_QUAD);
        assert_eq!(spi.data_mode, SpiDataMode::Quad);
    }

    #[test]
    fn test_spi_tx_buffer_write() {
        let mut spi = Spi::new();
        spi.write(SPI_W0_REG, 4, 0x12345678);
        assert_eq!(spi.tx_buffer[0], 0x12345678);
    }

    #[test]
    fn test_spi_rx_buffer_read() {
        let mut spi = Spi::new();
        spi.rx_buffer[0] = 0xAABBCCDD;
        let val = spi.read(SPI_W0_REG, 4);
        assert_eq!(val, 0xAABBCCDD);
    }

    #[test]
    fn test_spi_transfer_loopback() {
        let mut spi = Spi::new();

        // Set TX data
        spi.write(SPI_W0_REG, 4, 0x12345678);

        // Set data length (32 bits - 1)
        spi.write(SPI_MOSI_DLEN_REG, 4, 31);

        // Start transfer
        spi.write(SPI_CMD_REG, 4, SPI_CMD_USR);

        // Check RX buffer has loopback data
        let rx = spi.read(SPI_W0_REG, 4);
        assert_eq!(rx, 0x12345678);
    }

    #[test]
    fn test_spi_clock_register() {
        let mut spi = Spi::new();
        spi.write(SPI_CLOCK_REG, 4, 0x1234);
        assert_eq!(spi.read(SPI_CLOCK_REG, 4), 0x1234);
    }

    #[test]
    fn test_spi_user_register() {
        let mut spi = Spi::new();
        spi.write(SPI_USER_REG, 4, SPI_USR_MOSI | SPI_USR_MISO);
        assert_eq!(spi.read(SPI_USER_REG, 4), SPI_USR_MOSI | SPI_USR_MISO);
    }

    #[test]
    fn test_spi_dma_configuration() {
        let mut spi = Spi::new();
        spi.write(SPI_DMA_CONF_REG, 4, 0x100);
        assert_eq!(spi.read(SPI_DMA_CONF_REG, 4), 0x100);
    }

    #[test]
    fn test_spi_multi_word_transfer() {
        let mut spi = Spi::new();

        // Write multiple words to TX buffer
        spi.write(SPI_W0_REG, 4, 0x11111111);
        spi.write(SPI_W0_REG + 4, 4, 0x22222222);
        spi.write(SPI_W0_REG + 8, 4, 0x33333333);

        // Set data length (96 bits - 1)
        spi.write(SPI_MOSI_DLEN_REG, 4, 95);

        // Start transfer
        spi.write(SPI_CMD_REG, 4, SPI_CMD_USR);

        // Check RX buffer
        assert_eq!(spi.read(SPI_W0_REG, 4), 0x11111111);
        assert_eq!(spi.read(SPI_W0_REG + 4, 4), 0x22222222);
        assert_eq!(spi.read(SPI_W0_REG + 8, 4), 0x33333333);
    }

    #[test]
    fn test_spi_command_clear_after_transfer() {
        let mut spi = Spi::new();
        spi.write(SPI_MOSI_DLEN_REG, 4, 7); // 8 bits

        // Start transfer
        spi.write(SPI_CMD_REG, 4, SPI_CMD_USR);

        // Command bit should be cleared after transfer
        assert_eq!(spi.read(SPI_CMD_REG, 4) & SPI_CMD_USR, 0);
    }

    #[test]
    fn test_spi_buffer_bounds() {
        let mut spi = Spi::new();

        // Write to all 16 buffer registers
        for i in 0..16 {
            spi.write(SPI_W0_REG + (i * 4), 4, 0x1000 + i);
        }

        // Read back and verify
        for i in 0..16 {
            let val = spi.read(SPI_W0_REG + (i * 4), 4);
            assert_eq!(val, 0x1000 + i);
        }
    }

    #[test]
    fn test_spi_addr_register() {
        let mut spi = Spi::new();
        spi.write(SPI_ADDR_REG, 4, 0xABCD1234);
        assert_eq!(spi.read(SPI_ADDR_REG, 4), 0xABCD1234);
    }
}
