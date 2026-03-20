/// FreeRTOS event group implementation for task synchronization
///
/// Event groups provide a way for tasks to wait for multiple conditions using a 32-bit bitfield.

use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use super::scheduler::{SCHEDULER, FreeRtosScheduler};
use super::task::TaskHandle;

pub type EventGroupHandle = usize;
pub const MAX_EVENT_GROUPS: usize = 128;

/// Event group (32-bit bitfield)
///
/// In FreeRTOS, event groups use a 24-bit bitfield (bits 0-23).
/// The upper 8 bits are reserved for control flags.
pub struct EventGroup {
    /// Event group handle (for debugging)
    handle: EventGroupHandle,

    /// Current event bits (24 usable bits)
    bits: u32,

    /// Waiting tasks
    waiters: Vec<EventWaiter>,
}

/// Event waiter information
struct EventWaiter {
    /// Task waiting on events
    task_handle: TaskHandle,

    /// Bits to wait for
    wait_bits: u32,

    /// Wait for all bits vs. any bit
    wait_all: bool,

    /// Clear bits on exit
    clear_on_exit: bool,
}

impl EventGroup {
    pub fn new(handle: EventGroupHandle) -> Self {
        Self {
            handle,
            bits: 0,
            waiters: Vec::new(),
        }
    }

    /// Set event bits
    pub fn set_bits(&mut self, bits: u32, scheduler: &mut FreeRtosScheduler) -> u32 {
        // Mask to 24 bits
        let bits = bits & 0x00FFFFFF;
        self.bits |= bits;

        // Wake tasks whose wait conditions are met
        let mut to_wake = Vec::new();
        self.waiters.retain(|waiter| {
            let satisfied = if waiter.wait_all {
                (self.bits & waiter.wait_bits) == waiter.wait_bits
            } else {
                (self.bits & waiter.wait_bits) != 0
            };

            if satisfied {
                to_wake.push((waiter.task_handle, waiter.clear_on_exit, waiter.wait_bits));
                false  // Remove from waiters
            } else {
                true  // Keep waiting
            }
        });

        // Wake tasks and clear bits if requested
        for (task_handle, clear_on_exit, wait_bits) in to_wake {
            let _ = scheduler.wake_task(task_handle);
            if clear_on_exit {
                self.bits &= !wait_bits;
            }
        }

        self.bits
    }

    /// Wait for event bits
    pub fn wait_bits(
        &mut self,
        wait_bits: u32,
        clear_on_exit: bool,
        wait_all: bool,
        timeout_ticks: u32,
        scheduler: &mut FreeRtosScheduler,
    ) -> u32 {
        // Mask to 24 bits
        let wait_bits = wait_bits & 0x00FFFFFF;

        let satisfied = if wait_all {
            (self.bits & wait_bits) == wait_bits
        } else {
            (self.bits & wait_bits) != 0
        };

        if satisfied {
            let result = self.bits;
            if clear_on_exit {
                self.bits &= !wait_bits;
            }
            result
        } else if timeout_ticks == 0 {
            0  // Non-blocking, not satisfied
        } else {
            // Block current task
            if let Some(task_handle) = scheduler.get_current_task() {
                self.waiters.push(EventWaiter {
                    task_handle,
                    wait_bits,
                    wait_all,
                    clear_on_exit,
                });
                let _ = scheduler.block_task(task_handle, timeout_ticks);
            }
            0  // Return 0 when blocking
        }
    }

    /// Clear event bits
    pub fn clear_bits(&mut self, bits: u32) -> u32 {
        let bits = bits & 0x00FFFFFF;
        self.bits &= !bits;
        self.bits
    }

    /// Get current event bits
    pub fn get_bits(&self) -> u32 {
        self.bits
    }
}

/// Event group manager
pub struct EventGroupManager {
    groups: Vec<Option<EventGroup>>,
}

impl EventGroupManager {
    pub fn new() -> Self {
        let mut groups = Vec::with_capacity(MAX_EVENT_GROUPS);
        for _ in 0..MAX_EVENT_GROUPS {
            groups.push(None);
        }
        Self { groups }
    }

    /// Create a new event group
    pub fn create(&mut self) -> Result<EventGroupHandle, String> {
        let handle = self.groups.iter().position(|g| g.is_none())
            .ok_or("Maximum event groups reached")?;

        self.groups[handle] = Some(EventGroup::new(handle));
        Ok(handle)
    }

    /// Delete an event group
    pub fn delete(&mut self, handle: EventGroupHandle) -> Result<(), String> {
        if handle >= MAX_EVENT_GROUPS {
            return Err("Invalid event group handle".to_string());
        }
        self.groups[handle] = None;
        Ok(())
    }

    /// Get mutable reference to event group
    pub fn get_mut(&mut self, handle: EventGroupHandle) -> Result<&mut EventGroup, String> {
        self.groups.get_mut(handle)
            .and_then(|g| g.as_mut())
            .ok_or_else(|| "Invalid event group handle".to_string())
    }

    /// Get immutable reference to event group
    pub fn get(&self, handle: EventGroupHandle) -> Result<&EventGroup, String> {
        self.groups.get(handle)
            .and_then(|g| g.as_ref())
            .ok_or_else(|| "Invalid event group handle".to_string())
    }
}

lazy_static! {
    /// Global event group manager instance
    pub static ref EVENT_GROUP_MANAGER: Arc<Mutex<EventGroupManager>> =
        Arc::new(Mutex::new(EventGroupManager::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_group_creation() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        assert_eq!(handle, 0);

        let group = manager.get(handle).unwrap();
        assert_eq!(group.get_bits(), 0);
    }

    #[test]
    fn test_set_clear_bits() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        let mut scheduler = FreeRtosScheduler::new();

        // Set bits 0, 1, 2
        let group = manager.get_mut(handle).unwrap();
        group.set_bits(0b111, &mut scheduler);
        assert_eq!(group.get_bits(), 0b111);

        // Set more bits
        group.set_bits(0b11000, &mut scheduler);
        assert_eq!(group.get_bits(), 0b11111);

        // Clear some bits
        group.clear_bits(0b10101);
        assert_eq!(group.get_bits(), 0b01010);
    }

    #[test]
    fn test_wait_any_bit() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        let mut scheduler = FreeRtosScheduler::new();

        // Set bits 0 and 2
        let group = manager.get_mut(handle).unwrap();
        group.set_bits(0b101, &mut scheduler);

        // Wait for any of bits 0, 1, 2 (should succeed immediately)
        let result = group.wait_bits(0b111, false, false, 0, &mut scheduler);
        assert_eq!(result, 0b101);  // Returns current bits
    }

    #[test]
    fn test_wait_all_bits() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        let mut scheduler = FreeRtosScheduler::new();

        // Set only bits 0 and 2
        let group = manager.get_mut(handle).unwrap();
        group.set_bits(0b101, &mut scheduler);

        // Wait for ALL of bits 0, 1, 2 (should fail, bit 1 not set)
        let result = group.wait_bits(0b111, false, true, 0, &mut scheduler);
        assert_eq!(result, 0);  // Not satisfied

        // Set bit 1
        group.set_bits(0b010, &mut scheduler);

        // Now wait for ALL should succeed
        let result = group.wait_bits(0b111, false, true, 0, &mut scheduler);
        assert_eq!(result, 0b111);
    }

    #[test]
    fn test_clear_on_exit() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        let mut scheduler = FreeRtosScheduler::new();

        // Set bits 0, 1, 2
        let group = manager.get_mut(handle).unwrap();
        group.set_bits(0b111, &mut scheduler);
        assert_eq!(group.get_bits(), 0b111);

        // Wait for bits 0 and 1, clear on exit
        let result = group.wait_bits(0b011, true, true, 0, &mut scheduler);
        assert_eq!(result, 0b111);  // Returns bits before clear
        assert_eq!(group.get_bits(), 0b100);  // Bits 0 and 1 cleared
    }

    #[test]
    fn test_24_bit_mask() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        let mut scheduler = FreeRtosScheduler::new();

        // Try to set upper 8 bits (should be masked out)
        let group = manager.get_mut(handle).unwrap();
        group.set_bits(0xFF000001, &mut scheduler);
        assert_eq!(group.get_bits(), 0x00000001);  // Only bit 0 set
    }

    #[test]
    fn test_multiple_event_groups() {
        let mut manager = EventGroupManager::new();
        let h1 = manager.create().unwrap();
        let h2 = manager.create().unwrap();
        let h3 = manager.create().unwrap();

        assert_eq!(h1, 0);
        assert_eq!(h2, 1);
        assert_eq!(h3, 2);

        let mut scheduler = FreeRtosScheduler::new();

        // Set different bits in each group
        manager.get_mut(h1).unwrap().set_bits(0b001, &mut scheduler);
        manager.get_mut(h2).unwrap().set_bits(0b010, &mut scheduler);
        manager.get_mut(h3).unwrap().set_bits(0b100, &mut scheduler);

        assert_eq!(manager.get(h1).unwrap().get_bits(), 0b001);
        assert_eq!(manager.get(h2).unwrap().get_bits(), 0b010);
        assert_eq!(manager.get(h3).unwrap().get_bits(), 0b100);
    }

    #[test]
    fn test_event_group_deletion() {
        let mut manager = EventGroupManager::new();
        let handle = manager.create().unwrap();
        assert!(manager.get(handle).is_ok());

        manager.delete(handle).unwrap();
        assert!(manager.get(handle).is_err());
    }
}
