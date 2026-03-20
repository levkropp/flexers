use flexers_core::memory::MmioHandler;
use crate::interrupt::{InterruptSource, InterruptRaiser};
use std::sync::{Arc, Mutex};

/// Timer register offsets
const TIMER_COUNTER_LOW: u32 = 0x00;
const TIMER_COUNTER_HIGH: u32 = 0x04;
const TIMER_ALARM_LOW: u32 = 0x08;
const TIMER_ALARM_HIGH: u32 = 0x0C;
const TIMER_CONFIG: u32 = 0x10;
const TIMER_LOAD_LOW: u32 = 0x14;
const TIMER_LOAD_HIGH: u32 = 0x18;

/// Timer configuration bits
const TIMER_ENABLE: u32 = 1 << 0;
const TIMER_AUTO_RELOAD: u32 = 1 << 1;
const TIMER_INT_ENABLE: u32 = 1 << 2;

/// Timer peripheral
pub struct Timer {
    /// Current counter value
    counter: u64,

    /// Alarm/compare value
    alarm: u64,

    /// Auto-reload value (for periodic mode)
    load_value: u64,

    /// Timer enabled
    enabled: bool,

    /// Auto-reload enabled
    auto_reload: bool,

    /// Interrupt on alarm match
    int_enabled: bool,

    /// Interrupt raiser
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,

    /// Interrupt source
    int_source: InterruptSource,
}

impl Timer {
    pub fn new(int_source: InterruptSource) -> Self {
        Self {
            counter: 0,
            alarm: 0,
            load_value: 0,
            enabled: false,
            auto_reload: false,
            int_enabled: false,
            int_raiser: None,
            int_source,
        }
    }

    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Tick timer (called every CPU cycle)
    pub fn tick(&mut self) {
        if !self.enabled {
            return;
        }

        self.counter += 1;

        // Check for alarm match
        if self.counter >= self.alarm && self.alarm > 0 {
            if self.int_enabled {
                if let Some(ref raiser) = self.int_raiser {
                    if let Ok(mut raiser_lock) = raiser.lock() {
                        raiser_lock.raise(self.int_source);
                    }
                }
            }

            // Auto-reload if enabled
            if self.auto_reload {
                self.counter = self.load_value;
            } else {
                self.enabled = false; // One-shot mode
            }
        }
    }
}

impl MmioHandler for Timer {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFF {
            0x00 => (self.counter & 0xFFFFFFFF) as u32,      // Counter low
            0x04 => ((self.counter >> 32) & 0xFFFFFFFF) as u32, // Counter high
            0x08 => (self.alarm & 0xFFFFFFFF) as u32,         // Alarm low
            0x0C => ((self.alarm >> 32) & 0xFFFFFFFF) as u32, // Alarm high
            0x10 => {
                let mut val = 0u32;
                if self.enabled { val |= TIMER_ENABLE; }
                if self.auto_reload { val |= TIMER_AUTO_RELOAD; }
                if self.int_enabled { val |= TIMER_INT_ENABLE; }
                val
            }
            0x14 => (self.load_value & 0xFFFFFFFF) as u32,    // Load low
            0x18 => ((self.load_value >> 32) & 0xFFFFFFFF) as u32, // Load high
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFF {
            0x00 => self.counter = (self.counter & 0xFFFFFFFF00000000) | val as u64,
            0x04 => self.counter = (self.counter & 0x00000000FFFFFFFF) | ((val as u64) << 32),
            0x08 => self.alarm = (self.alarm & 0xFFFFFFFF00000000) | val as u64,
            0x0C => self.alarm = (self.alarm & 0x00000000FFFFFFFF) | ((val as u64) << 32),
            0x10 => {
                self.enabled = (val & TIMER_ENABLE) != 0;
                self.auto_reload = (val & TIMER_AUTO_RELOAD) != 0;
                self.int_enabled = (val & TIMER_INT_ENABLE) != 0;
            }
            0x14 => self.load_value = (self.load_value & 0xFFFFFFFF00000000) | val as u64,
            0x18 => self.load_value = (self.load_value & 0x00000000FFFFFFFF) | ((val as u64) << 32),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyInterruptRaiser {
        raised: Vec<InterruptSource>,
    }

    impl InterruptRaiser for DummyInterruptRaiser {
        fn raise(&mut self, source: InterruptSource) {
            self.raised.push(source);
        }
    }

    #[test]
    fn test_timer_basic() {
        let mut timer = Timer::new(InterruptSource::Timer0Group0);

        // Verify initial state
        assert!(!timer.enabled);
        assert_eq!(timer.counter, 0);

        // Enable timer
        timer.write(0x10, 4, TIMER_ENABLE);
        assert!(timer.enabled);

        // Tick timer
        timer.tick();
        assert_eq!(timer.counter, 1);

        timer.tick();
        assert_eq!(timer.counter, 2);
    }

    #[test]
    fn test_timer_alarm_oneshot() {
        let mut timer = Timer::new(InterruptSource::Timer0Group0);
        let raiser = Arc::new(Mutex::new(DummyInterruptRaiser { raised: Vec::new() }));
        timer.set_interrupt_raiser(raiser.clone());

        // Set alarm at 100 cycles
        timer.write(0x08, 4, 100); // Alarm low
        timer.write(0x10, 4, TIMER_ENABLE | TIMER_INT_ENABLE);

        // Tick 99 times - no interrupt yet
        for _ in 0..99 {
            timer.tick();
        }

        let raiser_lock = raiser.lock().unwrap();
        assert!(raiser_lock.raised.is_empty());
        drop(raiser_lock);

        // Tick once more - should trigger alarm
        timer.tick();

        let raiser_lock = raiser.lock().unwrap();
        assert_eq!(raiser_lock.raised.len(), 1);
        assert_eq!(raiser_lock.raised[0] as u8, InterruptSource::Timer0Group0 as u8);
        drop(raiser_lock);

        // Timer should be disabled (one-shot mode)
        let config = timer.read(0x10, 4);
        assert_eq!(config & TIMER_ENABLE, 0);
    }

    #[test]
    fn test_timer_alarm_autoreload() {
        let mut timer = Timer::new(InterruptSource::Timer0Group0);
        let raiser = Arc::new(Mutex::new(DummyInterruptRaiser { raised: Vec::new() }));
        timer.set_interrupt_raiser(raiser.clone());

        // Set alarm at 10 cycles, auto-reload enabled
        timer.write(0x08, 4, 10); // Alarm low
        timer.write(0x14, 4, 0);  // Load low (reload to 0)
        timer.write(0x10, 4, TIMER_ENABLE | TIMER_AUTO_RELOAD | TIMER_INT_ENABLE);

        // Tick 10 times - should trigger alarm
        for _ in 0..10 {
            timer.tick();
        }

        // Should have triggered interrupt
        let raiser_lock = raiser.lock().unwrap();
        assert_eq!(raiser_lock.raised.len(), 1);
        drop(raiser_lock);

        // Timer should still be enabled and counter reset
        assert!(timer.enabled);
        assert_eq!(timer.counter, 0);

        // Tick 10 more times - should trigger again
        for _ in 0..10 {
            timer.tick();
        }

        let raiser_lock = raiser.lock().unwrap();
        assert_eq!(raiser_lock.raised.len(), 2);
    }

    #[test]
    fn test_timer_64bit_counter() {
        let mut timer = Timer::new(InterruptSource::Timer0Group0);

        // Write 64-bit value to counter
        timer.write(0x00, 4, 0x12345678); // Low
        timer.write(0x04, 4, 0xABCDEF00); // High

        assert_eq!(timer.counter, 0xABCDEF0012345678);

        // Read back
        assert_eq!(timer.read(0x00, 4), 0x12345678);
        assert_eq!(timer.read(0x04, 4), 0xABCDEF00);
    }
}
