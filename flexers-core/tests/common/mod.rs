use flexers_core::{cpu::XtensaCpu, memory::Memory};
use std::sync::Arc;

/// Create a minimal test firmware in memory
/// Returns: (memory, entry_point)
pub fn create_minimal_firmware() -> (Arc<Memory>, u32) {
    let mem = Arc::new(Memory::new());
    let entry = 0x40000400;

    // Write a simple program:
    // 1. MOVI a2, 42    (load immediate)
    // 2. ADDI a2, a2, 1 (increment)
    // 3. Infinite loop (BEQZ a0, -4)

    // MOVI a2, 42
    // Format: op0=2 (LSAI), t=2, imm12=42
    // MOVI is in the LSAI space with op1=10 (0xA)
    // bits [0:3] = 2 (op0)
    // bits [4:7] = 2 (t=a2)
    // bits [8:19] = 42 (imm12)
    // bits [8:11] = 42 & 0xF = 10 = 0xA
    // bits [12:15] = (42 >> 4) & 0xF = 2
    // bits [16:19] = (42 >> 8) & 0xF = 0
    // But wait, for MOVI, the encoding is different...
    // Let me use ADDI instead for simplicity

    // Actually, let's write a simpler program:
    // 1. NOP
    // 2. NOP
    // 3. NOP (or loop forever)

    // For now, just write NOPs or a simple instruction sequence
    // We'll test that we can load code and PC advances

    // Write some simple ADD instructions for testing
    // ADD a2, a1, a1 (a2 = a1 + a1, with a1=0, so a2=0)
    // Format: RRR with op0=0, op1=0, op2=0 (ADD)
    // t=a1, s=a1, r=a2
    let add_insn = 0x001120u32; // op0=0, t=1, s=1, r=2, op1=0, op2=0
    mem.write_u8(entry, (add_insn & 0xFF) as u8);
    mem.write_u8(entry + 1, ((add_insn >> 8) & 0xFF) as u8);
    mem.write_u8(entry + 2, ((add_insn >> 16) & 0xFF) as u8);

    // Write another ADD: a3 = a2 + a2
    let add_insn2 = 0x002230u32; // op0=0, t=2, s=2, r=3
    mem.write_u8(entry + 3, (add_insn2 & 0xFF) as u8);
    mem.write_u8(entry + 4, ((add_insn2 >> 8) & 0xFF) as u8);
    mem.write_u8(entry + 5, ((add_insn2 >> 16) & 0xFF) as u8);

    // Write a third instruction: a4 = a3 + a3
    let add_insn3 = 0x003340u32; // op0=0, t=3, s=3, r=4
    mem.write_u8(entry + 6, (add_insn3 & 0xFF) as u8);
    mem.write_u8(entry + 7, ((add_insn3 >> 8) & 0xFF) as u8);
    mem.write_u8(entry + 8, ((add_insn3 >> 16) & 0xFF) as u8);

    (mem, entry)
}

/// Run CPU until PC reaches target or max cycles
pub fn run_until_pc(cpu: &mut XtensaCpu, target_pc: u32, max_cycles: usize) -> usize {
    let mut total = 0;

    while cpu.pc() != target_pc && total < max_cycles {
        match flexers_core::run_batch(cpu, 1) {
            Ok(n) => total += n,
            Err(_) => break,
        }
    }

    total
}
