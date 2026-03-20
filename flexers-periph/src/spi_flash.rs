use flexers_core::memory::MmioHandler;
use crate::interrupt::{InterruptSource, InterruptRaiser};
use std::sync::{Arc, Mutex};

/// SPI Flash controller registers (ESP32 SPI0/SPI1)
///
/// Base addresses:
/// - SPI0: 0x3FF42000 (used for flash cache, read-only from firmware perspective)
/// - SPI1: 0x3FF43000 (general SPI for firmware flash ops)
const SPI_CMD_REG: u32 = 0x00;         // Command register
const SPI_ADDR_REG: u32 = 0x04;        // Address register
const SPI_CTRL_REG: u32 = 0x08;        // Control register
const SPI_RD_STATUS_REG: u32 = 0x10;   // Flash status
const SPI_W0_REG: u32 = 0x80;          // Data buffer W0 (64 bytes: W0-W15)

/// Flash commands (standard SPI flash opcodes)
const CMD_READ: u8 = 0x03;
const CMD_WRITE: u8 = 0x02;
const CMD_ERASE_SECTOR: u8 = 0x20;     // 4KB sector
const CMD_ERASE_BLOCK_32K: u8 = 0x52;
const CMD_ERASE_BLOCK_64K: u8 = 0xD8;
const CMD_READ_STATUS: u8 = 0x05;
const CMD_WRITE_ENABLE: u8 = 0x06;

/// SPI Flash controller
pub struct SpiFlash {
    /// Command register (includes execute bit in bit 31)
    cmd_reg: u32,

    /// Address register (24-bit flash address)
    addr_reg: u32,

    /// Control register
    ctrl_reg: u32,

    /// Flash status register
    status_reg: u32,

    /// Data buffer (64 bytes for read/write ops)
    data_buf: [u32; 16],  // W0-W15 registers

    /// Internal flash storage (shared with memory-mapped regions)
    flash_store: Arc<Mutex<Vec<u8>>>,

    /// Flash size in bytes
    flash_size: usize,

    /// Write enable latch (must be set before WRITE/ERASE)
    write_enabled: bool,

    /// Busy flag (set during operations)
    busy: bool,

    /// Interrupt raiser
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,

    /// Interrupt source ID
    int_source: InterruptSource,
}

impl SpiFlash {
    pub fn new(flash_size: usize, int_source: InterruptSource) -> Self {
        Self {
            cmd_reg: 0,
            addr_reg: 0,
            ctrl_reg: 0,
            status_reg: 0,
            data_buf: [0; 16],
            flash_store: Arc::new(Mutex::new(vec![0xFF; flash_size])), // Erased = 0xFF
            flash_size,
            write_enabled: false,
            busy: false,
            int_raiser: None,
            int_source,
        }
    }

    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Get shared reference to internal flash store (for memory mapping)
    pub fn flash_store(&self) -> Arc<Mutex<Vec<u8>>> {
        self.flash_store.clone()
    }

    /// Load flash contents from file
    pub fn load_from_file(&mut self, path: &str) -> std::io::Result<()> {
        let data = std::fs::read(path)?;
        let mut store = self.flash_store.lock().unwrap();

        // Copy loaded data to flash (up to flash_size)
        let copy_len = data.len().min(self.flash_size);
        store[..copy_len].copy_from_slice(&data[..copy_len]);

        Ok(())
    }

    /// Save flash contents to file
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let store = self.flash_store.lock().unwrap();
        std::fs::write(path, &store[..])
    }

    /// Execute SPI command (called when CMD_REG[31] is set)
    fn execute_command(&mut self) {
        let cmd = (self.cmd_reg & 0xFF) as u8;
        let addr = self.addr_reg & 0x00FFFFFF; // 24-bit address

        match cmd {
            CMD_READ => self.execute_read(addr),
            CMD_WRITE => self.execute_write(addr),
            CMD_ERASE_SECTOR => self.execute_erase_sector(addr),
            CMD_ERASE_BLOCK_32K => self.execute_erase_block(addr, 32 * 1024),
            CMD_ERASE_BLOCK_64K => self.execute_erase_block(addr, 64 * 1024),
            CMD_READ_STATUS => self.execute_read_status(),
            CMD_WRITE_ENABLE => self.write_enabled = true,
            _ => {
                // Unknown command - just clear execute bit
            }
        }

        // Clear execute bit, set done
        self.cmd_reg &= !0x8000_0000;
        self.busy = false;

        // Raise interrupt if enabled
        self.raise_interrupt();
    }

    fn execute_read(&mut self, addr: u32) {
        if addr as usize >= self.flash_size {
            return; // Out of bounds
        }

        // Copy up to 64 bytes from flash to data buffer
        let store = self.flash_store.lock().unwrap();
        let start = addr as usize;
        let end = (start + 64).min(self.flash_size);

        for (i, &byte) in store[start..end].iter().enumerate() {
            let word_idx = i / 4;
            let byte_idx = i % 4;

            if word_idx < 16 {
                // Pack bytes into u32 words (little-endian)
                self.data_buf[word_idx] &= !(0xFF << (byte_idx * 8));
                self.data_buf[word_idx] |= (byte as u32) << (byte_idx * 8);
            }
        }
    }

    fn execute_write(&mut self, addr: u32) {
        if !self.write_enabled {
            return; // Write enable latch not set
        }

        if addr as usize >= self.flash_size {
            return; // Out of bounds
        }

        // Write up to 64 bytes from data buffer to flash
        let mut store = self.flash_store.lock().unwrap();
        let start = addr as usize;

        for (word_idx, &word) in self.data_buf.iter().enumerate() {
            for byte_idx in 0..4 {
                let flash_idx = start + word_idx * 4 + byte_idx;
                if flash_idx < self.flash_size {
                    let byte = ((word >> (byte_idx * 8)) & 0xFF) as u8;
                    // Flash write: can only change 1→0, not 0→1
                    store[flash_idx] &= byte;
                }
            }
        }

        self.write_enabled = false; // Clear after write
    }

    fn execute_erase_sector(&mut self, addr: u32) {
        if !self.write_enabled {
            return;
        }

        // Erase 4KB sector (set all bytes to 0xFF)
        let sector_start = (addr & !0xFFF) as usize; // Align to 4KB
        let sector_end = (sector_start + 4096).min(self.flash_size);

        let mut store = self.flash_store.lock().unwrap();
        for byte in &mut store[sector_start..sector_end] {
            *byte = 0xFF;
        }

        self.write_enabled = false;
    }

    fn execute_erase_block(&mut self, addr: u32, size: usize) {
        if !self.write_enabled {
            return;
        }

        let block_start = (addr as usize) & !(size - 1); // Align to block size
        let block_end = (block_start + size).min(self.flash_size);

        let mut store = self.flash_store.lock().unwrap();
        for byte in &mut store[block_start..block_end] {
            *byte = 0xFF;
        }

        self.write_enabled = false;
    }

    fn execute_read_status(&mut self) {
        // Return flash status in W0 register
        // Bit 0: WIP (write in progress) = 0 (instant operations for now)
        // Bit 1: WEL (write enable latch)
        let status = if self.write_enabled { 0x02 } else { 0x00 };
        self.data_buf[0] = status;
    }

    fn raise_interrupt(&self) {
        if let Some(ref raiser) = self.int_raiser {
            if let Ok(mut raiser_lock) = raiser.lock() {
                raiser_lock.raise(self.int_source);
            }
        }
    }
}

impl MmioHandler for SpiFlash {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFF {
            0x00 => self.cmd_reg,
            0x04 => self.addr_reg,
            0x08 => self.ctrl_reg,
            0x10 => self.status_reg,
            0x80..=0xBF => {
                // SPI_W0_REG to SPI_W15_REG
                let idx = ((addr & 0xFF) - 0x80) / 4;
                if (idx as usize) < 16 {
                    self.data_buf[idx as usize]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFF {
            0x00 => {
                self.cmd_reg = val;

                // Check if execute bit (bit 31) is set
                if (val & 0x8000_0000) != 0 {
                    self.busy = true;
                    self.execute_command();
                }
            }
            0x04 => self.addr_reg = val & 0x00FFFFFF, // 24-bit address
            0x08 => self.ctrl_reg = val,
            0x10 => self.status_reg = val,
            0x80..=0xBF => {
                // SPI_W0_REG to SPI_W15_REG
                let idx = ((addr & 0xFF) - 0x80) / 4;
                if (idx as usize) < 16 {
                    self.data_buf[idx as usize] = val;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_read() {
        let mut spi = SpiFlash::new(1024, InterruptSource::Spi1);

        // Write some test data to flash
        {
            let mut store = spi.flash_store.lock().unwrap();
            store[0x100] = 0xAA;
            store[0x101] = 0xBB;
            store[0x102] = 0xCC;
            store[0x103] = 0xDD;
        }

        // Execute READ command
        spi.write(SPI_ADDR_REG, 1, 0x100);  // Address = 0x100
        spi.write(SPI_CMD_REG, 1, CMD_READ as u32);
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_READ as u32)); // Execute

        // Check data buffer
        let w0 = spi.read(SPI_W0_REG, 1);
        assert_eq!(w0, 0xDDCCBBAA); // Little-endian
    }

    #[test]
    fn test_flash_write() {
        let mut spi = SpiFlash::new(1024, InterruptSource::Spi1);

        // Enable writes
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));

        // Write data to buffer
        spi.write(SPI_W0_REG, 1, 0x12345678);

        // Execute WRITE command
        spi.write(SPI_ADDR_REG, 1, 0x200);
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_WRITE as u32));

        // Verify flash contents
        let store = spi.flash_store.lock().unwrap();
        assert_eq!(store[0x200], 0x78);
        assert_eq!(store[0x201], 0x56);
        assert_eq!(store[0x202], 0x34);
        assert_eq!(store[0x203], 0x12);
    }

    #[test]
    fn test_flash_erase_sector() {
        let mut spi = SpiFlash::new(8192, InterruptSource::Spi1);

        // Write test pattern
        {
            let mut store = spi.flash_store.lock().unwrap();
            for i in 0..4096 {
                store[i] = 0xAA;
            }
        }

        // Enable writes and erase sector
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));
        spi.write(SPI_ADDR_REG, 1, 0x000); // Sector 0
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_ERASE_SECTOR as u32));

        // Verify sector erased (all 0xFF)
        let store = spi.flash_store.lock().unwrap();
        for i in 0..4096 {
            assert_eq!(store[i], 0xFF);
        }
    }

    #[test]
    fn test_write_protection() {
        let mut spi = SpiFlash::new(1024, InterruptSource::Spi1);

        // Try to write without enabling
        spi.write(SPI_W0_REG, 1, 0x12345678);
        spi.write(SPI_ADDR_REG, 1, 0x100);
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_WRITE as u32));

        // Verify nothing written (still 0xFF)
        let store = spi.flash_store.lock().unwrap();
        assert_eq!(store[0x100], 0xFF);
    }

    #[test]
    fn test_register_access() {
        let mut spi = SpiFlash::new(1024, InterruptSource::Spi1);

        // Test address register
        spi.write(SPI_ADDR_REG, 1, 0xABCDEF);
        assert_eq!(spi.read(SPI_ADDR_REG, 1), 0xABCDEF);

        // Test control register
        spi.write(SPI_CTRL_REG, 1, 0x12345678);
        assert_eq!(spi.read(SPI_CTRL_REG, 1), 0x12345678);

        // Test data buffer registers
        for i in 0..16u32 {
            let reg_addr = SPI_W0_REG + (i * 4);
            let val = 0x10000000 + i;
            spi.write(reg_addr, 1, val);
            assert_eq!(spi.read(reg_addr, 1), val);
        }
    }

    #[test]
    fn test_read_status() {
        let mut spi = SpiFlash::new(1024, InterruptSource::Spi1);

        // Initially, write enable should be cleared
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_READ_STATUS as u32));
        let status = spi.read(SPI_W0_REG, 1);
        assert_eq!(status & 0x02, 0x00);

        // Enable writes
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));

        // Check status again
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_READ_STATUS as u32));
        let status = spi.read(SPI_W0_REG, 1);
        assert_eq!(status & 0x02, 0x02);
    }

    #[test]
    fn test_large_read() {
        let mut spi = SpiFlash::new(1024, InterruptSource::Spi1);

        // Write 64 bytes of test pattern
        {
            let mut store = spi.flash_store.lock().unwrap();
            for i in 0..64 {
                store[i] = i as u8;
            }
        }

        // Read 64 bytes
        spi.write(SPI_ADDR_REG, 1, 0x000);
        spi.write(SPI_CMD_REG, 1, 0x8000_0000 | (CMD_READ as u32));

        // Verify all 16 words (64 bytes)
        for word_idx in 0..16 {
            let word = spi.read(SPI_W0_REG + (word_idx * 4), 1);
            let expected = ((word_idx * 4 + 0) as u32)
                | (((word_idx * 4 + 1) as u32) << 8)
                | (((word_idx * 4 + 2) as u32) << 16)
                | (((word_idx * 4 + 3) as u32) << 24);
            assert_eq!(word, expected);
        }
    }
}
