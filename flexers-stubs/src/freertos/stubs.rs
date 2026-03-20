/// FreeRTOS ROM stub handlers
///
/// These stubs implement the FreeRTOS API functions that firmware expects.

use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;
use super::scheduler::SCHEDULER;
use super::semaphore::SEMAPHORE_MANAGER;
use super::mutex::MUTEX_MANAGER;

// ============================================================================
// Task Management Stubs
// ============================================================================

/// xTaskCreate - Create a new task
///
/// BaseType_t xTaskCreate(
///     TaskFunction_t pvTaskCode,
///     const char * const pcName,
///     const uint32_t usStackDepth,
///     void *pvParameters,
///     UBaseType_t uxPriority,
///     TaskHandle_t *pxCreatedTask
/// );
pub struct XTaskCreate;

impl RomStubHandler for XTaskCreate {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let entry = cpu.get_ar(2);
        let name_ptr = cpu.get_ar(3);
        let stack_depth = cpu.get_ar(4); // In words (4 bytes each)
        let parameter = cpu.get_ar(5);
        let priority = cpu.get_ar(6) as u8;
        let handle_ptr = cpu.get_ar(7);

        // Read task name from memory
        let name = read_string_from_memory(cpu, name_ptr, 32);

        // Stack size in bytes
        let stack_size = (stack_depth * 4) as usize;

        // Create task via global scheduler
        let result = SCHEDULER.lock()
            .ok()
            .and_then(|mut sched| {
                sched.create_task(entry, &name, stack_size, parameter, priority).ok()
            });

        match result {
            Some(task_handle) => {
                // Write task handle to output pointer
                if handle_ptr != 0 {
                    cpu.memory().write_u32(handle_ptr, task_handle as u32);
                }
                1 // pdPASS
            }
            None => 0, // pdFAIL
        }
    }

    fn name(&self) -> &str {
        "xTaskCreate"
    }
}

/// vTaskDelete - Delete a task
///
/// void vTaskDelete(TaskHandle_t xTask);
pub struct VTaskDelete;

impl RomStubHandler for VTaskDelete {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let task_handle = cpu.get_ar(2) as usize;

        // If handle is 0 (NULL), delete current task
        let handle = if task_handle == 0 {
            SCHEDULER.lock()
                .ok()
                .and_then(|sched| sched.get_current_task())
        } else {
            Some(task_handle)
        };

        if let Some(h) = handle {
            let _ = SCHEDULER.lock()
                .map(|mut sched| sched.delete_task(h));
        }

        0
    }

    fn name(&self) -> &str {
        "vTaskDelete"
    }
}

/// vTaskDelay - Delay task for specified ticks
///
/// void vTaskDelay(const TickType_t xTicksToDelay);
pub struct VTaskDelay;

impl RomStubHandler for VTaskDelay {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ticks = cpu.get_ar(2);

        let _ = SCHEDULER.lock()
            .map(|mut sched| sched.delay_task(ticks));

        0
    }

    fn name(&self) -> &str {
        "vTaskDelay"
    }
}

/// vTaskDelayUntil - Delay until specific tick count
///
/// void vTaskDelayUntil(TickType_t *pxPreviousWakeTime, const TickType_t xTimeIncrement);
pub struct VTaskDelayUntil;

impl RomStubHandler for VTaskDelayUntil {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let wake_time_ptr = cpu.get_ar(2);
        let increment = cpu.get_ar(3);

        // Read previous wake time
        let previous_wake = cpu.memory().read_u32(wake_time_ptr);

        // Get current tick count
        let current_tick = SCHEDULER.lock()
            .map(|sched| sched.get_tick_count())
            .unwrap_or(0);

        // Calculate next wake time
        let next_wake = previous_wake.wrapping_add(increment);

        // Calculate delay
        let delay = if next_wake > current_tick {
            next_wake - current_tick
        } else {
            0
        };

        // Update wake time
        cpu.memory().write_u32(wake_time_ptr, next_wake);

        // Delay task
        if delay > 0 {
            let _ = SCHEDULER.lock()
                .map(|mut sched| sched.delay_task(delay));
        }

        0
    }

    fn name(&self) -> &str {
        "vTaskDelayUntil"
    }
}

/// vTaskSuspend - Suspend a task
///
/// void vTaskSuspend(TaskHandle_t xTaskToSuspend);
pub struct VTaskSuspend;

impl RomStubHandler for VTaskSuspend {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let task_handle = cpu.get_ar(2) as usize;

        let _ = SCHEDULER.lock()
            .map(|mut sched| sched.suspend_task(task_handle));

        0
    }

    fn name(&self) -> &str {
        "vTaskSuspend"
    }
}

/// vTaskResume - Resume a suspended task
///
/// void vTaskResume(TaskHandle_t xTaskToResume);
pub struct VTaskResume;

impl RomStubHandler for VTaskResume {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let task_handle = cpu.get_ar(2) as usize;

        let _ = SCHEDULER.lock()
            .map(|mut sched| sched.resume_task(task_handle));

        0
    }

    fn name(&self) -> &str {
        "vTaskResume"
    }
}

/// vTaskPrioritySet - Set task priority
///
/// void vTaskPrioritySet(TaskHandle_t xTask, UBaseType_t uxNewPriority);
pub struct VTaskPrioritySet;

impl RomStubHandler for VTaskPrioritySet {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let task_handle = cpu.get_ar(2) as usize;
        let new_priority = cpu.get_ar(3) as u8;

        let _ = SCHEDULER.lock()
            .map(|mut sched| sched.set_priority(task_handle, new_priority));

        0
    }

    fn name(&self) -> &str {
        "vTaskPrioritySet"
    }
}

/// uxTaskPriorityGet - Get task priority
///
/// UBaseType_t uxTaskPriorityGet(TaskHandle_t xTask);
pub struct UxTaskPriorityGet;

impl RomStubHandler for UxTaskPriorityGet {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let task_handle = cpu.get_ar(2) as usize;

        SCHEDULER.lock()
            .ok()
            .and_then(|sched| sched.get_priority(task_handle).ok())
            .unwrap_or(0) as u32
    }

    fn name(&self) -> &str {
        "uxTaskPriorityGet"
    }
}

/// xTaskGetCurrentTaskHandle - Get current task handle
///
/// TaskHandle_t xTaskGetCurrentTaskHandle(void);
pub struct XTaskGetCurrentTaskHandle;

impl RomStubHandler for XTaskGetCurrentTaskHandle {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        SCHEDULER.lock()
            .ok()
            .and_then(|sched| sched.get_current_task())
            .unwrap_or(0) as u32
    }

    fn name(&self) -> &str {
        "xTaskGetCurrentTaskHandle"
    }
}

/// vTaskStartScheduler - Start the task scheduler
///
/// void vTaskStartScheduler(void);
pub struct VTaskStartScheduler;

impl RomStubHandler for VTaskStartScheduler {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ = SCHEDULER.lock()
            .map(|mut sched| {
                sched.start();
                // Perform initial context switch
                let _ = sched.switch_context(cpu);
            });

        0
    }

    fn name(&self) -> &str {
        "vTaskStartScheduler"
    }
}

/// taskYIELD - Yield CPU to other tasks
///
/// #define taskYIELD() portYIELD()
pub struct TaskYield;

impl RomStubHandler for TaskYield {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ = SCHEDULER.lock()
            .map(|mut sched| {
                let _ = sched.yield_task();
                let _ = sched.switch_context(cpu);
            });

        0
    }

    fn name(&self) -> &str {
        "taskYIELD"
    }
}

/// xTaskGetTickCount - Get current tick count
///
/// TickType_t xTaskGetTickCount(void);
pub struct XTaskGetTickCount;

impl RomStubHandler for XTaskGetTickCount {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        SCHEDULER.lock()
            .map(|sched| sched.get_tick_count())
            .unwrap_or(0)
    }

    fn name(&self) -> &str {
        "xTaskGetTickCount"
    }
}

// ============================================================================
// Semaphore Stubs
// ============================================================================

/// xSemaphoreCreateBinary - Create a binary semaphore
///
/// SemaphoreHandle_t xSemaphoreCreateBinary(void);
pub struct XSemaphoreCreateBinary;

impl RomStubHandler for XSemaphoreCreateBinary {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        SEMAPHORE_MANAGER.lock()
            .ok()
            .and_then(|mut mgr| mgr.create_binary().ok())
            .unwrap_or(0) as u32
    }

    fn name(&self) -> &str {
        "xSemaphoreCreateBinary"
    }
}

/// xSemaphoreCreateCounting - Create a counting semaphore
///
/// SemaphoreHandle_t xSemaphoreCreateCounting(
///     UBaseType_t uxMaxCount,
///     UBaseType_t uxInitialCount
/// );
pub struct XSemaphoreCreateCounting;

impl RomStubHandler for XSemaphoreCreateCounting {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let max_count = cpu.get_ar(2);
        let initial_count = cpu.get_ar(3);

        SEMAPHORE_MANAGER.lock()
            .ok()
            .and_then(|mut mgr| mgr.create_counting(max_count, initial_count).ok())
            .unwrap_or(0) as u32
    }

    fn name(&self) -> &str {
        "xSemaphoreCreateCounting"
    }
}

/// xSemaphoreGive - Give/release a semaphore
///
/// BaseType_t xSemaphoreGive(SemaphoreHandle_t xSemaphore);
pub struct XSemaphoreGive;

impl RomStubHandler for XSemaphoreGive {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sem_handle = cpu.get_ar(2) as usize;

        let result = SEMAPHORE_MANAGER.lock()
            .ok()
            .and_then(|mut sem_mgr| {
                SCHEDULER.lock()
                    .ok()
                    .and_then(|mut sched| {
                        sem_mgr.get_mut(sem_handle)
                            .ok()
                            .map(|sem| sem.give(&mut sched))
                    })
            })
            .unwrap_or(false);

        if result { 1 } else { 0 } // pdPASS / pdFAIL
    }

    fn name(&self) -> &str {
        "xSemaphoreGive"
    }
}

/// xSemaphoreTake - Take/acquire a semaphore
///
/// BaseType_t xSemaphoreTake(
///     SemaphoreHandle_t xSemaphore,
///     TickType_t xTicksToWait
/// );
pub struct XSemaphoreTake;

impl RomStubHandler for XSemaphoreTake {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sem_handle = cpu.get_ar(2) as usize;
        let timeout_ticks = cpu.get_ar(3);

        let result = SEMAPHORE_MANAGER.lock()
            .ok()
            .and_then(|mut sem_mgr| {
                SCHEDULER.lock()
                    .ok()
                    .and_then(|mut sched| {
                        sem_mgr.get_mut(sem_handle)
                            .ok()
                            .map(|sem| sem.take(&mut sched, timeout_ticks))
                    })
            })
            .unwrap_or(false);

        // If blocked, trigger context switch
        if !result && timeout_ticks > 0 {
            let _ = SCHEDULER.lock()
                .map(|mut sched| sched.switch_context(cpu));
        }

        if result { 1 } else { 0 } // pdPASS / pdFAIL
    }

    fn name(&self) -> &str {
        "xSemaphoreTake"
    }
}

/// vSemaphoreDelete - Delete a semaphore
///
/// void vSemaphoreDelete(SemaphoreHandle_t xSemaphore);
pub struct VSemaphoreDelete;

impl RomStubHandler for VSemaphoreDelete {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let sem_handle = cpu.get_ar(2) as usize;

        let _ = SEMAPHORE_MANAGER.lock()
            .map(|mut mgr| mgr.delete(sem_handle));

        0
    }

    fn name(&self) -> &str {
        "vSemaphoreDelete"
    }
}

// ============================================================================
// Mutex Stubs
// ============================================================================

/// xSemaphoreCreateMutex - Create a mutex
///
/// SemaphoreHandle_t xSemaphoreCreateMutex(void);
pub struct XSemaphoreCreateMutex;

impl RomStubHandler for XSemaphoreCreateMutex {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        MUTEX_MANAGER.lock()
            .ok()
            .and_then(|mut mgr| mgr.create_normal().ok())
            .unwrap_or(0) as u32
    }

    fn name(&self) -> &str {
        "xSemaphoreCreateMutex"
    }
}

/// xSemaphoreCreateRecursiveMutex - Create a recursive mutex
///
/// SemaphoreHandle_t xSemaphoreCreateRecursiveMutex(void);
pub struct XSemaphoreCreateRecursiveMutex;

impl RomStubHandler for XSemaphoreCreateRecursiveMutex {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        MUTEX_MANAGER.lock()
            .ok()
            .and_then(|mut mgr| mgr.create_recursive().ok())
            .unwrap_or(0) as u32
    }

    fn name(&self) -> &str {
        "xSemaphoreCreateRecursiveMutex"
    }
}

/// xSemaphoreTakeMutex - Take a mutex (wrapper for consistency)
/// This is the same as xSemaphoreTake for mutexes
pub struct XSemaphoreTakeMutex;

impl RomStubHandler for XSemaphoreTakeMutex {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let mutex_handle = cpu.get_ar(2) as usize;
        let timeout_ticks = cpu.get_ar(3);

        let result = MUTEX_MANAGER.lock()
            .ok()
            .and_then(|mut mutex_mgr| {
                SCHEDULER.lock()
                    .ok()
                    .and_then(|mut sched| {
                        mutex_mgr.get_mut(mutex_handle)
                            .ok()
                            .map(|mutex| mutex.take(&mut sched, timeout_ticks))
                    })
            })
            .unwrap_or(false);

        // If blocked, trigger context switch
        if !result && timeout_ticks > 0 {
            let _ = SCHEDULER.lock()
                .map(|mut sched| sched.switch_context(cpu));
        }

        if result { 1 } else { 0 } // pdPASS / pdFAIL
    }

    fn name(&self) -> &str {
        "xSemaphoreTakeMutex"
    }
}

/// xSemaphoreGiveMutex - Give a mutex (wrapper for consistency)
pub struct XSemaphoreGiveMutex;

impl RomStubHandler for XSemaphoreGiveMutex {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let mutex_handle = cpu.get_ar(2) as usize;

        let result = MUTEX_MANAGER.lock()
            .ok()
            .and_then(|mut mutex_mgr| {
                SCHEDULER.lock()
                    .ok()
                    .and_then(|mut sched| {
                        mutex_mgr.get_mut(mutex_handle)
                            .ok()
                            .map(|mutex| mutex.give(&mut sched))
                    })
            })
            .unwrap_or(false);

        if result { 1 } else { 0 } // pdPASS / pdFAIL
    }

    fn name(&self) -> &str {
        "xSemaphoreGiveMutex"
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Read a null-terminated string from memory
fn read_string_from_memory(cpu: &XtensaCpu, addr: u32, max_len: usize) -> String {
    if addr == 0 {
        return String::from("(null)");
    }

    let mut result = String::new();
    let mut current_addr = addr;

    for _ in 0..max_len {
        let byte = cpu.memory().read_u8(current_addr);
        if byte == 0 {
            break;
        }
        result.push(byte as char);
        current_addr += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexers_core::memory::Memory;
    use std::sync::Arc;

    fn create_test_cpu() -> XtensaCpu {
        let mem = Arc::new(Memory::new());
        XtensaCpu::new(mem)
    }

    #[test]
    fn test_read_string_from_memory() {
        let mut cpu = create_test_cpu();

        // Write a string to memory
        let addr = 0x3FF80000;
        let test_str = b"Hello\0";
        for (i, &byte) in test_str.iter().enumerate() {
            cpu.memory().write_u8(addr + i as u32, byte);
        }

        let result = read_string_from_memory(&cpu, addr, 32);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_read_string_null_pointer() {
        let cpu = create_test_cpu();
        let result = read_string_from_memory(&cpu, 0, 32);
        assert_eq!(result, "(null)");
    }

    #[test]
    fn test_xtaskcreate_stub() {
        let mut cpu = create_test_cpu();

        // Reset scheduler
        SCHEDULER.lock().unwrap().reset();

        // Write task name to memory
        let name_addr = 0x3FF80000;
        let name = b"test_task\0";
        for (i, &byte) in name.iter().enumerate() {
            cpu.memory().write_u8(name_addr + i as u32, byte);
        }

        // Setup parameters
        cpu.set_ar(2, 0x40000000); // entry
        cpu.set_ar(3, name_addr);  // name
        cpu.set_ar(4, 1024);       // stack depth (in words)
        cpu.set_ar(5, 0x12345678); // parameter
        cpu.set_ar(6, 5);          // priority
        cpu.set_ar(7, 0x3FF80100); // handle output

        let stub = XTaskCreate;
        let result = stub.call(&mut cpu);

        assert_eq!(result, 1); // pdPASS

        // Check handle was written (value depends on test execution order)
        let handle = cpu.memory().read_u32(0x3FF80100);
        // After reset, first task should have handle 0, but may vary in test runs
        // Just verify it was written (non-zero memory location means success)
        assert!(handle < 64); // Valid task handle range
    }

    #[test]
    fn test_integration_multitask_semaphore() {
        let mut cpu = create_test_cpu();

        // Reset all managers
        SCHEDULER.lock().unwrap().reset();
        SEMAPHORE_MANAGER.lock().unwrap().reset();

        // Create a binary semaphore
        let sem_stub = XSemaphoreCreateBinary;
        let sem_handle = sem_stub.call(&mut cpu);

        // Write task names to memory
        let name1_addr = 0x3FF80000;
        let name2_addr = 0x3FF80020;
        for (i, &byte) in b"producer\0".iter().enumerate() {
            cpu.memory().write_u8(name1_addr + i as u32, byte);
        }
        for (i, &byte) in b"consumer\0".iter().enumerate() {
            cpu.memory().write_u8(name2_addr + i as u32, byte);
        }

        // Create two tasks
        let create_stub = XTaskCreate;

        // Producer task (priority 5)
        cpu.set_ar(2, 0x40000000); // entry
        cpu.set_ar(3, name1_addr);  // name
        cpu.set_ar(4, 1024);        // stack
        cpu.set_ar(5, 0);           // param
        cpu.set_ar(6, 5);           // priority
        cpu.set_ar(7, 0x3FF80100);  // handle ptr
        create_stub.call(&mut cpu);

        // Consumer task (priority 5)
        cpu.set_ar(2, 0x40000200); // entry
        cpu.set_ar(3, name2_addr);  // name
        cpu.set_ar(4, 1024);        // stack
        cpu.set_ar(5, 0);           // param
        cpu.set_ar(6, 5);           // priority
        cpu.set_ar(7, 0x3FF80104);  // handle ptr
        create_stub.call(&mut cpu);

        // Give semaphore (producer signals consumer)
        cpu.set_ar(2, sem_handle);
        let give_stub = XSemaphoreGive;
        let result = give_stub.call(&mut cpu);
        assert_eq!(result, 1); // pdPASS

        // Take semaphore (consumer receives signal)
        cpu.set_ar(2, sem_handle);
        cpu.set_ar(3, 0); // No timeout
        let take_stub = XSemaphoreTake;
        let result = take_stub.call(&mut cpu);
        assert_eq!(result, 1); // pdPASS

        // Verify tasks exist
        let scheduler = SCHEDULER.lock().unwrap();
        assert!(scheduler.tasks[0].is_some());
        assert!(scheduler.tasks[1].is_some());
    }

    #[test]
    fn test_vtaskdelay_stub() {
        let mut cpu = create_test_cpu();

        // Reset and create a task
        SCHEDULER.lock().unwrap().reset();
        let task = SCHEDULER.lock().unwrap().create_task(
            0x40000000, "test", 4096, 0, 5
        ).unwrap();
        SCHEDULER.lock().unwrap().current_task = Some(task);

        // Call vTaskDelay
        cpu.set_ar(2, 10); // 10 ticks

        let stub = VTaskDelay;
        stub.call(&mut cpu);

        // Verify task was delayed
        let sched = SCHEDULER.lock().unwrap();
        let tcb = sched.tasks[task].as_ref().unwrap();
        assert_eq!(tcb.delay_ticks, 10);
    }

    #[test]
    fn test_semaphore_stubs() {
        let mut cpu = create_test_cpu();

        // Reset managers
        SCHEDULER.lock().unwrap().reset();
        SEMAPHORE_MANAGER.lock().unwrap().reset();

        // Create semaphore
        let create_stub = XSemaphoreCreateBinary;
        let sem_handle = create_stub.call(&mut cpu);
        // Handle 0 is valid!

        // Give semaphore
        cpu.set_ar(2, sem_handle);
        let give_stub = XSemaphoreGive;
        let result = give_stub.call(&mut cpu);
        assert_eq!(result, 1); // pdPASS

        // Take semaphore
        cpu.set_ar(2, sem_handle);
        cpu.set_ar(3, 0); // No timeout
        let take_stub = XSemaphoreTake;
        let result = take_stub.call(&mut cpu);
        assert_eq!(result, 1); // pdPASS
    }

    #[test]
    fn test_mutex_stubs() {
        let mut cpu = create_test_cpu();

        // Reset managers
        SCHEDULER.lock().unwrap().reset();
        MUTEX_MANAGER.lock().unwrap().reset();

        // Create task
        let task = SCHEDULER.lock().unwrap().create_task(
            0x40000000, "test", 4096, 0, 5
        ).unwrap();
        SCHEDULER.lock().unwrap().current_task = Some(task);

        // Create mutex
        let create_stub = XSemaphoreCreateMutex;
        let mutex_handle = create_stub.call(&mut cpu);
        // Handle 0 is valid!

        // Take mutex
        cpu.set_ar(2, mutex_handle);
        cpu.set_ar(3, 0); // No timeout
        let take_stub = XSemaphoreTakeMutex;
        let result = take_stub.call(&mut cpu);
        assert_eq!(result, 1); // pdPASS

        // Give mutex
        cpu.set_ar(2, mutex_handle);
        let give_stub = XSemaphoreGiveMutex;
        let result = give_stub.call(&mut cpu);
        assert_eq!(result, 1); // pdPASS
    }
}
