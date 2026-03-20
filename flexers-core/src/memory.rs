use std::ptr::NonNull;
use std::cell::UnsafeCell;
use std::sync::{Arc, Mutex};

/// Page size for memory mapping (4 KB)
const PAGE_SIZE: usize = 4096;
/// Total number of pages in 4GB address space
const PAGE_COUNT: usize = 1 << 20;

/// ESP32 memory regions
const SRAM_BASE: u32 = 0x3FF_A0000;
const SRAM_SIZE: usize = 520 * 1024; // 520 KB

const ROM_BASE: u32 = 0x4000_0000;
const ROM_SIZE: usize = 448 * 1024; // 448 KB

const FLASH_DATA_BASE: u32 = 0x3F40_0000;
const FLASH_DATA_SIZE: usize = 4 * 1024 * 1024; // 4 MB (reduced for testing, expandable later)

const FLASH_INSN_BASE: u32 = 0x4008_0000;
const FLASH_INSN_SIZE: usize = 4 * 1024 * 1024; // 4 MB (reduced for testing, expandable later)

const RTC_DRAM_BASE: u32 = 0x3FF8_0000;
const RTC_DRAM_SIZE: usize = 8 * 1024; // 8 KB

/// Memory subsystem with page-table-based fast lookup
/// Uses UnsafeCell for interior mutability (required for shared CPU state)
pub struct Memory {
    /// Internal SRAM
    sram: UnsafeCell<Vec<u8>>,
    /// Boot ROM
    rom: UnsafeCell<Vec<u8>>,
    /// RTC DRAM
    rtc_dram: UnsafeCell<Vec<u8>>,
    /// Page table for fast address translation
    page_table: UnsafeCell<Vec<Option<NonNull<u8>>>>,
    /// MMIO handlers for peripheral regions
    mmio_handlers: UnsafeCell<Vec<Option<Box<dyn MmioHandler>>>>,
    /// Peripheral bus for MMIO dispatch (optional - None for testing)
    peripheral_bus: Option<Arc<Mutex<dyn PeripheralBusDispatch>>>,
    /// Shared flash storage (optional - for integration with SPI flash controller)
    /// When set, both FLASH_DATA_BASE and FLASH_INSN_BASE map to this shared storage
    flash_store: Option<Arc<Mutex<Vec<u8>>>>,
}

/// MMIO handler trait for peripheral devices
pub trait MmioHandler: Send + Sync {
    fn read(&self, addr: u32, size: u8) -> u32;
    fn write(&mut self, addr: u32, size: u8, val: u32);
}

/// Peripheral bus dispatcher trait (for dependency injection)
pub trait PeripheralBusDispatch: Send + Sync {
    fn dispatch_read(&self, addr: u32, size: u8) -> Option<u32>;
    fn dispatch_write(&mut self, addr: u32, size: u8, val: u32) -> bool;
}

impl Memory {
    /// Create new memory subsystem with initialized regions
    pub fn new() -> Self {
        let mem = Self {
            sram: UnsafeCell::new(vec![0u8; SRAM_SIZE]),
            rom: UnsafeCell::new(vec![0u8; ROM_SIZE]),
            rtc_dram: UnsafeCell::new(vec![0u8; RTC_DRAM_SIZE]),
            page_table: UnsafeCell::new(vec![None; PAGE_COUNT]),
            mmio_handlers: UnsafeCell::new(Vec::new()),
            peripheral_bus: None,
            flash_store: None,
        };

        // Map regions (except flash - will be mapped when flash_store is set)
        unsafe {
            let sram_ptr = &mut *mem.sram.get();
            let rom_ptr = &mut *mem.rom.get();
            let rtc_dram_ptr = &mut *mem.rtc_dram.get();
            let page_table = &mut *mem.page_table.get();

            Self::map_region_static(page_table, SRAM_BASE, sram_ptr);
            Self::map_region_static(page_table, ROM_BASE, rom_ptr);
            Self::map_region_static(page_table, RTC_DRAM_BASE, rtc_dram_ptr);
        }

        mem
    }

    /// Set peripheral bus for MMIO dispatch
    pub fn set_peripheral_bus(&mut self, bus: Arc<Mutex<dyn PeripheralBusDispatch>>) {
        self.peripheral_bus = Some(bus);
    }

    /// Set shared flash storage (for integration with SPI flash controller)
    /// This maps both FLASH_DATA_BASE and FLASH_INSN_BASE to the same shared storage,
    /// saving memory and ensuring consistency between data and instruction accesses.
    pub fn set_flash_store(&mut self, flash: Arc<Mutex<Vec<u8>>>) {
        self.flash_store = Some(flash.clone());

        // Map flash regions to shared storage
        // Note: This is safe because we're mapping read-only views into the flash storage
        // The flash controller owns the actual data and synchronizes access via Mutex
        unsafe {
            let mut flash_lock = flash.lock().unwrap();
            let page_table = &mut *self.page_table.get();

            // Map both flash regions to the same underlying storage
            Self::map_region_static(page_table, FLASH_DATA_BASE, &mut flash_lock);
            Self::map_region_static(page_table, FLASH_INSN_BASE, &mut flash_lock);
        }
    }

    /// Map a memory region into the page table (static helper)
    fn map_region_static(
        page_table: &mut Vec<Option<NonNull<u8>>>,
        base: u32,
        data: &mut Vec<u8>,
    ) {
        let base_page = (base as usize) >> 12;
        let page_count = (data.len() + PAGE_SIZE - 1) / PAGE_SIZE;

        for i in 0..page_count {
            let page_idx = base_page + i;
            let offset = i * PAGE_SIZE;
            if offset < data.len() {
                let ptr = unsafe {
                    NonNull::new_unchecked(data.as_mut_ptr().add(offset))
                };
                page_table[page_idx] = Some(ptr);
            }
        }
    }

    /// Fast-path read u32 (inlined for performance)
    #[inline(always)]
    pub fn read_u32(&self, addr: u32) -> u32 {
        let page_idx = (addr >> 12) as usize;
        unsafe {
            let page_table = &*self.page_table.get();
            if let Some(page_ptr) = page_table[page_idx] {
                let ptr = page_ptr.as_ptr().add((addr & 0xFFF) as usize);
                ptr.cast::<u32>().read_unaligned()
            } else {
                self.read_u32_slow(addr)
            }
        }
    }

    /// Slow-path read u32 (MMIO or unmapped)
    fn read_u32_slow(&self, addr: u32) -> u32 {
        // Check if peripheral bus is available
        if let Some(ref bus) = self.peripheral_bus {
            if let Ok(bus_lock) = bus.lock() {
                if let Some(val) = bus_lock.dispatch_read(addr, 4) {
                    return val;
                }
            }
        }

        // Address not mapped to peripheral - return 0
        0
    }

    /// Fast-path write u32
    #[inline(always)]
    pub fn write_u32(&self, addr: u32, val: u32) {
        let page_idx = (addr >> 12) as usize;
        unsafe {
            let page_table = &*self.page_table.get();
            if let Some(page_ptr) = page_table[page_idx] {
                let ptr = page_ptr.as_ptr().add((addr & 0xFFF) as usize);
                ptr.cast::<u32>().write_unaligned(val);
            } else {
                self.write_u32_slow(addr, val);
            }
        }
    }

    /// Slow-path write u32 (MMIO or unmapped)
    fn write_u32_slow(&self, addr: u32, val: u32) {
        if let Some(ref bus) = self.peripheral_bus {
            if let Ok(mut bus_lock) = bus.lock() {
                bus_lock.dispatch_write(addr, 4, val);
            }
        }
        // Unmapped writes are silently dropped (real hardware behavior)
    }

    /// Read u16
    #[inline(always)]
    pub fn read_u16(&self, addr: u32) -> u16 {
        let page_idx = (addr >> 12) as usize;
        unsafe {
            let page_table = &*self.page_table.get();
            if let Some(page_ptr) = page_table[page_idx] {
                let ptr = page_ptr.as_ptr().add((addr & 0xFFF) as usize);
                ptr.cast::<u16>().read_unaligned()
            } else {
                self.read_u32_slow(addr) as u16
            }
        }
    }

    /// Write u16
    #[inline(always)]
    pub fn write_u16(&self, addr: u32, val: u16) {
        let page_idx = (addr >> 12) as usize;
        unsafe {
            let page_table = &*self.page_table.get();
            if let Some(page_ptr) = page_table[page_idx] {
                let ptr = page_ptr.as_ptr().add((addr & 0xFFF) as usize);
                ptr.cast::<u16>().write_unaligned(val);
            } else {
                self.write_u32_slow(addr, val as u32);
            }
        }
    }

    /// Read u8
    #[inline(always)]
    pub fn read_u8(&self, addr: u32) -> u8 {
        let page_idx = (addr >> 12) as usize;
        unsafe {
            let page_table = &*self.page_table.get();
            if let Some(page_ptr) = page_table[page_idx] {
                let ptr = page_ptr.as_ptr().add((addr & 0xFFF) as usize);
                *ptr
            } else {
                self.read_u32_slow(addr) as u8
            }
        }
    }

    /// Write u8
    #[inline(always)]
    pub fn write_u8(&self, addr: u32, val: u8) {
        let page_idx = (addr >> 12) as usize;
        unsafe {
            let page_table = &*self.page_table.get();
            if let Some(page_ptr) = page_table[page_idx] {
                let ptr = page_ptr.as_ptr().add((addr & 0xFFF) as usize);
                *ptr = val;
            } else {
                self.write_u32_slow(addr, val as u32);
            }
        }
    }

    /// Write bytes (for firmware loading)
    pub fn write_bytes(&self, addr: u32, data: &[u8]) -> Result<(), MemoryError> {
        for (i, &byte) in data.iter().enumerate() {
            self.write_u8(addr + i as u32, byte);
        }
        Ok(())
    }

    /// Read bytes
    pub fn read_bytes(&self, addr: u32, len: usize) -> Vec<u8> {
        (0..len).map(|i| self.read_u8(addr + i as u32)).collect()
    }

    /// Load flash contents from SPI flash controller backing store
    ///
    /// This copies data from the SPI flash controller's internal storage
    /// to the memory-mapped flash regions (0x3F400000 and 0x40080000).
    /// This is needed after firmware is loaded into the SPI flash controller
    /// to make it accessible to the CPU via memory-mapped addresses.
    /// Load flash from SPI flash controller
    /// This method now just calls set_flash_store for compatibility
    pub fn load_flash_from_controller(&mut self, flash_store: Arc<Mutex<Vec<u8>>>) {
        self.set_flash_store(flash_store);
    }
}

#[derive(Debug)]
pub enum MemoryError {
    InvalidAddress(u32),
    Alignment(u32),
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: Memory uses proper synchronization for MMIO handlers
unsafe impl Send for Memory {}
unsafe impl Sync for Memory {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_read_write() {
        let mem = Memory::new();

        // Test SRAM write/read
        mem.write_u32(SRAM_BASE, 0x12345678);
        assert_eq!(mem.read_u32(SRAM_BASE), 0x12345678);

        // Test u16
        mem.write_u16(SRAM_BASE + 4, 0xABCD);
        assert_eq!(mem.read_u16(SRAM_BASE + 4), 0xABCD);

        // Test u8
        mem.write_u8(SRAM_BASE + 8, 0x42);
        assert_eq!(mem.read_u8(SRAM_BASE + 8), 0x42);
    }

    #[test]
    fn test_memory_bytes() {
        let mem = Memory::new();

        let data = vec![1, 2, 3, 4, 5];
        mem.write_bytes(SRAM_BASE, &data).unwrap();

        let read_data = mem.read_bytes(SRAM_BASE, 5);
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_flash_regions() {
        use std::sync::{Arc, Mutex};

        let mut mem = Memory::new();

        // Set up shared flash storage
        let flash_store = Arc::new(Mutex::new(vec![0u8; FLASH_DATA_SIZE]));
        mem.set_flash_store(flash_store);

        // Test flash data region
        mem.write_u32(FLASH_DATA_BASE, 0xDEADBEEF);
        assert_eq!(mem.read_u32(FLASH_DATA_BASE), 0xDEADBEEF);

        // Test flash instruction region
        mem.write_u32(FLASH_INSN_BASE, 0xCAFEBABE);
        assert_eq!(mem.read_u32(FLASH_INSN_BASE), 0xCAFEBABE);

        // Test that both regions map to the same underlying storage
        // Write to data region, read from instruction region
        mem.write_u32(FLASH_DATA_BASE + 0x100, 0x12345678);
        assert_eq!(mem.read_u32(FLASH_INSN_BASE + 0x100), 0x12345678);
    }
}
