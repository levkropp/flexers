/// FreeRTOS system tick timer integration
///
/// This module implements automatic scheduler preemption via timer interrupts.
/// Uses Timer0Group0 as the dedicated system tick timer.

use flexers_periph::timer::Timer;
use flexers_periph::interrupt::{InterruptSource, InterruptRaiser};
use super::scheduler::SCHEDULER;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

/// FreeRTOS system tick configuration
pub const DEFAULT_TICK_RATE_HZ: u32 = 100; // 100 Hz = 10ms tick
pub const CPU_CLOCK_HZ: u32 = 160_000_000; // ESP32 default 160 MHz

lazy_static! {
    /// Global system tick timer instance
    pub static ref SYSTEM_TICK_TIMER: Arc<Mutex<SystemTickTimer>> = {
        Arc::new(Mutex::new(SystemTickTimer::new(DEFAULT_TICK_RATE_HZ)))
    };
}

/// System tick timer (uses Timer0Group0)
pub struct SystemTickTimer {
    /// Timer peripheral
    timer: Arc<Mutex<Timer>>,

    /// Configured tick rate in Hz
    tick_rate_hz: u32,

    /// Interrupt count (for debugging)
    interrupt_count: u64,
}

impl SystemTickTimer {
    /// Create a new system tick timer
    pub fn new(tick_rate_hz: u32) -> Self {
        let timer = Arc::new(Mutex::new(Timer::new(InterruptSource::Timer0Group0)));

        Self {
            timer,
            tick_rate_hz,
            interrupt_count: 0,
        }
    }

    /// Initialize and start the timer with interrupt controller integration
    pub fn start(&mut self, interrupt_raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        let alarm_value = (CPU_CLOCK_HZ / self.tick_rate_hz) as u64;

        let mut timer = self.timer.lock().unwrap();
        timer.set_interrupt_raiser(interrupt_raiser);
        timer.set_alarm(alarm_value);
        timer.set_auto_reload(true);
        timer.set_load_value(0);
        timer.set_int_enabled(true);
        timer.set_enabled(true);
    }

    /// Stop the timer
    pub fn stop(&mut self) {
        let mut timer = self.timer.lock().unwrap();
        timer.set_enabled(false);
    }

    /// Tick the timer (advances counter)
    pub fn tick_timer(&mut self) {
        let mut timer = self.timer.lock().unwrap();
        timer.tick();
    }

    /// Get the timer peripheral for manual configuration
    pub fn get_timer(&self) -> Arc<Mutex<Timer>> {
        Arc::clone(&self.timer)
    }

    /// Interrupt handler - called when timer alarm triggers
    pub fn on_interrupt(&mut self, cpu: &mut flexers_core::cpu::XtensaCpu) {
        self.interrupt_count += 1;

        // Tick the scheduler
        {
            let mut scheduler = SCHEDULER.lock().unwrap();
            scheduler.tick();
        }

        // Check if context switch needed and perform it
        if Self::should_switch_context() {
            let mut scheduler = SCHEDULER.lock().unwrap();
            let _ = scheduler.switch_context(cpu);
        }
    }

    /// Check if context switch should occur (static method for use by interrupt handler)
    fn should_switch_context() -> bool {
        let scheduler = SCHEDULER.lock().unwrap();

        if !scheduler.is_running() {
            return false;
        }

        // Find highest priority ready task
        if let Some(next_task_handle) = scheduler.peek_next_task() {
            // Switch if higher priority task is ready
            if let Some(current) = scheduler.get_current_task() {
                if let Some(next_priority) = scheduler.get_task_priority(next_task_handle) {
                    if let Some(current_priority) = scheduler.get_task_priority(current) {
                        return next_priority > current_priority;
                    }
                }
            }
            // Switch if no current task
            return true;
        }
        false
    }

    /// Get interrupt count
    pub fn get_interrupt_count(&self) -> u64 {
        self.interrupt_count
    }

    /// Get tick rate
    pub fn get_tick_rate_hz(&self) -> u32 {
        self.tick_rate_hz
    }

    /// Set tick rate (requires restart to take effect)
    pub fn set_tick_rate_hz(&mut self, tick_rate_hz: u32) {
        self.tick_rate_hz = tick_rate_hz;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let timer = SystemTickTimer::new(100);
        assert_eq!(timer.get_tick_rate_hz(), 100);
        assert_eq!(timer.get_interrupt_count(), 0);
    }

    #[test]
    fn test_tick_rate_calculation() {
        let timer = SystemTickTimer::new(100);
        let alarm_value = CPU_CLOCK_HZ / timer.get_tick_rate_hz();
        assert_eq!(alarm_value, 1_600_000); // 160MHz / 100Hz = 1.6M cycles
    }

    #[test]
    fn test_different_tick_rates() {
        let timer_100hz = SystemTickTimer::new(100);
        let timer_1000hz = SystemTickTimer::new(1000);

        assert_eq!(timer_100hz.get_tick_rate_hz(), 100);
        assert_eq!(timer_1000hz.get_tick_rate_hz(), 1000);
    }
}
