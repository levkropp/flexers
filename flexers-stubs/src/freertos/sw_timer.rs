/// FreeRTOS software timer implementation
///
/// Software timers provide callback-based periodic or one-shot timing without dedicated tasks.

use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use flexers_core::cpu::XtensaCpu;

pub type TimerHandle = usize;
pub const MAX_SW_TIMERS: usize = 128;

/// Software timer
pub struct SoftwareTimer {
    /// Timer handle (for debugging)
    handle: TimerHandle,

    /// Timer name
    name: String,

    /// Period in ticks
    period_ticks: u32,

    /// Auto-reload (periodic) vs one-shot
    auto_reload: bool,

    /// Callback function pointer
    callback: u32,

    /// Timer ID (passed to callback as argument)
    timer_id: u32,

    /// Remaining ticks until fire
    remaining_ticks: u32,

    /// Timer active flag
    active: bool,
}

impl SoftwareTimer {
    pub fn new(
        handle: TimerHandle,
        name: String,
        period_ticks: u32,
        auto_reload: bool,
        callback: u32,
        timer_id: u32,
    ) -> Self {
        Self {
            handle,
            name,
            period_ticks,
            auto_reload,
            callback,
            timer_id,
            remaining_ticks: period_ticks,
            active: false,
        }
    }

    /// Tick the timer (called by scheduler)
    pub fn tick(&mut self) -> Option<(u32, u32)> {
        if !self.active || self.remaining_ticks == 0 {
            return None;
        }

        self.remaining_ticks -= 1;

        if self.remaining_ticks == 0 {
            // Timer expired
            let callback_info = Some((self.callback, self.timer_id));

            if self.auto_reload {
                self.remaining_ticks = self.period_ticks;
            } else {
                self.active = false;
            }

            callback_info
        } else {
            None
        }
    }

    /// Start the timer
    pub fn start(&mut self, period_ticks: u32) {
        self.period_ticks = period_ticks;
        self.remaining_ticks = period_ticks;
        self.active = true;
    }

    /// Stop the timer
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Reset the timer (restart with current period)
    pub fn reset(&mut self) {
        self.remaining_ticks = self.period_ticks;
        self.active = true;
    }

    /// Change the period
    pub fn change_period(&mut self, new_period_ticks: u32) {
        self.period_ticks = new_period_ticks;
        self.remaining_ticks = new_period_ticks;
        self.active = true;
    }

    /// Check if timer is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get timer name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get timer period in ticks
    pub fn period_ticks(&self) -> u32 {
        self.period_ticks
    }
}

/// Software timer manager
pub struct SoftwareTimerManager {
    timers: Vec<Option<SoftwareTimer>>,
}

impl SoftwareTimerManager {
    pub fn new() -> Self {
        let mut timers = Vec::with_capacity(MAX_SW_TIMERS);
        for _ in 0..MAX_SW_TIMERS {
            timers.push(None);
        }
        Self { timers }
    }

    /// Create a new software timer
    pub fn create(
        &mut self,
        name: &str,
        period_ticks: u32,
        auto_reload: bool,
        callback: u32,
        timer_id: u32,
    ) -> Result<TimerHandle, String> {
        let handle = self.timers.iter().position(|t| t.is_none())
            .ok_or("Maximum software timers reached")?;

        self.timers[handle] = Some(SoftwareTimer::new(
            handle,
            name.to_string(),
            period_ticks,
            auto_reload,
            callback,
            timer_id,
        ));
        Ok(handle)
    }

    /// Delete a timer
    pub fn delete(&mut self, handle: TimerHandle) -> Result<(), String> {
        if handle >= MAX_SW_TIMERS {
            return Err("Invalid timer handle".to_string());
        }
        self.timers[handle] = None;
        Ok(())
    }

    /// Get mutable reference to timer
    pub fn get_mut(&mut self, handle: TimerHandle) -> Result<&mut SoftwareTimer, String> {
        self.timers.get_mut(handle)
            .and_then(|t| t.as_mut())
            .ok_or_else(|| "Invalid timer handle".to_string())
    }

    /// Get immutable reference to timer
    pub fn get(&self, handle: TimerHandle) -> Result<&SoftwareTimer, String> {
        self.timers.get(handle)
            .and_then(|t| t.as_ref())
            .ok_or_else(|| "Invalid timer handle".to_string())
    }

    /// Tick all timers and collect callbacks to execute
    pub fn tick(&mut self, callbacks: &mut Vec<(u32, u32)>) {
        for timer_opt in self.timers.iter_mut() {
            if let Some(timer) = timer_opt.as_mut() {
                if let Some(callback_info) = timer.tick() {
                    callbacks.push(callback_info);
                }
            }
        }
    }
}

lazy_static! {
    /// Global software timer manager instance
    pub static ref SW_TIMER_MANAGER: Arc<Mutex<SoftwareTimerManager>> =
        Arc::new(Mutex::new(SoftwareTimerManager::new()));
}

/// Execute timer callback
///
/// This is called by the scheduler's tick handler for each expired timer.
/// In a real FreeRTOS implementation, this would be done by the timer daemon task.
///
/// Note: This is a simplified implementation that just sets up the callback parameters.
/// A full implementation would execute the callback function and manage the call stack.
pub fn execute_timer_callback(cpu: &mut XtensaCpu, _callback_addr: u32, timer_id: u32) {
    // Set up callback parameters
    cpu.set_ar(2, timer_id);  // Pass timer handle as argument

    // In a full implementation, we would:
    // 1. Save current PC and registers
    // 2. Set PC to callback_addr
    // 3. Execute the function
    // 4. Restore state
    //
    // For now, this is a placeholder that shows the callback would be invoked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("test_timer", 100, true, 0x40000000, 42).unwrap();
        assert_eq!(handle, 0);

        let timer = manager.get(handle).unwrap();
        assert_eq!(timer.name(), "test_timer");
        assert!(!timer.is_active());
    }

    #[test]
    fn test_oneshot_timer() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("oneshot", 10, false, 0x40000000, 1).unwrap();

        // Start timer
        manager.get_mut(handle).unwrap().start(10);
        assert!(manager.get(handle).unwrap().is_active());

        // Tick 9 times - no callback yet
        let mut callbacks = Vec::new();
        for _ in 0..9 {
            let timer = manager.get_mut(handle).unwrap();
            if let Some(cb) = timer.tick() {
                callbacks.push(cb);
            }
        }
        assert_eq!(callbacks.len(), 0);
        assert!(manager.get(handle).unwrap().is_active());

        // Tick once more - should fire
        let timer = manager.get_mut(handle).unwrap();
        if let Some(cb) = timer.tick() {
            callbacks.push(cb);
        }
        assert_eq!(callbacks.len(), 1);
        assert_eq!(callbacks[0], (0x40000000, 1));

        // Timer should be inactive (one-shot)
        assert!(!manager.get(handle).unwrap().is_active());
    }

    #[test]
    fn test_periodic_timer() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("periodic", 5, true, 0x40000000, 2).unwrap();

        // Start timer
        manager.get_mut(handle).unwrap().start(5);

        let mut callbacks = Vec::new();

        // Tick 15 times - should fire 3 times (at tick 5, 10, 15)
        for _ in 0..15 {
            let timer = manager.get_mut(handle).unwrap();
            if let Some(cb) = timer.tick() {
                callbacks.push(cb);
            }
        }

        assert_eq!(callbacks.len(), 3);
        assert!(manager.get(handle).unwrap().is_active());  // Still active (periodic)
    }

    #[test]
    fn test_timer_stop() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("stoppable", 10, true, 0x40000000, 3).unwrap();

        manager.get_mut(handle).unwrap().start(10);
        assert!(manager.get(handle).unwrap().is_active());

        // Stop timer
        manager.get_mut(handle).unwrap().stop();
        assert!(!manager.get(handle).unwrap().is_active());

        // Tick - should not fire
        let mut callbacks = Vec::new();
        for _ in 0..20 {
            let timer = manager.get_mut(handle).unwrap();
            if let Some(cb) = timer.tick() {
                callbacks.push(cb);
            }
        }
        assert_eq!(callbacks.len(), 0);
    }

    #[test]
    fn test_timer_reset() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("resetable", 10, false, 0x40000000, 4).unwrap();

        manager.get_mut(handle).unwrap().start(10);

        // Tick 5 times
        for _ in 0..5 {
            manager.get_mut(handle).unwrap().tick();
        }

        // Reset timer
        manager.get_mut(handle).unwrap().reset();

        // Should take 10 more ticks to fire (not 5)
        let mut callbacks = Vec::new();
        for _ in 0..5 {
            let timer = manager.get_mut(handle).unwrap();
            if let Some(cb) = timer.tick() {
                callbacks.push(cb);
            }
        }
        assert_eq!(callbacks.len(), 0);  // Not fired yet

        // Tick 5 more times
        for _ in 0..5 {
            let timer = manager.get_mut(handle).unwrap();
            if let Some(cb) = timer.tick() {
                callbacks.push(cb);
            }
        }
        assert_eq!(callbacks.len(), 1);  // Now fired
    }

    #[test]
    fn test_change_period() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("changeable", 10, true, 0x40000000, 5).unwrap();

        manager.get_mut(handle).unwrap().start(10);

        // Change period to 5
        manager.get_mut(handle).unwrap().change_period(5);

        // Should fire after 5 ticks (not 10)
        let mut callbacks = Vec::new();
        for _ in 0..5 {
            let timer = manager.get_mut(handle).unwrap();
            if let Some(cb) = timer.tick() {
                callbacks.push(cb);
            }
        }
        assert_eq!(callbacks.len(), 1);
    }

    #[test]
    fn test_multiple_timers() {
        let mut manager = SoftwareTimerManager::new();
        let h1 = manager.create("timer1", 5, true, 0x40000000, 1).unwrap();
        let h2 = manager.create("timer2", 10, true, 0x40000001, 2).unwrap();
        let h3 = manager.create("timer3", 15, false, 0x40000002, 3).unwrap();

        manager.get_mut(h1).unwrap().start(5);
        manager.get_mut(h2).unwrap().start(10);
        manager.get_mut(h3).unwrap().start(15);

        let mut callbacks = Vec::new();
        for _ in 0..20 {
            manager.tick(&mut callbacks);
        }

        // Timer 1: fires at 5, 10, 15, 20 = 4 times
        // Timer 2: fires at 10, 20 = 2 times
        // Timer 3: fires at 15 = 1 time
        // Total: 7 callbacks
        assert_eq!(callbacks.len(), 7);
    }

    #[test]
    fn test_timer_deletion() {
        let mut manager = SoftwareTimerManager::new();
        let handle = manager.create("deletable", 10, true, 0x40000000, 6).unwrap();
        assert!(manager.get(handle).is_ok());

        manager.delete(handle).unwrap();
        assert!(manager.get(handle).is_err());
    }
}
