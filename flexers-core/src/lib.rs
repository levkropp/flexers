pub mod cpu;
pub mod memory;
pub mod decode;
pub mod exec;

use cpu::XtensaCpu;
use decode::fetch;
use exec::{execute, StopReason, ExecError};

/// Run CPU for a batch of cycles
/// Returns number of cycles actually executed
pub fn run_batch(cpu: &mut XtensaCpu, cycles: usize) -> Result<usize, ExecError> {
    let mut executed = 0;

    while executed < cycles && cpu.is_running() {
        // Fetch instruction
        let insn = fetch(cpu.memory(), cpu.pc())
            .map_err(|_| ExecError::MemoryFault(cpu.pc()))?;

        // Execute instruction
        match execute(cpu, insn)? {
            StopReason::Continue => {
                // Normal instruction - advance PC by instruction length
                cpu.set_pc(cpu.pc() + insn.len as u32);
                cpu.inc_cycles(1);
                executed += 1;
            }
            StopReason::PcWritten => {
                // Branch/jump - PC already updated
                cpu.inc_cycles(1);
                executed += 1;
            }
            StopReason::Halted => {
                // CPU halted
                break;
            }
        }
    }

    Ok(executed)
}

/// Run CPU until it halts or reaches max_cycles
pub fn run_until_halt(cpu: &mut XtensaCpu, max_cycles: usize) -> Result<usize, ExecError> {
    let mut total = 0;

    while total < max_cycles && cpu.is_running() {
        let batch_size = (max_cycles - total).min(1000);
        let executed = run_batch(cpu, batch_size)?;
        total += executed;

        if executed < batch_size {
            // CPU halted
            break;
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_run_batch_basic() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem.clone());

        // Write a simple program: NOP followed by infinite loop
        // NOP: 0x0020F0 (wide encoding)
        cpu.memory().write_u8(0x40000400, 0xF0);
        cpu.memory().write_u8(0x40000401, 0x20);
        cpu.memory().write_u8(0x40000402, 0x00);

        // Branch back to self: BEQZ a0, -4
        // This will loop forever since a0 is 0
        cpu.memory().write_u8(0x40000403, 0x16);
        cpu.memory().write_u8(0x40000404, 0x00);
        cpu.memory().write_u8(0x40000405, 0xFC); // offset = -4

        // Run for 10 cycles
        let result = run_batch(&mut cpu, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cycle_counting() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem.clone());

        // Write NOPs
        for i in 0..10 {
            let addr = 0x40000400 + i * 3;
            cpu.memory().write_u8(addr, 0xF0);
            cpu.memory().write_u8(addr + 1, 0x20);
            cpu.memory().write_u8(addr + 2, 0x00);
        }

        let initial_cycles = cpu.cycle_count();
        run_batch(&mut cpu, 5).unwrap();

        assert_eq!(cpu.cycle_count(), initial_cycles + 5);
    }
}
