/// FreeRTOS mutex implementation with priority inheritance

use super::task::TaskHandle;
use super::scheduler::FreeRtosScheduler;
use std::fmt;

/// Mutex handle (index into mutex array)
pub type MutexHandle = usize;

/// Maximum number of mutexes
pub const MAX_MUTEXES: usize = 256;

/// Mutex type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexType {
    /// Normal mutex (can only lock once)
    Normal,
    /// Recursive mutex (same task can lock multiple times)
    Recursive,
}

impl fmt::Display for MutexType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MutexType::Normal => write!(f, "Normal"),
            MutexType::Recursive => write!(f, "Recursive"),
        }
    }
}

/// Mutex with priority inheritance
#[derive(Debug, Clone)]
pub struct Mutex {
    /// Mutex type
    mutex_type: MutexType,

    /// Current owner task (None if unlocked)
    owner: Option<TaskHandle>,

    /// Original priority of owner (for priority inheritance)
    original_priority: Option<u8>,

    /// Recursive lock count (for recursive mutexes)
    recursive_count: u32,

    /// Queue of waiting tasks (priority order)
    waiting_tasks: Vec<TaskHandle>,

    /// Mutex handle (for debugging)
    handle: MutexHandle,
}

impl Mutex {
    /// Create a new normal mutex
    pub fn new_normal(handle: MutexHandle) -> Self {
        Self {
            mutex_type: MutexType::Normal,
            owner: None,
            original_priority: None,
            recursive_count: 0,
            waiting_tasks: Vec::new(),
            handle,
        }
    }

    /// Create a new recursive mutex
    pub fn new_recursive(handle: MutexHandle) -> Self {
        Self {
            mutex_type: MutexType::Recursive,
            owner: None,
            original_priority: None,
            recursive_count: 0,
            waiting_tasks: Vec::new(),
            handle,
        }
    }

    /// Take mutex (blocking with timeout)
    /// Returns true if mutex was acquired, false if timeout
    pub fn take(&mut self, scheduler: &mut FreeRtosScheduler, timeout_ticks: u32) -> bool {
        let current_task_handle = match scheduler.get_current_task() {
            Some(h) => h,
            None => return false,
        };

        let current_priority = match scheduler.get_priority(current_task_handle) {
            Ok(p) => p,
            Err(_) => return false,
        };

        match self.owner {
            Some(owner_handle) => {
                // Mutex is locked

                // Check if same task (recursive mutex)
                if self.mutex_type == MutexType::Recursive && owner_handle == current_task_handle {
                    self.recursive_count += 1;
                    return true;
                }

                // Priority inheritance: boost owner priority if needed
                if current_priority > scheduler.get_priority(owner_handle).unwrap_or(0) {
                    if self.original_priority.is_none() {
                        self.original_priority = scheduler.get_priority(owner_handle).ok();
                    }

                    // Boost owner priority
                    let _ = scheduler.set_priority(owner_handle, current_priority);
                }

                // Block current task if timeout > 0
                if timeout_ticks > 0 {
                    self.waiting_tasks.push(current_task_handle);

                    // Set task delay (for timeout)
                    if timeout_ticks != u32::MAX {
                        let _ = scheduler.delay_task(timeout_ticks);
                    } else {
                        // Infinite timeout - just block
                        if let Ok(priority) = scheduler.get_priority(current_task_handle) {
                            scheduler.ready_queues[priority as usize].retain(|&h| h != current_task_handle);
                        }

                        if let Some(task) = scheduler.tasks.get_mut(current_task_handle)
                            .and_then(|t| t.as_mut()) {
                            task.set_state(super::task::TaskState::Blocked);
                        }
                    }

                    return false;
                }

                false
            }
            None => {
                // Mutex is available, acquire it
                self.owner = Some(current_task_handle);
                self.recursive_count = 1;
                true
            }
        }
    }

    /// Give mutex (unlock)
    /// Returns true if mutex was successfully released
    pub fn give(&mut self, scheduler: &mut FreeRtosScheduler) -> bool {
        let current_task_handle = match scheduler.get_current_task() {
            Some(h) => h,
            None => return false,
        };

        // Verify current task owns the mutex
        if self.owner != Some(current_task_handle) {
            return false;
        }

        // Handle recursive mutex
        if self.mutex_type == MutexType::Recursive && self.recursive_count > 1 {
            self.recursive_count -= 1;
            return true;
        }

        // Restore original priority if it was boosted
        if let Some(original_priority) = self.original_priority {
            let _ = scheduler.set_priority(current_task_handle, original_priority);
            self.original_priority = None;
        }

        // Release mutex
        self.owner = None;
        self.recursive_count = 0;

        // Wake highest priority waiting task
        if !self.waiting_tasks.is_empty() {
            // Find highest priority waiter
            let highest_priority_idx = self.waiting_tasks.iter()
                .enumerate()
                .max_by_key(|(_, &task_handle)| {
                    scheduler.get_priority(task_handle).unwrap_or(0)
                })
                .map(|(idx, _)| idx);

            if let Some(idx) = highest_priority_idx {
                let task_handle = self.waiting_tasks.remove(idx);

                // Wake the task
                if let Some(task) = scheduler.tasks.get_mut(task_handle)
                    .and_then(|t| t.as_mut()) {
                    task.set_state(super::task::TaskState::Ready);
                    task.delay_ticks = 0;

                    // Add back to ready queue
                    if let Ok(priority) = scheduler.get_priority(task_handle) {
                        scheduler.ready_queues[priority as usize].push(task_handle);
                    }
                }

                // Transfer ownership to woken task
                self.owner = Some(task_handle);
                self.recursive_count = 1;
            }
        }

        true
    }

    /// Check if mutex is locked
    pub fn is_locked(&self) -> bool {
        self.owner.is_some()
    }

    /// Get current owner
    pub fn get_owner(&self) -> Option<TaskHandle> {
        self.owner
    }

    /// Get number of waiting tasks
    pub fn get_waiting_count(&self) -> usize {
        self.waiting_tasks.len()
    }

    /// Get mutex type
    pub fn get_type(&self) -> MutexType {
        self.mutex_type
    }

    /// Get recursive count
    pub fn get_recursive_count(&self) -> u32 {
        self.recursive_count
    }
}

/// Global mutex storage
pub struct MutexManager {
    mutexes: Vec<Option<Mutex>>,
    next_handle: MutexHandle,
}

impl MutexManager {
    /// Create a new mutex manager
    pub fn new() -> Self {
        Self {
            mutexes: vec![None; MAX_MUTEXES],
            next_handle: 0,
        }
    }

    /// Create a normal mutex
    pub fn create_normal(&mut self) -> Result<MutexHandle, String> {
        let handle = self.mutexes.iter()
            .position(|m| m.is_none())
            .ok_or("Maximum number of mutexes reached")?;

        self.mutexes[handle] = Some(Mutex::new_normal(handle));
        self.next_handle = self.next_handle.max(handle + 1);

        Ok(handle)
    }

    /// Create a recursive mutex
    pub fn create_recursive(&mut self) -> Result<MutexHandle, String> {
        let handle = self.mutexes.iter()
            .position(|m| m.is_none())
            .ok_or("Maximum number of mutexes reached")?;

        self.mutexes[handle] = Some(Mutex::new_recursive(handle));
        self.next_handle = self.next_handle.max(handle + 1);

        Ok(handle)
    }

    /// Delete a mutex
    pub fn delete(&mut self, handle: MutexHandle) -> Result<(), String> {
        if handle >= MAX_MUTEXES {
            return Err("Invalid mutex handle".to_string());
        }

        self.mutexes[handle] = None;
        Ok(())
    }

    /// Get a mutex (mutable)
    pub fn get_mut(&mut self, handle: MutexHandle) -> Result<&mut Mutex, String> {
        self.mutexes.get_mut(handle)
            .and_then(|m| m.as_mut())
            .ok_or_else(|| "Invalid mutex handle".to_string())
    }

    /// Get a mutex (immutable)
    pub fn get(&self, handle: MutexHandle) -> Result<&Mutex, String> {
        self.mutexes.get(handle)
            .and_then(|m| m.as_ref())
            .ok_or_else(|| "Invalid mutex handle".to_string())
    }

    /// Reset manager (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for MutexManager {
    fn default() -> Self {
        Self::new()
    }
}

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex as StdMutex};

lazy_static! {
    /// Global mutex manager
    pub static ref MUTEX_MANAGER: Arc<StdMutex<MutexManager>> =
        Arc::new(StdMutex::new(MutexManager::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freertos::scheduler::FreeRtosScheduler;

    fn create_test_scheduler() -> FreeRtosScheduler {
        FreeRtosScheduler::new()
    }

    #[test]
    fn test_mutex_creation() {
        let mutex = Mutex::new_normal(0);
        assert_eq!(mutex.mutex_type, MutexType::Normal);
        assert!(mutex.owner.is_none());
        assert_eq!(mutex.recursive_count, 0);
        assert!(!mutex.is_locked());
    }

    #[test]
    fn test_mutex_lock_unlock() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task);

        // Lock mutex
        assert!(mutex.take(&mut scheduler, 0));
        assert!(mutex.is_locked());
        assert_eq!(mutex.get_owner(), Some(task));
        assert_eq!(mutex.recursive_count, 1);

        // Unlock mutex
        assert!(mutex.give(&mut scheduler));
        assert!(!mutex.is_locked());
        assert_eq!(mutex.get_owner(), None);
        assert_eq!(mutex.recursive_count, 0);
    }

    #[test]
    fn test_recursive_mutex() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_recursive(0);

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task);

        // Lock 3 times
        assert!(mutex.take(&mut scheduler, 0));
        assert_eq!(mutex.recursive_count, 1);

        assert!(mutex.take(&mut scheduler, 0));
        assert_eq!(mutex.recursive_count, 2);

        assert!(mutex.take(&mut scheduler, 0));
        assert_eq!(mutex.recursive_count, 3);

        // Unlock 2 times - still locked
        assert!(mutex.give(&mut scheduler));
        assert_eq!(mutex.recursive_count, 2);
        assert!(mutex.is_locked());

        assert!(mutex.give(&mut scheduler));
        assert_eq!(mutex.recursive_count, 1);
        assert!(mutex.is_locked());

        // Final unlock
        assert!(mutex.give(&mut scheduler));
        assert_eq!(mutex.recursive_count, 0);
        assert!(!mutex.is_locked());
    }

    #[test]
    fn test_normal_mutex_no_recursion() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task);

        // Lock once
        assert!(mutex.take(&mut scheduler, 0));

        // Try to lock again (should fail for normal mutex with timeout=0)
        assert!(!mutex.take(&mut scheduler, 0));
    }

    #[test]
    fn test_mutex_blocking() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        // Task 1 locks mutex
        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task1);
        assert!(mutex.take(&mut scheduler, 0));

        // Task 2 tries to lock (should block)
        let task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task2);
        assert!(!mutex.take(&mut scheduler, 10));

        // Task 2 should be in waiting queue
        assert_eq!(mutex.waiting_tasks.len(), 1);
        assert_eq!(mutex.waiting_tasks[0], task2);
    }

    #[test]
    fn test_mutex_wake_waiter() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        // Task 1 locks mutex
        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task1);
        mutex.take(&mut scheduler, 0);

        // Task 2 blocks on mutex
        let task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task2);
        mutex.take(&mut scheduler, u32::MAX);

        assert_eq!(mutex.waiting_tasks.len(), 1);

        // Task 1 unlocks - should wake task 2 and transfer ownership
        scheduler.current_task = Some(task1);
        assert!(mutex.give(&mut scheduler));

        assert_eq!(mutex.waiting_tasks.len(), 0);
        assert_eq!(mutex.get_owner(), Some(task2)); // Ownership transferred
    }

    #[test]
    fn test_priority_inheritance() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        // Low priority task locks mutex
        let low_task = scheduler.create_task(0x40000000, "low", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(low_task);
        mutex.take(&mut scheduler, 0);

        assert_eq!(scheduler.get_priority(low_task).unwrap(), 5);

        // High priority task tries to lock (should boost low task's priority)
        let high_task = scheduler.create_task(0x40000100, "high", 4096, 0, 15).unwrap();
        scheduler.current_task = Some(high_task);
        mutex.take(&mut scheduler, 10);

        // Low task's priority should be boosted
        assert_eq!(scheduler.get_priority(low_task).unwrap(), 15);
        assert_eq!(mutex.original_priority, Some(5));

        // Low task unlocks - priority should be restored
        scheduler.current_task = Some(low_task);
        mutex.give(&mut scheduler);

        assert_eq!(scheduler.get_priority(low_task).unwrap(), 5);
    }

    #[test]
    fn test_priority_inheritance_multiple_waiters() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        // Low priority task locks mutex
        let low_task = scheduler.create_task(0x40000000, "low", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(low_task);
        mutex.take(&mut scheduler, 0);

        // Medium priority waiter
        let med_task = scheduler.create_task(0x40000100, "med", 4096, 0, 10).unwrap();
        scheduler.current_task = Some(med_task);
        mutex.take(&mut scheduler, 10);

        // Priority boosted to 10
        assert_eq!(scheduler.get_priority(low_task).unwrap(), 10);

        // High priority waiter (should boost further)
        let high_task = scheduler.create_task(0x40000200, "high", 4096, 0, 20).unwrap();
        scheduler.current_task = Some(high_task);
        mutex.take(&mut scheduler, 10);

        // Priority boosted to 20
        assert_eq!(scheduler.get_priority(low_task).unwrap(), 20);

        // Unlock - should give to highest priority waiter
        scheduler.current_task = Some(low_task);
        mutex.give(&mut scheduler);

        // Highest priority waiter (high_task) should get ownership
        assert_eq!(mutex.get_owner(), Some(high_task));
    }

    #[test]
    fn test_mutex_manager() {
        let mut manager = MutexManager::new();

        // Create normal mutex
        let norm_handle = manager.create_normal().unwrap();
        assert_eq!(norm_handle, 0);

        let norm_mutex = manager.get(norm_handle).unwrap();
        assert_eq!(norm_mutex.get_type(), MutexType::Normal);

        // Create recursive mutex
        let rec_handle = manager.create_recursive().unwrap();
        assert_eq!(rec_handle, 1);

        let rec_mutex = manager.get(rec_handle).unwrap();
        assert_eq!(rec_mutex.get_type(), MutexType::Recursive);

        // Delete mutex
        manager.delete(norm_handle).unwrap();
        assert!(manager.get(norm_handle).is_err());
    }

    #[test]
    fn test_unlock_without_ownership() {
        let mut scheduler = create_test_scheduler();
        let mut mutex = Mutex::new_normal(0);

        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        let task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();

        // Task 1 locks
        scheduler.current_task = Some(task1);
        mutex.take(&mut scheduler, 0);

        // Task 2 tries to unlock (should fail)
        scheduler.current_task = Some(task2);
        assert!(!mutex.give(&mut scheduler));

        // Mutex should still be locked by task 1
        assert_eq!(mutex.get_owner(), Some(task1));
    }
}
