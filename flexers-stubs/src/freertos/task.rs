/// Task Control Block and task state management

use std::fmt;

/// Task handle (index into task array)
pub type TaskHandle = usize;

/// Task Control Block - contains all state for a single task
#[derive(Debug, Clone)]
pub struct TaskControlBlock {
    /// Task name (for debugging)
    pub name: String,

    /// Task state
    pub state: TaskState,

    /// Priority (0 = lowest, 31 = highest)
    pub priority: u8,

    /// Saved program counter
    pub pc: u32,

    /// Saved registers (A0-A15, window 0)
    pub registers: [u32; 16],

    /// Stack pointer (points to task stack)
    pub stack_ptr: u32,

    /// Stack size
    pub stack_size: usize,

    /// Task entry point
    pub entry: u32,

    /// Task parameter (void* arg)
    pub parameter: u32,

    /// Delay ticks (for vTaskDelay)
    pub delay_ticks: u32,

    /// Task creation order (for debugging)
    pub id: TaskHandle,

    /// Stack base address (for bounds checking)
    pub stack_base: u32,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is blocked (waiting on semaphore/mutex/delay)
    Blocked,
    /// Task is suspended
    Suspended,
    /// Task has been deleted (pending cleanup)
    Deleted,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskState::Ready => write!(f, "Ready"),
            TaskState::Running => write!(f, "Running"),
            TaskState::Blocked => write!(f, "Blocked"),
            TaskState::Suspended => write!(f, "Suspended"),
            TaskState::Deleted => write!(f, "Deleted"),
        }
    }
}

impl TaskControlBlock {
    /// Create a new task control block
    pub fn new(
        id: TaskHandle,
        entry: u32,
        name: &str,
        stack_size: usize,
        parameter: u32,
        priority: u8,
        stack_base: u32,
    ) -> Self {
        // Initialize stack pointer to top of stack (stack grows down)
        let stack_ptr = stack_base + stack_size as u32;

        Self {
            name: name.to_string(),
            state: TaskState::Ready,
            priority: priority.min(31), // Cap at max priority
            pc: entry,
            registers: [0; 16],
            stack_ptr,
            stack_size,
            entry,
            parameter,
            delay_ticks: 0,
            id,
            stack_base,
        }
    }

    /// Check if task is in a runnable state
    pub fn is_runnable(&self) -> bool {
        matches!(self.state, TaskState::Ready | TaskState::Running)
    }

    /// Get task name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get task priority
    pub fn priority(&self) -> u8 {
        self.priority
    }

    /// Set task priority
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority.min(31);
    }

    /// Get task state
    pub fn state(&self) -> TaskState {
        self.state
    }

    /// Set task state
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    /// Suspend task
    pub fn suspend(&mut self) {
        if self.state != TaskState::Deleted {
            self.state = TaskState::Suspended;
        }
    }

    /// Resume task
    pub fn resume(&mut self) {
        if self.state == TaskState::Suspended {
            self.state = TaskState::Ready;
        }
    }

    /// Delete task
    pub fn delete(&mut self) {
        self.state = TaskState::Deleted;
    }

    /// Set delay ticks
    pub fn set_delay(&mut self, ticks: u32) {
        self.delay_ticks = ticks;
        if ticks > 0 {
            self.state = TaskState::Blocked;
        }
    }

    /// Decrement delay ticks (returns true if task should wake)
    pub fn tick(&mut self) -> bool {
        if self.delay_ticks > 0 {
            self.delay_ticks -= 1;
            if self.delay_ticks == 0 {
                self.state = TaskState::Ready;
                return true;
            }
        }
        false
    }

    /// Check if stack pointer is within bounds
    pub fn check_stack_bounds(&self) -> bool {
        self.stack_ptr >= self.stack_base &&
        self.stack_ptr <= self.stack_base + self.stack_size as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = TaskControlBlock::new(
            0,
            0x40000000,
            "test_task",
            4096,
            0x12345678,
            5,
            0x3FF80000,
        );

        assert_eq!(task.name, "test_task");
        assert_eq!(task.state, TaskState::Ready);
        assert_eq!(task.priority, 5);
        assert_eq!(task.entry, 0x40000000);
        assert_eq!(task.parameter, 0x12345678);
        assert_eq!(task.stack_size, 4096);
        assert_eq!(task.stack_base, 0x3FF80000);
        assert_eq!(task.stack_ptr, 0x3FF80000 + 4096);
    }

    #[test]
    fn test_task_priority_cap() {
        let task = TaskControlBlock::new(
            0,
            0x40000000,
            "test_task",
            4096,
            0,
            100, // Above max
            0x3FF80000,
        );

        assert_eq!(task.priority, 31); // Capped
    }

    #[test]
    fn test_task_states() {
        let mut task = TaskControlBlock::new(
            0,
            0x40000000,
            "test_task",
            4096,
            0,
            5,
            0x3FF80000,
        );

        assert!(task.is_runnable());
        assert_eq!(task.state(), TaskState::Ready);

        task.suspend();
        assert_eq!(task.state(), TaskState::Suspended);
        assert!(!task.is_runnable());

        task.resume();
        assert_eq!(task.state(), TaskState::Ready);
        assert!(task.is_runnable());

        task.delete();
        assert_eq!(task.state(), TaskState::Deleted);
        assert!(!task.is_runnable());
    }

    #[test]
    fn test_task_delay() {
        let mut task = TaskControlBlock::new(
            0,
            0x40000000,
            "test_task",
            4096,
            0,
            5,
            0x3FF80000,
        );

        task.set_delay(5);
        assert_eq!(task.delay_ticks, 5);
        assert_eq!(task.state(), TaskState::Blocked);

        // Tick down
        assert!(!task.tick()); // Still blocked
        assert_eq!(task.delay_ticks, 4);

        for _ in 0..3 {
            task.tick();
        }

        assert_eq!(task.delay_ticks, 1);
        assert!(task.tick()); // Last tick, should wake and return true

        assert_eq!(task.delay_ticks, 0);
        assert_eq!(task.state(), TaskState::Ready);
    }

    #[test]
    fn test_stack_bounds() {
        let task = TaskControlBlock::new(
            0,
            0x40000000,
            "test_task",
            4096,
            0,
            5,
            0x3FF80000,
        );

        assert!(task.check_stack_bounds());

        let mut task_bad = task.clone();
        task_bad.stack_ptr = 0x3FF70000; // Below base
        assert!(!task_bad.check_stack_bounds());

        let mut task_bad2 = task.clone();
        task_bad2.stack_ptr = 0x3FF90000; // Above top
        assert!(!task_bad2.check_stack_bounds());
    }
}
