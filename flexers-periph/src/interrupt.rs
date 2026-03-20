use std::sync::{Arc, Mutex};

/// ESP32 interrupt sources (simplified - not all 64 sources)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InterruptSource {
    Uart0 = 0,
    Uart1 = 1,
    Uart2 = 2,
    Timer0Group0 = 3,
    Timer1Group0 = 4,
    Timer0Group1 = 5,
    Timer1Group1 = 6,
    GpioNmi = 7,
    Gpio = 8,
    // Add more as needed
}

/// Interrupt priority levels (0 = no interrupt, 1-5 = increasing priority)
pub type InterruptLevel = u8;

/// Interrupt controller manages interrupt sources and priorities
pub struct InterruptController {
    /// Pending interrupts (bitfield, 1 = pending)
    pending: u64,

    /// Enabled interrupts (bitfield, 1 = enabled)
    enabled: u64,

    /// Priority level for each source (0-5)
    priorities: [InterruptLevel; 64],

    /// Currently servicing interrupt level (0 = none)
    current_level: InterruptLevel,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            pending: 0,
            enabled: 0,
            priorities: [1; 64], // Default priority 1 for all
            current_level: 0,
        }
    }

    /// Raise an interrupt (called by peripherals)
    pub fn raise(&mut self, source: InterruptSource) {
        self.pending |= 1u64 << (source as u32);
    }

    /// Clear an interrupt (called by exception handler)
    pub fn clear(&mut self, source: InterruptSource) {
        self.pending &= !(1u64 << (source as u32));
    }

    /// Update enabled mask (from CPU INTENABLE register)
    pub fn set_enabled(&mut self, mask: u64) {
        self.enabled = mask;
    }

    /// Get enabled mask
    pub fn get_enabled(&self) -> u64 {
        self.enabled
    }

    /// Get pending interrupts
    pub fn get_pending(&self) -> u64 {
        self.pending
    }

    /// Set priority for an interrupt source
    pub fn set_priority(&mut self, source: InterruptSource, priority: InterruptLevel) {
        self.priorities[source as usize] = priority;
    }

    /// Get highest-priority pending interrupt
    pub fn get_pending_interrupt(&self) -> Option<(InterruptSource, InterruptLevel)> {
        let active = self.pending & self.enabled;
        if active == 0 {
            return None;
        }

        // Find highest priority
        let mut highest_level = 0;
        let mut highest_source = None;

        for i in 0..64 {
            if (active & (1u64 << i)) != 0 {
                let level = self.priorities[i];
                if level > highest_level {
                    highest_level = level;
                    highest_source = Some(i);
                }
            }
        }

        if let Some(source) = highest_source {
            // Return if priority higher than current level
            if highest_level > self.current_level {
                return Some((
                    unsafe { std::mem::transmute(source as u8) },
                    highest_level
                ));
            }
        }

        None
    }

    /// Set current interrupt level (when entering/exiting handler)
    pub fn set_current_level(&mut self, level: InterruptLevel) {
        self.current_level = level;
    }

    /// Get current interrupt level
    pub fn get_current_level(&self) -> InterruptLevel {
        self.current_level
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

impl flexers_core::memory::MmioHandler for InterruptController {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFF {
            0x00 => (self.pending & 0xFFFFFFFF) as u32,  // Pending low
            0x04 => (self.pending >> 32) as u32,         // Pending high
            0x08 => (self.enabled & 0xFFFFFFFF) as u32,  // Enabled low
            0x0C => (self.enabled >> 32) as u32,         // Enabled high
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFF {
            0x08 => self.enabled = (self.enabled & 0xFFFFFFFF00000000) | val as u64,
            0x0C => self.enabled = (self.enabled & 0x00000000FFFFFFFF) | ((val as u64) << 32),
            _ => {} // Other registers read-only
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_raise_clear() {
        let mut ic = InterruptController::new();

        // Raise interrupt
        ic.raise(InterruptSource::Uart0);
        assert_eq!(ic.get_pending() & 0x1, 0x1);

        // Clear interrupt
        ic.clear(InterruptSource::Uart0);
        assert_eq!(ic.get_pending() & 0x1, 0x0);
    }

    #[test]
    fn test_interrupt_priority() {
        let mut ic = InterruptController::new();

        // Set priorities
        ic.set_priority(InterruptSource::Uart0, 2);
        ic.set_priority(InterruptSource::Timer0Group0, 3);

        // Enable all interrupts
        ic.set_enabled(0xFFFFFFFFFFFFFFFF);

        // Raise both
        ic.raise(InterruptSource::Uart0);
        ic.raise(InterruptSource::Timer0Group0);

        // Should return highest priority (Timer)
        let result = ic.get_pending_interrupt();
        assert!(result.is_some());
        let (source, level) = result.unwrap();
        assert_eq!(source as u8, InterruptSource::Timer0Group0 as u8);
        assert_eq!(level, 3);
    }

    #[test]
    fn test_interrupt_masking() {
        let mut ic = InterruptController::new();

        // Raise interrupt but don't enable it
        ic.raise(InterruptSource::Uart0);
        ic.set_enabled(0);

        // Should not be pending
        assert!(ic.get_pending_interrupt().is_none());

        // Enable it
        ic.set_enabled(1 << (InterruptSource::Uart0 as u8));

        // Should now be pending
        assert!(ic.get_pending_interrupt().is_some());
    }

    #[test]
    fn test_current_level_masking() {
        let mut ic = InterruptController::new();

        // Set up interrupt with priority 2
        ic.set_priority(InterruptSource::Uart0, 2);
        ic.set_enabled(0xFFFFFFFFFFFFFFFF);
        ic.raise(InterruptSource::Uart0);

        // Should be pending when current level is 0
        ic.set_current_level(0);
        assert!(ic.get_pending_interrupt().is_some());

        // Should not be pending when current level is >= priority
        ic.set_current_level(2);
        assert!(ic.get_pending_interrupt().is_none());

        ic.set_current_level(3);
        assert!(ic.get_pending_interrupt().is_none());

        // Should be pending again when current level drops
        ic.set_current_level(1);
        assert!(ic.get_pending_interrupt().is_some());
    }
}
