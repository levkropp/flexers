/// FreeRTOS queue implementation for inter-task communication
///
/// Queues provide FIFO message passing between tasks with blocking send/receive operations.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use flexers_core::cpu::XtensaCpu;
use super::scheduler::{SCHEDULER, FreeRtosScheduler};
use super::task::TaskHandle;

pub type QueueHandle = usize;
pub const MAX_QUEUES: usize = 256;

/// Queue for inter-task communication
pub struct Queue {
    /// Queue handle (for debugging)
    handle: QueueHandle,

    /// Item size in bytes
    item_size: usize,

    /// Maximum number of items
    capacity: usize,

    /// Current items (stored as byte vectors)
    items: VecDeque<Vec<u8>>,

    /// Tasks waiting to send (queue full)
    send_waiters: Vec<TaskHandle>,

    /// Tasks waiting to receive (queue empty)
    recv_waiters: Vec<TaskHandle>,
}

impl Queue {
    pub fn new(handle: QueueHandle, item_size: usize, capacity: usize) -> Self {
        Self {
            handle,
            item_size,
            capacity,
            items: VecDeque::with_capacity(capacity),
            send_waiters: Vec::new(),
            recv_waiters: Vec::new(),
        }
    }

    /// Send item to queue (blocking with timeout)
    pub fn send(
        &mut self,
        scheduler: &mut FreeRtosScheduler,
        item_ptr: u32,
        timeout_ticks: u32,
        cpu: &XtensaCpu,
    ) -> bool {
        if self.items.len() < self.capacity {
            // Space available, copy item from memory
            let mut item = vec![0u8; self.item_size];
            for i in 0..self.item_size {
                item[i] = cpu.memory().read_u8(item_ptr + i as u32);
            }
            self.items.push_back(item);

            // Wake a receiver if waiting
            if let Some(task_handle) = self.recv_waiters.pop() {
                let _ = scheduler.wake_task(task_handle);
            }

            true
        } else if timeout_ticks == 0 {
            // No space, non-blocking
            false
        } else {
            // Block current task
            if let Some(task_handle) = scheduler.get_current_task() {
                self.send_waiters.push(task_handle);
                let _ = scheduler.block_task(task_handle, timeout_ticks);
            }
            false
        }
    }

    /// Receive item from queue (blocking with timeout)
    pub fn receive(
        &mut self,
        scheduler: &mut FreeRtosScheduler,
        item_ptr: u32,
        timeout_ticks: u32,
        cpu: &mut XtensaCpu,
    ) -> bool {
        if let Some(item) = self.items.pop_front() {
            // Item available, copy to memory
            for i in 0..self.item_size {
                cpu.memory().write_u8(item_ptr + i as u32, item[i]);
            }

            // Wake a sender if waiting
            if let Some(task_handle) = self.send_waiters.pop() {
                let _ = scheduler.wake_task(task_handle);
            }

            true
        } else if timeout_ticks == 0 {
            // No items, non-blocking
            false
        } else {
            // Block current task
            if let Some(task_handle) = scheduler.get_current_task() {
                self.recv_waiters.push(task_handle);
                let _ = scheduler.block_task(task_handle, timeout_ticks);
            }
            false
        }
    }

    /// Get number of items in queue
    pub fn get_count(&self) -> usize {
        self.items.len()
    }

    /// Get available space in queue
    pub fn get_space(&self) -> usize {
        self.capacity - self.items.len()
    }

    /// Reset queue (clear all items)
    pub fn reset(&mut self) {
        self.items.clear();
        // Note: waiters are NOT woken on reset
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.items.len() >= self.capacity
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Send to queue from ISR (non-blocking)
    pub fn send_from_isr(
        &mut self,
        item_ptr: u32,
        cpu: &XtensaCpu,
    ) -> bool {
        if self.items.len() < self.capacity {
            let mut item = vec![0u8; self.item_size];
            for i in 0..self.item_size {
                item[i] = cpu.memory().read_u8(item_ptr + i as u32);
            }
            self.items.push_back(item);
            true
        } else {
            false
        }
    }

    /// Receive from queue from ISR (non-blocking)
    pub fn receive_from_isr(
        &mut self,
        item_ptr: u32,
        cpu: &mut XtensaCpu,
    ) -> bool {
        if let Some(item) = self.items.pop_front() {
            for i in 0..self.item_size {
                cpu.memory().write_u8(item_ptr + i as u32, item[i]);
            }
            true
        } else {
            false
        }
    }
}

/// Queue manager
pub struct QueueManager {
    queues: Vec<Option<Queue>>,
}

impl QueueManager {
    pub fn new() -> Self {
        let mut queues = Vec::with_capacity(MAX_QUEUES);
        for _ in 0..MAX_QUEUES {
            queues.push(None);
        }
        Self { queues }
    }

    /// Create a new queue
    pub fn create(&mut self, item_size: usize, capacity: usize) -> Result<QueueHandle, String> {
        let handle = self.queues.iter().position(|q| q.is_none())
            .ok_or("Maximum queues reached")?;

        self.queues[handle] = Some(Queue::new(handle, item_size, capacity));
        Ok(handle)
    }

    /// Delete a queue
    pub fn delete(&mut self, handle: QueueHandle) -> Result<(), String> {
        if handle >= MAX_QUEUES {
            return Err("Invalid queue handle".to_string());
        }
        self.queues[handle] = None;
        Ok(())
    }

    /// Get mutable reference to queue
    pub fn get_mut(&mut self, handle: QueueHandle) -> Result<&mut Queue, String> {
        self.queues.get_mut(handle)
            .and_then(|q| q.as_mut())
            .ok_or_else(|| "Invalid queue handle".to_string())
    }

    /// Get immutable reference to queue
    pub fn get(&self, handle: QueueHandle) -> Result<&Queue, String> {
        self.queues.get(handle)
            .and_then(|q| q.as_ref())
            .ok_or_else(|| "Invalid queue handle".to_string())
    }
}

lazy_static! {
    /// Global queue manager instance
    pub static ref QUEUE_MANAGER: Arc<Mutex<QueueManager>> =
        Arc::new(Mutex::new(QueueManager::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexers_core::memory::Memory;
    use flexers_core::cpu::XtensaCpu;
    use std::sync::Arc;

    fn create_test_cpu() -> XtensaCpu {
        XtensaCpu::new(Arc::new(Memory::new()))
    }

    #[test]
    fn test_queue_creation() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 10).unwrap();
        assert_eq!(handle, 0);

        let queue = manager.get(handle).unwrap();
        assert_eq!(queue.get_count(), 0);
        assert_eq!(queue.get_space(), 10);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
    }

    #[test]
    fn test_queue_send_receive_basic() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 10).unwrap();
        let mut cpu = create_test_cpu();

        // Write test value to memory
        let test_ptr = 0x3FFE_0000;
        cpu.memory().write_u32(test_ptr, 0x12345678);

        // Send to queue (non-blocking)
        let queue = manager.get_mut(handle).unwrap();
        let mut scheduler = super::super::scheduler::FreeRtosScheduler::new();
        let result = queue.send(&mut scheduler, test_ptr, 0, &cpu);
        assert!(result);
        assert_eq!(queue.get_count(), 1);

        // Receive from queue
        let recv_ptr = 0x3FFE_0100;
        let result = queue.receive(&mut scheduler, recv_ptr, 0, &mut cpu);
        assert!(result);
        assert_eq!(cpu.memory().read_u32(recv_ptr), 0x12345678);
        assert_eq!(queue.get_count(), 0);
    }

    #[test]
    fn test_queue_fifo_ordering() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 10).unwrap();
        let mut cpu = create_test_cpu();
        let mut scheduler = super::super::scheduler::FreeRtosScheduler::new();

        // Send multiple values
        for i in 0..5u32 {
            let ptr = 0x3FFE_0000 + (i * 4);
            cpu.memory().write_u32(ptr, i);
            let queue = manager.get_mut(handle).unwrap();
            queue.send(&mut scheduler, ptr, 0, &cpu);
        }

        // Receive and verify FIFO order
        for i in 0..5u32 {
            let ptr = 0x3FFE_0100 + (i * 4);
            let queue = manager.get_mut(handle).unwrap();
            queue.receive(&mut scheduler, ptr, 0, &mut cpu);
            assert_eq!(cpu.memory().read_u32(ptr), i);
        }
    }

    #[test]
    fn test_queue_full() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 2).unwrap(); // Small queue (capacity 2)
        let cpu = create_test_cpu();
        let mut scheduler = super::super::scheduler::FreeRtosScheduler::new();

        // Fill queue
        for i in 0..2u32 {
            let ptr = 0x3FFE_0000 + (i * 4);
            cpu.memory().write_u32(ptr, i);
            let queue = manager.get_mut(handle).unwrap();
            let result = queue.send(&mut scheduler, ptr, 0, &cpu);
            assert!(result);
        }

        // Queue should be full
        let queue = manager.get(handle).unwrap();
        assert!(queue.is_full());
        assert_eq!(queue.get_space(), 0);

        // Try to send one more (should fail, non-blocking)
        let ptr = 0x3FFE_0010;
        cpu.memory().write_u32(ptr, 999);
        let queue = manager.get_mut(handle).unwrap();
        let result = queue.send(&mut scheduler, ptr, 0, &cpu);
        assert!(!result);
    }

    #[test]
    fn test_queue_empty() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 10).unwrap();
        let mut scheduler = super::super::scheduler::FreeRtosScheduler::new();
        let mut cpu = create_test_cpu();

        // Try to receive from empty queue (non-blocking)
        let ptr = 0x3FFE_0000;
        let queue = manager.get_mut(handle).unwrap();
        let result = queue.receive(&mut scheduler, ptr, 0, &mut cpu);
        assert!(!result);
    }

    #[test]
    fn test_queue_reset() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 10).unwrap();
        let cpu = create_test_cpu();
        let mut scheduler = super::super::scheduler::FreeRtosScheduler::new();

        // Add items
        for i in 0..5u32 {
            let ptr = 0x3FFE_0000 + (i * 4);
            cpu.memory().write_u32(ptr, i);
            let queue = manager.get_mut(handle).unwrap();
            queue.send(&mut scheduler, ptr, 0, &cpu);
        }

        assert_eq!(manager.get(handle).unwrap().get_count(), 5);

        // Reset queue
        manager.get_mut(handle).unwrap().reset();
        assert_eq!(manager.get(handle).unwrap().get_count(), 0);
        assert!(manager.get(handle).unwrap().is_empty());
    }

    #[test]
    fn test_queue_deletion() {
        let mut manager = QueueManager::new();
        let handle = manager.create(4, 10).unwrap();
        assert!(manager.get(handle).is_ok());

        manager.delete(handle).unwrap();
        assert!(manager.get(handle).is_err());
    }

    #[test]
    fn test_multiple_queues() {
        let mut manager = QueueManager::new();
        let h1 = manager.create(4, 10).unwrap();
        let h2 = manager.create(8, 5).unwrap();
        let h3 = manager.create(16, 20).unwrap();

        assert_eq!(h1, 0);
        assert_eq!(h2, 1);
        assert_eq!(h3, 2);

        assert!(manager.get(h1).is_ok());
        assert!(manager.get(h2).is_ok());
        assert!(manager.get(h3).is_ok());
    }
}
