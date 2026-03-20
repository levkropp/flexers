/// FreeRTOS task scheduling and synchronization primitives
///
/// This module implements the core FreeRTOS functionality needed for multitasking:
/// - Task scheduler with 32 priority levels
/// - Context switching
/// - Semaphores (binary and counting)
/// - Mutexes with priority inheritance
/// - ROM stub interface

pub mod scheduler;
pub mod task;
pub mod semaphore;
pub mod mutex;
pub mod stubs;
pub mod timer_tick;
pub mod queue;
pub mod event_group;
pub mod sw_timer;

// Re-export key types
pub use scheduler::{FreeRtosScheduler, SCHEDULER, NUM_PRIORITIES, MAX_TASKS};
pub use task::{TaskControlBlock, TaskState, TaskHandle};
pub use semaphore::{Semaphore, SemaphoreType, SemaphoreHandle, SEMAPHORE_MANAGER};
pub use mutex::{Mutex, MutexType, MutexHandle, MUTEX_MANAGER};
pub use timer_tick::{SystemTickTimer, SYSTEM_TICK_TIMER, DEFAULT_TICK_RATE_HZ, CPU_CLOCK_HZ};
pub use queue::{Queue, QueueHandle, QUEUE_MANAGER, MAX_QUEUES};
pub use event_group::{EventGroup, EventGroupHandle, EVENT_GROUP_MANAGER, MAX_EVENT_GROUPS};
pub use sw_timer::{SoftwareTimer, TimerHandle, SW_TIMER_MANAGER, MAX_SW_TIMERS};
