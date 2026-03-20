/// FreeRTOS task scheduler with priority-based preemptive scheduling

use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use flexers_core::cpu::XtensaCpu;
use super::task::{TaskControlBlock, TaskState, TaskHandle};

/// Maximum number of tasks
pub const MAX_TASKS: usize = 64;

/// Number of priority levels (0-31)
pub const NUM_PRIORITIES: usize = 32;

/// RTC DRAM base for task storage
const RTC_DRAM_BASE: u32 = 0x3FF80000;

/// Stack allocation cursor (starts after TCB storage)
const STACK_ALLOC_START: u32 = RTC_DRAM_BASE + 0x1000; // 4KB for TCBs

lazy_static! {
    /// Global scheduler instance
    pub static ref SCHEDULER: Arc<Mutex<FreeRtosScheduler>> =
        Arc::new(Mutex::new(FreeRtosScheduler::new()));
}

/// FreeRTOS scheduler
pub struct FreeRtosScheduler {
    /// Task list (pub for semaphore/mutex access)
    pub(crate) tasks: Vec<Option<TaskControlBlock>>,

    /// Ready queues (one per priority level)
    /// Each queue contains task handles sorted by creation order
    pub(crate) ready_queues: [Vec<TaskHandle>; NUM_PRIORITIES],

    /// Current running task handle
    pub(crate) current_task: Option<TaskHandle>,

    /// System tick counter
    tick_count: u32,

    /// Scheduler running flag
    running: bool,

    /// Next task ID
    next_task_id: TaskHandle,

    /// Stack allocation cursor
    stack_cursor: u32,

    /// Idle task handle (always runs when no other task ready)
    idle_task: Option<TaskHandle>,
}

impl FreeRtosScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            tasks: vec![None; MAX_TASKS],
            ready_queues: Default::default(),
            current_task: None,
            tick_count: 0,
            running: false,
            next_task_id: 0,
            stack_cursor: STACK_ALLOC_START,
            idle_task: None,
        }
    }

    /// Create a new task
    pub fn create_task(
        &mut self,
        entry: u32,
        name: &str,
        stack_size: usize,
        parameter: u32,
        priority: u8,
    ) -> Result<TaskHandle, String> {
        // Find free task slot
        let task_id = self.tasks.iter()
            .position(|t| t.is_none())
            .ok_or("Maximum number of tasks reached")?;

        // Allocate stack
        let stack_base = self.stack_cursor;
        self.stack_cursor += stack_size as u32;

        // Create task control block
        let mut tcb = TaskControlBlock::new(
            task_id,
            entry,
            name,
            stack_size,
            parameter,
            priority.min(31),
            stack_base,
        );

        // Initialize A0 (return address) to a special marker
        // This helps detect if task function returns (should never happen)
        tcb.registers[0] = 0xDEADBEEF;

        // Initialize A2 (first argument) to parameter
        tcb.registers[2] = parameter;

        // Add to task list
        self.tasks[task_id] = Some(tcb);

        // Add to ready queue
        let priority = priority.min(31) as usize;
        self.ready_queues[priority].push(task_id);

        self.next_task_id = self.next_task_id.max(task_id + 1);

        Ok(task_id)
    }

    /// Delete a task
    pub fn delete_task(&mut self, task_handle: TaskHandle) -> Result<(), String> {
        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid task handle")?;

        // Remove from ready queue
        let priority = task.priority() as usize;
        self.ready_queues[priority].retain(|&h| h != task_handle);

        // Mark as deleted
        task.delete();

        // If deleting current task, trigger context switch
        if self.current_task == Some(task_handle) {
            self.current_task = None;
        }

        // Actually remove the task
        self.tasks[task_handle] = None;

        Ok(())
    }

    /// Delay current task for specified ticks
    pub fn delay_task(&mut self, ticks: u32) -> Result<(), String> {
        let task_handle = self.current_task
            .ok_or("No current task")?;

        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid current task")?;

        if ticks > 0 {
            task.set_delay(ticks);

            // Remove from ready queue
            let priority = task.priority() as usize;
            self.ready_queues[priority].retain(|&h| h != task_handle);
        }

        Ok(())
    }

    /// Suspend a task
    pub fn suspend_task(&mut self, task_handle: TaskHandle) -> Result<(), String> {
        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid task handle")?;

        // Remove from ready queue
        let priority = task.priority() as usize;
        self.ready_queues[priority].retain(|&h| h != task_handle);

        task.suspend();

        Ok(())
    }

    /// Resume a suspended task
    pub fn resume_task(&mut self, task_handle: TaskHandle) -> Result<(), String> {
        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid task handle")?;

        if task.state() == TaskState::Suspended {
            task.resume();

            // Add back to ready queue
            let priority = task.priority() as usize;
            self.ready_queues[priority].push(task_handle);
        }

        Ok(())
    }

    /// Set task priority
    pub fn set_priority(&mut self, task_handle: TaskHandle, new_priority: u8) -> Result<(), String> {
        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid task handle")?;

        let old_priority = task.priority() as usize;
        let new_priority = new_priority.min(31) as usize;

        if old_priority != new_priority {
            // Remove from old ready queue
            self.ready_queues[old_priority].retain(|&h| h != task_handle);

            // Update priority
            task.set_priority(new_priority as u8);

            // Add to new ready queue (if task is ready)
            if task.state() == TaskState::Ready {
                self.ready_queues[new_priority].push(task_handle);
            }
        }

        Ok(())
    }

    /// Get task priority
    pub fn get_priority(&self, task_handle: TaskHandle) -> Result<u8, String> {
        self.tasks.get(task_handle)
            .and_then(|t| t.as_ref())
            .map(|t| t.priority())
            .ok_or_else(|| "Invalid task handle".to_string())
    }

    /// Get current task handle
    pub fn get_current_task(&self) -> Option<TaskHandle> {
        self.current_task
    }

    /// Get tick count
    pub fn get_tick_count(&self) -> u32 {
        self.tick_count
    }

    /// Start the scheduler
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop the scheduler
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Select next task to run (priority-based scheduling)
    pub fn schedule(&self) -> Option<TaskHandle> {
        // Find highest priority non-empty queue
        for priority in (0..NUM_PRIORITIES).rev() {
            if let Some(&task_handle) = self.ready_queues[priority].first() {
                return Some(task_handle);
            }
        }

        // No ready tasks
        None
    }

    /// Peek at next task without removing it from queue
    pub fn peek_next_task(&self) -> Option<TaskHandle> {
        self.schedule()
    }

    /// Get task priority by handle
    pub fn get_task_priority(&self, task_handle: TaskHandle) -> Option<u8> {
        self.tasks.get(task_handle)
            .and_then(|t| t.as_ref())
            .map(|t| t.priority())
    }

    /// Wake a blocked task (helper for queues, event groups, etc.)
    pub fn wake_task(&mut self, task_handle: TaskHandle) -> Result<(), String> {
        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid task handle")?;

        if task.state() == TaskState::Blocked {
            task.wake();
            let priority = task.priority() as usize;
            self.ready_queues[priority].push(task_handle);
        }

        Ok(())
    }

    /// Block a task for specified ticks (helper for queues, event groups, etc.)
    pub fn block_task(&mut self, task_handle: TaskHandle, timeout_ticks: u32) -> Result<(), String> {
        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid task handle")?;

        let priority = task.priority() as usize;
        task.delay(timeout_ticks);

        // Remove from ready queue
        self.ready_queues[priority].retain(|&h| h != task_handle);

        Ok(())
    }

    /// Yield current task (move to end of its priority queue)
    pub fn yield_task(&mut self) -> Result<(), String> {
        let task_handle = self.current_task
            .ok_or("No current task")?;

        let priority = self.get_priority(task_handle)? as usize;

        // Move to end of queue (round-robin within priority)
        let queue = &mut self.ready_queues[priority];
        if let Some(pos) = queue.iter().position(|&h| h == task_handle) {
            queue.remove(pos);
            queue.push(task_handle);
        }

        Ok(())
    }

    /// System tick - update delayed tasks
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Process all tasks
        for task_handle in 0..MAX_TASKS {
            if let Some(Some(task)) = self.tasks.get_mut(task_handle) {
                if task.state() == TaskState::Blocked && task.delay_ticks > 0 {
                    if task.tick() {
                        // Task woke up, add to ready queue
                        let priority = task.priority() as usize;
                        self.ready_queues[priority].push(task_handle);
                    }
                }
            }
        }
    }

    /// Save current task context
    pub fn save_context(&mut self, cpu: &XtensaCpu) -> Result<(), String> {
        let task_handle = self.current_task
            .ok_or("No current task to save")?;

        let task = self.tasks.get_mut(task_handle)
            .and_then(|t| t.as_mut())
            .ok_or("Invalid current task")?;

        // Save PC
        task.pc = cpu.pc();

        // Save A0-A15 (window 0 only for simplicity)
        for i in 0..16 {
            task.registers[i] = cpu.get_ar(i as u32);
        }

        // Save stack pointer (A1)
        task.stack_ptr = cpu.get_ar(1);

        Ok(())
    }

    /// Load task context
    pub fn load_context(&self, cpu: &mut XtensaCpu, task_handle: TaskHandle) -> Result<(), String> {
        let task = self.tasks.get(task_handle)
            .and_then(|t| t.as_ref())
            .ok_or("Invalid task to load")?;

        // Restore PC
        cpu.set_pc(task.pc);

        // Restore A0-A15
        for i in 0..16 {
            cpu.set_ar(i as u32, task.registers[i]);
        }

        Ok(())
    }

    /// Perform context switch
    pub fn switch_context(&mut self, cpu: &mut XtensaCpu) -> Result<(), String> {
        if !self.running {
            return Err("Scheduler not running".to_string());
        }

        // Save current task context (if any)
        if let Some(old_task_handle) = self.current_task {
            if let Err(e) = self.save_context(cpu) {
                // Current task might be deleted, that's okay
                if !e.contains("Invalid current task") {
                    return Err(e);
                }
            }

            // Mark old task as ready (if it was running and not blocked/suspended)
            if let Some(Some(old_task)) = self.tasks.get_mut(old_task_handle) {
                if old_task.state() == TaskState::Running {
                    old_task.set_state(TaskState::Ready);
                }
            }
        }

        // Select next task
        let next_task_handle = self.schedule()
            .ok_or("No ready tasks")?;

        // Load new task context
        self.load_context(cpu, next_task_handle)?;

        // Update current task
        self.current_task = Some(next_task_handle);

        // Mark new task as running
        if let Some(Some(new_task)) = self.tasks.get_mut(next_task_handle) {
            new_task.set_state(TaskState::Running);
        }

        Ok(())
    }

    /// Get task info (for debugging)
    pub fn get_task_info(&self, task_handle: TaskHandle) -> Option<String> {
        self.tasks.get(task_handle)
            .and_then(|t| t.as_ref())
            .map(|t| format!(
                "Task {}: {} (priority={}, state={}, pc=0x{:08X})",
                task_handle,
                t.name(),
                t.priority(),
                t.state(),
                t.pc
            ))
    }

    /// List all tasks (for debugging)
    pub fn list_tasks(&self) -> Vec<String> {
        self.tasks.iter()
            .enumerate()
            .filter_map(|(i, t)| {
                t.as_ref().map(|task| {
                    format!(
                        "[{}] {} - P:{} S:{} PC:0x{:08X}",
                        i,
                        task.name(),
                        task.priority(),
                        task.state(),
                        task.pc
                    )
                })
            })
            .collect()
    }

    /// Reset scheduler (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for FreeRtosScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexers_core::memory::Memory;

    fn create_test_cpu() -> XtensaCpu {
        let mem = Arc::new(Memory::new());
        XtensaCpu::new(mem)
    }

    #[test]
    fn test_scheduler_creation() {
        let scheduler = FreeRtosScheduler::new();
        assert!(!scheduler.is_running());
        assert_eq!(scheduler.get_tick_count(), 0);
        assert_eq!(scheduler.get_current_task(), None);
    }

    #[test]
    fn test_create_task() {
        let mut scheduler = FreeRtosScheduler::new();

        let task_handle = scheduler.create_task(
            0x40000000,
            "test_task",
            4096,
            0x12345678,
            5,
        ).unwrap();

        assert_eq!(task_handle, 0);

        // Verify task is in ready queue
        assert_eq!(scheduler.ready_queues[5].len(), 1);
        assert_eq!(scheduler.ready_queues[5][0], task_handle);
    }

    #[test]
    fn test_multiple_tasks_same_priority() {
        let mut scheduler = FreeRtosScheduler::new();

        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        let task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();
        let task3 = scheduler.create_task(0x40000200, "task3", 4096, 0, 5).unwrap();

        // All should be in same priority queue
        assert_eq!(scheduler.ready_queues[5].len(), 3);
        assert_eq!(scheduler.ready_queues[5], vec![task1, task2, task3]);
    }

    #[test]
    fn test_priority_scheduling() {
        let mut scheduler = FreeRtosScheduler::new();

        let _low = scheduler.create_task(0x40000000, "low", 4096, 0, 1).unwrap();
        let _med = scheduler.create_task(0x40000100, "med", 4096, 0, 5).unwrap();
        let high = scheduler.create_task(0x40000200, "high", 4096, 0, 10).unwrap();

        // Should schedule highest priority
        let next = scheduler.schedule().unwrap();
        assert_eq!(next, high);
    }

    #[test]
    fn test_task_deletion() {
        let mut scheduler = FreeRtosScheduler::new();

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();

        assert_eq!(scheduler.ready_queues[5].len(), 1);

        scheduler.delete_task(task).unwrap();

        // Should be removed from ready queue
        assert_eq!(scheduler.ready_queues[5].len(), 0);
        assert!(scheduler.tasks[task].is_none());
    }

    #[test]
    fn test_task_delay() {
        let mut scheduler = FreeRtosScheduler::new();

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();

        scheduler.current_task = Some(task);
        scheduler.delay_task(10).unwrap();

        // Should be removed from ready queue
        assert_eq!(scheduler.ready_queues[5].len(), 0);

        // Task should be blocked
        let tcb = scheduler.tasks[task].as_ref().unwrap();
        assert_eq!(tcb.state(), TaskState::Blocked);
        assert_eq!(tcb.delay_ticks, 10);
    }

    #[test]
    fn test_tick_wakes_task() {
        let mut scheduler = FreeRtosScheduler::new();

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();

        scheduler.current_task = Some(task);
        scheduler.delay_task(3).unwrap();

        assert_eq!(scheduler.ready_queues[5].len(), 0);

        // Tick twice
        scheduler.tick();
        scheduler.tick();

        // Still blocked
        assert_eq!(scheduler.ready_queues[5].len(), 0);

        // Tick once more - should wake
        scheduler.tick();

        // Should be back in ready queue
        assert_eq!(scheduler.ready_queues[5].len(), 1);
        assert_eq!(scheduler.ready_queues[5][0], task);
    }

    #[test]
    fn test_suspend_resume() {
        let mut scheduler = FreeRtosScheduler::new();

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();

        assert_eq!(scheduler.ready_queues[5].len(), 1);

        scheduler.suspend_task(task).unwrap();

        // Should be removed from ready queue
        assert_eq!(scheduler.ready_queues[5].len(), 0);

        let tcb = scheduler.tasks[task].as_ref().unwrap();
        assert_eq!(tcb.state(), TaskState::Suspended);

        scheduler.resume_task(task).unwrap();

        // Should be back in ready queue
        assert_eq!(scheduler.ready_queues[5].len(), 1);

        let tcb = scheduler.tasks[task].as_ref().unwrap();
        assert_eq!(tcb.state(), TaskState::Ready);
    }

    #[test]
    fn test_priority_change() {
        let mut scheduler = FreeRtosScheduler::new();

        let task = scheduler.create_task(0x40000000, "test", 4096, 0, 5).unwrap();

        assert_eq!(scheduler.ready_queues[5].len(), 1);
        assert_eq!(scheduler.ready_queues[10].len(), 0);

        scheduler.set_priority(task, 10).unwrap();

        // Should move to new queue
        assert_eq!(scheduler.ready_queues[5].len(), 0);
        assert_eq!(scheduler.ready_queues[10].len(), 1);

        assert_eq!(scheduler.get_priority(task).unwrap(), 10);
    }

    #[test]
    fn test_yield_task() {
        let mut scheduler = FreeRtosScheduler::new();

        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        let task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();
        let task3 = scheduler.create_task(0x40000200, "task3", 4096, 0, 5).unwrap();

        // Queue is [task1, task2, task3]
        assert_eq!(scheduler.ready_queues[5], vec![task1, task2, task3]);

        // Yield task1
        scheduler.current_task = Some(task1);
        scheduler.yield_task().unwrap();

        // Queue should be [task2, task3, task1]
        assert_eq!(scheduler.ready_queues[5], vec![task2, task3, task1]);
    }

    #[test]
    fn test_context_switch() {
        let mut scheduler = FreeRtosScheduler::new();
        let mut cpu = create_test_cpu();

        let task1 = scheduler.create_task(0x40000000, "task1", 4096, 0, 5).unwrap();
        let _task2 = scheduler.create_task(0x40000100, "task2", 4096, 0, 5).unwrap();

        scheduler.start();

        // Set some CPU state
        cpu.set_pc(0x40001234);
        cpu.set_ar(0, 0xAAAAAAAA);
        cpu.set_ar(1, 0xBBBBBBBB);

        // First context switch should load task1
        scheduler.switch_context(&mut cpu).unwrap();

        assert_eq!(scheduler.current_task, Some(task1));

        // PC should be task1 entry point
        assert_eq!(cpu.pc(), 0x40000000);
    }

    #[test]
    fn test_max_tasks() {
        let mut scheduler = FreeRtosScheduler::new();

        // Create MAX_TASKS tasks
        for i in 0..MAX_TASKS {
            scheduler.create_task(
                0x40000000 + (i as u32 * 0x100),
                &format!("task{}", i),
                1024,
                0,
                0,
            ).unwrap();
        }

        // Next task should fail
        let result = scheduler.create_task(0x50000000, "overflow", 1024, 0, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Maximum number of tasks reached");
    }
}
