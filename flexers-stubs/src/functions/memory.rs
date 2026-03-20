/// Memory management ROM functions
/// Simplified implementations for emulation

use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// Simple bump allocator state (starting from heap base)
static mut HEAP_OFFSET: u32 = 0;
const HEAP_BASE: u32 = 0x3FFE_8000; // RTC DRAM area used as heap
const HEAP_SIZE: u32 = 8 * 1024;    // 8 KB

/// malloc(size) - Allocate memory
/// a2 = size (input)
/// a2 = pointer (output)
pub fn exec_malloc(cpu: &mut XtensaCpu) -> Result<(), String> {
    let size = cpu.get_register(2);

    unsafe {
        // Align to 4 bytes
        let aligned_size = (size + 3) & !3;

        if HEAP_OFFSET + aligned_size > HEAP_SIZE {
            // Out of memory - return NULL
            cpu.set_register(2, 0);
        } else {
            let ptr = HEAP_BASE + HEAP_OFFSET;
            HEAP_OFFSET += aligned_size;
            cpu.set_register(2, ptr);

            // Zero the allocated memory
            for i in 0..aligned_size {
                cpu.memory().write_u8(ptr + i, 0);
            }
        }
    }

    Ok(())
}

/// free(ptr) - Free memory
/// a2 = pointer (input)
/// Note: In this simple implementation, free() is a no-op
pub fn exec_free(_cpu: &mut XtensaCpu) -> Result<(), String> {
    // Simple bump allocator doesn't support free
    // In real implementation, would need proper allocator
    Ok(())
}

/// calloc(num, size) - Allocate and zero memory
/// a2 = num (input)
/// a3 = size (input)
/// a2 = pointer (output)
pub fn exec_calloc(cpu: &mut XtensaCpu) -> Result<(), String> {
    let num = cpu.get_register(2);
    let size = cpu.get_register(3);

    // Calculate total size
    let total_size = num.wrapping_mul(size);

    // Call malloc (which already zeros memory)
    cpu.set_register(2, total_size);
    exec_malloc(cpu)?;

    Ok(())
}

/// realloc(ptr, new_size) - Resize allocation
/// a2 = pointer (input)
/// a3 = new_size (input)
/// a2 = new_pointer (output)
pub fn exec_realloc(cpu: &mut XtensaCpu) -> Result<(), String> {
    let old_ptr = cpu.get_register(2);
    let new_size = cpu.get_register(3);

    if old_ptr == 0 {
        // realloc(NULL, size) == malloc(size)
        cpu.set_register(2, new_size);
        return exec_malloc(cpu);
    }

    if new_size == 0 {
        // realloc(ptr, 0) == free(ptr)
        exec_free(cpu)?;
        cpu.set_register(2, 0);
        return Ok(());
    }

    // Allocate new memory
    cpu.set_register(2, new_size);
    exec_malloc(cpu)?;
    let new_ptr = cpu.get_register(2);

    if new_ptr != 0 {
        // Copy data from old to new
        // Note: We don't know the old size, so we copy up to new_size
        // This is a simplification - real realloc would track sizes
        for i in 0..new_size {
            let byte = cpu.memory().read_u8(old_ptr + i);
            cpu.memory().write_u8(new_ptr + i, byte);
        }
    }

    Ok(())
}

/// Reset heap (for testing)
pub fn reset_heap() {
    unsafe {
        HEAP_OFFSET = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexers_core::memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_malloc_basic() {
        reset_heap();
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // Malloc 100 bytes
        cpu.set_register(2, 100);
        exec_malloc(&mut cpu).unwrap();

        let ptr = cpu.get_register(2);
        assert_ne!(ptr, 0); // Should not be NULL
        assert!(ptr >= HEAP_BASE && ptr < HEAP_BASE + HEAP_SIZE); // Valid heap address
    }

    #[test]
    fn test_malloc_multiple() {
        reset_heap();
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // First allocation
        cpu.set_register(2, 100);
        exec_malloc(&mut cpu).unwrap();
        let ptr1 = cpu.get_register(2);

        // Second allocation
        cpu.set_register(2, 200);
        exec_malloc(&mut cpu).unwrap();
        let ptr2 = cpu.get_register(2);

        assert_ne!(ptr1, ptr2);
        assert!(ptr2 > ptr1);
    }

    #[test]
    fn test_malloc_alignment() {
        reset_heap();
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // Allocate odd size
        cpu.set_register(2, 99);
        exec_malloc(&mut cpu).unwrap();
        let ptr1 = cpu.get_register(2);

        // Next allocation should be 4-byte aligned
        cpu.set_register(2, 4);
        exec_malloc(&mut cpu).unwrap();
        let ptr2 = cpu.get_register(2);

        assert_eq!((ptr2 - ptr1) % 4, 0);
    }

    #[test]
    fn test_calloc() {
        reset_heap();
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // Calloc 10 elements of 8 bytes each
        cpu.set_register(2, 10);
        cpu.set_register(3, 8);
        exec_calloc(&mut cpu).unwrap();

        let ptr = cpu.get_register(2);
        assert_ne!(ptr, 0);

        // Verify memory is zeroed
        for i in 0..80 {
            assert_eq!(cpu.memory().read_u8(ptr + i), 0);
        }
    }

    #[test]
    fn test_free_noop() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_register(2, 0x12345678);
        exec_free(&mut cpu).unwrap(); // Should not crash
    }

    #[test]
    fn test_realloc_from_null() {
        reset_heap();
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // realloc(NULL, 100) should act like malloc(100)
        cpu.set_register(2, 0);
        cpu.set_register(3, 100);
        exec_realloc(&mut cpu).unwrap();

        let ptr = cpu.get_register(2);
        assert_ne!(ptr, 0);
    }
}

// ROM Stub Handler implementations

pub struct Malloc;
impl RomStubHandler for Malloc {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_malloc(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "malloc" }
}

pub struct Free;
impl RomStubHandler for Free {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_free(cpu).ok();
        0
    }
    fn name(&self) -> &str { "free" }
}

pub struct Calloc;
impl RomStubHandler for Calloc {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_calloc(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "calloc" }
}

pub struct Realloc;
impl RomStubHandler for Realloc {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_realloc(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "realloc" }
}
