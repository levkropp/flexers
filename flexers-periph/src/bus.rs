use flexers_core::memory::MmioHandler;
use std::collections::HashMap;

/// Address range for peripheral mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddrRange {
    pub start: u32,
    pub end: u32,
}

impl AddrRange {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.start && addr < self.end
    }
}

/// Peripheral bus routes MMIO accesses to registered handlers
pub struct PeripheralBus {
    handlers: HashMap<AddrRange, Box<dyn MmioHandler>>,
}

impl PeripheralBus {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a peripheral handler for a specific address range
    pub fn register(&mut self, range: AddrRange, handler: Box<dyn MmioHandler>) {
        self.handlers.insert(range, handler);
    }

    /// Dispatch a read operation to the appropriate handler
    pub fn dispatch_read(&self, addr: u32, size: u8) -> Option<u32> {
        for (range, handler) in &self.handlers {
            if range.contains(addr) {
                return Some(handler.read(addr, size));
            }
        }
        None // Address not mapped to any peripheral
    }

    /// Dispatch a write operation to the appropriate handler
    pub fn dispatch_write(&mut self, addr: u32, size: u8, val: u32) -> bool {
        for (range, handler) in &mut self.handlers {
            if range.contains(addr) {
                handler.write(addr, size, val);
                return true;
            }
        }
        false // Address not mapped
    }
}

impl Default for PeripheralBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyHandler {
        value: u32,
    }

    impl MmioHandler for DummyHandler {
        fn read(&self, _addr: u32, _size: u8) -> u32 {
            self.value
        }

        fn write(&mut self, _addr: u32, _size: u8, val: u32) {
            self.value = val;
        }
    }

    #[test]
    fn test_peripheral_bus_dispatch() {
        let mut bus = PeripheralBus::new();

        // Register a dummy handler for address range 0x1000-0x2000
        let range = AddrRange::new(0x1000, 0x2000);
        let handler = Box::new(DummyHandler { value: 0x42 });
        bus.register(range, handler);

        // Test read within range
        let val = bus.dispatch_read(0x1500, 4);
        assert_eq!(val, Some(0x42));

        // Test write within range
        assert!(bus.dispatch_write(0x1500, 4, 0x1234));

        // Verify write took effect
        let val = bus.dispatch_read(0x1500, 4);
        assert_eq!(val, Some(0x1234));

        // Test read outside range
        let val = bus.dispatch_read(0x3000, 4);
        assert_eq!(val, None);

        // Test write outside range
        assert!(!bus.dispatch_write(0x3000, 4, 0x5678));
    }

    #[test]
    fn test_addr_range() {
        let range = AddrRange::new(0x1000, 0x2000);

        assert!(range.contains(0x1000));
        assert!(range.contains(0x1500));
        assert!(range.contains(0x1FFF));
        assert!(!range.contains(0x0FFF));
        assert!(!range.contains(0x2000));
        assert!(!range.contains(0x2001));
    }
}
