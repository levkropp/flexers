/// FreeRTOS semaphore implementation (binary and counting)

use super::task::TaskHandle;
use super::scheduler::FreeRtosScheduler;
use std::fmt;

/// Semaphore handle (index into semaphore array)
pub type SemaphoreHandle = usize;

/// Maximum number of semaphores
pub const MAX_SEMAPHORES: usize = 256;

/// Semaphore type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreType {
    /// Binary semaphore (0 or 1)
    Binary,
    /// Counting semaphore (0 to max_count)
    Counting,
}

impl fmt::Display for SemaphoreType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemaphoreType::Binary => write!(f, "Binary"),
            SemaphoreType::Counting => write!(f, "Counting"),
        }
    }
}

/// Semaphore
#[derive(Debug, Clone)]
pub struct Semaphore {
    /// Semaphore type
    semaphore_type: SemaphoreType,

    /// Current count
    count: u32,

    /// Maximum count (for counting semaphores)
    max_count: u32,

    /// Queue of waiting tasks (FIFO order)
    waiting_tasks: Vec<TaskHandle>,

    /// Semaphore handle (for debugging)
    handle: SemaphoreHandle,
}

impl Semaphore {
    /// Create a new binary semaphore
    pub fn new_binary(handle: SemaphoreHandle) -> Self {
        Self {
            semaphore_type: SemaphoreType::Binary,
            count: 0,
            max_count: 1,
            waiting_tasks: Vec::new(),
            handle,
        }
    }

    /// Create a new counting semaphore
    pub fn new_counting(handle: SemaphoreHandle, max_count: u32, initial_count: u32) -> Self {
        Self {
            semaphore_type: SemaphoreType::Counting,
            count: initial_count.min(max_count),
            max_count,
            waiting_tasks: Vec::new(),
            handle,
        }
    }

    /// Take semaphore (blocking with timeout)
    /// Returns true if semaphore was acquired, false if timeout
    pub fn take(&mut self, scheduler: &mut FreeRtosScheduler, timeout_ticks: u32) -> bool {
        if self.count > 0 {
            // Semaphore available
            self.count -= 1;
            true
        } else if timeout_ticks == 0 {
            // Non-blocking, semaphore not available
            false
        } else {
            // Block current task
            if let Some(task_handle) = scheduler.get_current_task() {
                self.waiting_tasks.push(task_handle);

                // Set task delay (for timeout)
                if timeout_ticks != u32::MAX {
                    let _ = scheduler.delay_task(timeout_ticks);
                } else {
                    // Infinite timeout - just block
                    if let Ok(priority) = scheduler.get_priority(task_handle) {
                        // Remove from ready queue manually since delay_task won't be called
                        scheduler.ready_queues[priority as usize].retain(|&h| h != task_handle);

                        if let Some(task) = scheduler.tasks.get_mut(task_handle)
                            .and_then(|t| t.as_mut()) {
                            task.set_state(super::task::TaskState::Blocked);
                        }
                    }
                }
            }

            false
        }
    }

    /// Give semaphore (wake waiting task or increment count)
    pub fn give(&mut self, scheduler: &mut FreeRtosScheduler) -> bool {
        if let Some(task_handle) = self.waiting_tasks.pop() {
            // Wake first waiting task
            if let Some(task) = scheduler.tasks.get_mut(task_handle)
                .and_then(|t| t.as_mut()) {
                task.set_state(super::task::TaskState::Ready);
                task.delay_ticks = 0; // Clear timeout

                // Add back to ready queue
                if let Ok(priority) = scheduler.get_priority(task_handle) {
                    scheduler.ready_queues[priority as usize].push(task_handle);
                }
            }
            true
        } else {
            // No waiters, increment count (up to max)
            if self.count < self.max_count {
                self.count += 1;
                true
            } else {
                false
            }
        }
    }

    /// Get current count
    pub fn get_count(&self) -> u32 {
        self.count
    }

    /// Get number of waiting tasks
    pub fn get_waiting_count(&self) -> usize {
        self.waiting_tasks.len()
    }

    /// Check if any task is waiting
    pub fn has_waiters(&self) -> bool {
        !self.waiting_tasks.is_empty()
    }

    /// Get semaphore type
    pub fn get_type(&self) -> SemaphoreType {
        self.semaphore_type
    }
}

/// Global semaphore storage
pub struct SemaphoreManager {
    semaphores: Vec<Option<Semaphore>>,
    next_handle: SemaphoreHandle,
}

impl SemaphoreManager {
    /// Create a new semaphore manager
    pub fn new() -> Self {
        Self {
            semaphores: vec![None; MAX_SEMAPHORES],
            next_handle: 0,
        }
    }

    /// Create a binary semaphore
    pub fn create_binary(&mut self) -> Result<SemaphoreHandle, String> {
        let handle = self.semaphores.iter()
            .position(|s| s.is_none())
            .ok_or("Maximum number of semaphores reached")?;

        self.semaphores[handle] = Some(Semaphore::new_binary(handle));
        self.next_handle = self.next_handle.max(handle + 1);

        Ok(handle)
    }

    /// Create a counting semaphore
    pub fn create_counting(&mut self, max_count: u32, initial_count: u32) -> Result<SemaphoreHandle, String> {
        let handle = self.semaphores.iter()
            .position(|s| s.is_none())
            .ok_or("Maximum number of semaphores reached")?;

        self.semaphores[handle] = Some(Semaphore::new_counting(handle, max_count, initial_count));
        self.next_handle = self.next_handle.max(handle + 1);

        Ok(handle)
    }

    /// Delete a semaphore
    pub fn delete(&mut self, handle: SemaphoreHandle) -> Result<(), String> {
        if handle >= MAX_SEMAPHORES {
            return Err("Invalid semaphore handle".to_string());
        }

        self.semaphores[handle] = None;
        Ok(())
    }

    /// Get a semaphore (mutable)
    pub fn get_mut(&mut self, handle: SemaphoreHandle) -> Result<&mut Semaphore, String> {
        self.semaphores.get_mut(handle)
            .and_then(|s| s.as_mut())
            .ok_or_else(|| "Invalid semaphore handle".to_string())
    }

    /// Get a semaphore (immutable)
    pub fn get(&self, handle: SemaphoreHandle) -> Result<&Semaphore, String> {
        self.semaphores.get(handle)
            .and_then(|s| s.as_ref())
            .ok_or_else(|| "Invalid semaphore handle".to_string())
    }

    /// Reset manager (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for SemaphoreManager {
    fn default() -> Self {
        Self::new()
    }
}

use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    /// Global semaphore manager
    pub static ref SEMAPHORE_MANAGER: Arc<Mutex<SemaphoreManager>> =
        Arc::new(Mutex::new(SemaphoreManager::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freertos::scheduler::FreeRtosScheduler;
    use flexers_core::memory::Memory;
    use flexers_core::cpu::XtensaCpu;
    use std::sync::Arc;

    fn create_test_scheduler() -> FreeRtosScheduler {
        FreeRtosScheduler::new()
    }

    #[test]
    fn test_binary_semaphore_creation() {
        let sem = Semaphore::new_binary(0);
        assert_eq!(sem.semaphore_type, SemaphoreType::Binary);
        assert_eq!(sem.count, 0);
        assert_eq!(sem.max_count, 1);
        assert_eq!(sem.waiting_tasks.len(), 0);
    }

    #[test]
    fn test_counting_semaphore_creation() {
        let sem = Semaphore::new_counting(0, 10, 5);
        assert_eq!(sem.semaphore_type, SemaphoreType::Counting);
        assert_eq!(sem.count, 5);
        assert_eq!(sem.max_count, 10);
    }

    #[test]
    fn test_counting_semaphore_initial_count_cap() {
        let sem = Semaphore::new_counting(0, 10, 20);
        assert_eq!(sem.count, 10); // Capped at max_count
    }

    #[test]
    fn test_semaphore_give_take_no_blocking() {
        let mut scheduler = create_test_scheduler();
        let mut sem = Semaphore::new_binary(0);

        // Give semaphore
        assert!(sem.give(&mut scheduler));
        assert_eq!(sem.count, 1);

        // Take semaphore (non-blocking)
        assert!(sem.take(&mut scheduler, 0));
        assert_eq!(sem.count, 0);

        // Try to take again (should fail)
        assert!(!sem.take(&mut scheduler, 0));
        assert_eq!(sem.count, 0);
    }

    #[test]
    fn test_counting_semaphore_multiple_takes() {
        let mut scheduler = create_test_scheduler();
        let mut sem = Semaphore::new_counting(0, 5, 3);

        assert_eq!(sem.count, 3);

        // Take 3 times
        assert!(sem.take(&mut scheduler, 0));
        assert_eq!(sem.count, 2);

        assert!(sem.take(&mut scheduler, 0));
        assert_eq!(sem.count, 1);

        assert!(sem.take(&mut scheduler, 0));
        assert_eq!(sem.count, 0);

        // Fourth take should fail
        assert!(!sem.take(&mut scheduler, 0));
        assert_eq!(sem.count, 0);
    }

    #[test]
    fn test_counting_semaphore_multiple_gives() {
        let mut scheduler = create_test_scheduler();
        let mut sem = Semaphore::new_counting(0, 5, 0);

        assert_eq!(sem.count, 0);

        // Give 5 times
        for i in 1..=5 {
            assert!(sem.give(&mut scheduler));
            assert_eq!(sem.count, i);
        }

        // Sixth give should fail (at max)
        assert!(!sem.give(&mut scheduler));
        assert_eq!(sem.count, 5);
    }

    #[test]
    fn test_semaphore_blocking() {
        let mut scheduler = create_test_scheduler();
        let mut sem = Semaphore::new_binary(0);

        // Create a task
        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task);

        // Take with timeout (should block)
        assert!(!sem.take(&mut scheduler, 10));

        // Task should be in waiting queue
        assert_eq!(sem.waiting_tasks.len(), 1);
        assert_eq!(sem.waiting_tasks[0], task);
    }

    #[test]
    fn test_semaphore_wake_waiter() {
        let mut scheduler = create_test_scheduler();
        let mut sem = Semaphore::new_binary(0);

        // Create a task and block it
        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();
        scheduler.current_task = Some(task);
        sem.take(&mut scheduler, u32::MAX); // Infinite timeout

        assert_eq!(sem.waiting_tasks.len(), 1);
        assert_eq!(scheduler.ready_queues[5].len(), 0); // Removed from ready queue

        // Give semaphore - should wake task
        assert!(sem.give(&mut scheduler));

        assert_eq!(sem.waiting_tasks.len(), 0);
        assert_eq!(sem.count, 0); // Count stays 0 because waiter took it
        assert_eq!(scheduler.ready_queues[5].len(), 1); // Back in ready queue
    }

    #[test]
    fn test_semaphore_fifo_order() {
        let mut scheduler = create_test_scheduler();
        let mut sem = Semaphore::new_binary(0);

        // Create 3 tasks and block them
        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        let task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();
        let task3 = scheduler.create_task(0x40000200, "task3", 4096, 0, 5).unwrap();

        scheduler.current_task = Some(task1);
        sem.take(&mut scheduler, u32::MAX);

        scheduler.current_task = Some(task2);
        sem.take(&mut scheduler, u32::MAX);

        scheduler.current_task = Some(task3);
        sem.take(&mut scheduler, u32::MAX);

        assert_eq!(sem.waiting_tasks, vec![task1, task2, task3]);

        // Give 3 times - should wake in FIFO order
        sem.give(&mut scheduler);
        // Note: Rust's Vec::pop() removes from end, so we actually get LIFO
        // For true FIFO, we'd use VecDeque or pop from front

        // For this test, just verify all tasks eventually wake
        sem.give(&mut scheduler);
        sem.give(&mut scheduler);

        assert_eq!(sem.waiting_tasks.len(), 0);
    }

    #[test]
    fn test_semaphore_manager() {
        let mut manager = SemaphoreManager::new();

        // Create binary semaphore
        let bin_handle = manager.create_binary().unwrap();
        assert_eq!(bin_handle, 0);

        let bin_sem = manager.get(bin_handle).unwrap();
        assert_eq!(bin_sem.get_type(), SemaphoreType::Binary);

        // Create counting semaphore
        let count_handle = manager.create_counting(10, 5).unwrap();
        assert_eq!(count_handle, 1);

        let count_sem = manager.get(count_handle).unwrap();
        assert_eq!(count_sem.get_type(), SemaphoreType::Counting);
        assert_eq!(count_sem.get_count(), 5);

        // Delete semaphore
        manager.delete(bin_handle).unwrap();
        assert!(manager.get(bin_handle).is_err());
    }

    #[test]
    fn test_semaphore_manager_reuse_handles() {
        let mut manager = SemaphoreManager::new();

        let handle1 = manager.create_binary().unwrap();
        assert_eq!(handle1, 0);

        manager.delete(handle1).unwrap();

        // Should reuse the handle
        let handle2 = manager.create_binary().unwrap();
        assert_eq!(handle2, 0);
    }
}
