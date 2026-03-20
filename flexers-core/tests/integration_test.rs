use flexers_core::{cpu::XtensaCpu, memory::Memory, run_batch};
use std::sync::Arc;

mod common;

#[test]
fn test_minimal_program_execution() {
    let (mem, entry) = common::create_minimal_firmware();
    let mut cpu = XtensaCpu::new(mem.clone());
    cpu.set_pc(entry);

    // Run for 3 cycles (3 ADD instructions)
    let executed = run_batch(&mut cpu, 3).unwrap();
    assert_eq!(executed, 3);

    // PC should have advanced by 9 bytes (3 instructions * 3 bytes each)
    assert_eq!(cpu.pc(), entry + 9);
}

#[test]
fn test_cycle_counting() {
    let (mem, entry) = common::create_minimal_firmware();
    let mut cpu = XtensaCpu::new(mem.clone());
    cpu.set_pc(entry);

    let initial_cycles = cpu.cycle_count();
    run_batch(&mut cpu, 10).unwrap();

    assert_eq!(cpu.cycle_count(), initial_cycles + 10);
}

#[test]
fn test_memory_operations() {
    let mem = Arc::new(Memory::new());
    let base = 0x3FFA_0000; // SRAM

    // Write test data
    mem.write_u32(base, 0xDEADBEEF);
    mem.write_u32(base + 4, 0xCAFEBABE);

    // Verify reads
    assert_eq!(mem.read_u32(base), 0xDEADBEEF);
    assert_eq!(mem.read_u32(base + 4), 0xCAFEBABE);
}

#[test]
fn test_register_operations() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem);

    // Set some registers
    cpu.set_register(1, 100);
    cpu.set_register(2, 200);

    // Verify reads
    assert_eq!(cpu.get_register(1), 100);
    assert_eq!(cpu.get_register(2), 200);

    // Verify a0 can be read/written (Xtensa doesn't have a hardwired zero register)
    cpu.set_register(0, 999);
    assert_eq!(cpu.get_register(0), 999);
}
