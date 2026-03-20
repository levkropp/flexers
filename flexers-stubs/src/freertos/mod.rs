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

// Re-export key types
pub use scheduler::{FreeRtosScheduler, SCHEDULER, NUM_PRIORITIES, MAX_TASKS};
pub use task::{TaskControlBlock, TaskState, TaskHandle};
pub use semaphore::{Semaphore, SemaphoreType, SemaphoreHandle, SEMAPHORE_MANAGER};
pub use mutex::{Mutex, MutexType, MutexHandle, MUTEX_MANAGER};
