use crate::cpu::XtensaCpu;
use crate::decode::{DecodedInsn, reg_s, call_offset};
use super::{StopReason, ExecError};

/// Execute CALL instructions (op0 = 5)
pub fn exec_call(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let n = (insn.word >> 4) & 0x3;

    match n {
        0 => exec_call0(cpu, insn),
        1 => exec_call4(cpu, insn),
        2 => exec_call8(cpu, insn),
        3 => exec_call12(cpu, insn),
        _ => unreachable!(),
    }
}

/// CALL0: Call without register window rotation
fn exec_call0(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let offset = call_offset(insn);

    // Save return address in a0
    cpu.set_register(0, cpu.pc() + 3);

    // Jump to target
    let target = (cpu.pc() as i32 + 4 + offset) as u32;
    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// CALL4: Call with 4-register window rotation
fn exec_call4(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let offset = call_offset(insn);

    // Rotate window forward by 4 registers
    cpu.rotate_window(1);

    // Save return address in a0 (new window)
    cpu.set_register(0, cpu.pc() + 3);

    // Jump to target
    let target = (cpu.pc() as i32 + 4 + offset) as u32;
    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// CALL8: Call with 8-register window rotation
fn exec_call8(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let offset = call_offset(insn);

    // Rotate window forward by 8 registers
    cpu.rotate_window(2);

    // Save return address in a0 (new window)
    cpu.set_register(0, cpu.pc() + 3);

    // Jump to target
    let target = (cpu.pc() as i32 + 4 + offset) as u32;
    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// CALL12: Call with 12-register window rotation
fn exec_call12(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let offset = call_offset(insn);

    // Rotate window forward by 12 registers
    cpu.rotate_window(3);

    // Save return address in a0 (new window)
    cpu.set_register(0, cpu.pc() + 3);

    // Jump to target
    let target = (cpu.pc() as i32 + 4 + offset) as u32;
    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// CALLX0: Indirect call without window rotation
pub fn exec_callx0(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);

    // Save return address in a0
    cpu.set_register(0, cpu.pc() + 3);

    // Jump to address in register
    let target = cpu.get_register(s);
    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// RET: Return (CALLX0 with s=0)
pub fn exec_ret(cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Return address is in a0
    let target = cpu.get_register(0);
    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// RETW: Return with window rotation
pub fn exec_retw(cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Return address is in a0
    let target = cpu.get_register(0);

    // Rotate window backward
    // The amount to rotate is encoded in WINDOWBASE
    // For simplicity, we rotate back by 1 (4 registers)
    // This should be determined by WINDOWSTART in a full implementation
    cpu.rotate_window(-1);

    cpu.set_pc(target);

    Ok(StopReason::PcWritten)
}

/// ENTRY: Function entry (allocate stack frame)
pub fn exec_entry(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let imm = ((insn.word >> 8) & 0xFFF) << 3; // Scaled by 8

    // Adjust stack pointer
    let sp = cpu.get_register(s);
    let new_sp = sp.wrapping_sub(imm);
    cpu.set_register(1, new_sp); // a1 is stack pointer

    Ok(StopReason::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_call0() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_pc(0x40000000);

        // CALL0 +64
        let insn = DecodedInsn {
            word: 0x000405, // op0=5, n=0, offset=16 words (64 bytes / 4)
            len: 3,
        };

        let result = exec_call0(&mut cpu, insn).unwrap();
        assert_eq!(result, StopReason::PcWritten);
        assert_eq!(cpu.pc(), 0x40000044); // PC + 4 + 64
        assert_eq!(cpu.get_register(0), 0x40000003); // Return address
    }

    #[test]
    fn test_call4() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_pc(0x40000000);
        let old_wb = cpu.read_special_register(72); // WINDOWBASE

        // CALL4 +32
        let insn = DecodedInsn {
            word: 0x000020_15, // offset=8 words = 32 bytes, n=1
            len: 3,
        };

        exec_call4(&mut cpu, insn).unwrap();

        // Window should have rotated
        let new_wb = cpu.read_special_register(72);
        assert_eq!(new_wb, (old_wb + 1) & 0xF);
    }

    #[test]
    fn test_ret() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // Set return address
        cpu.set_register(0, 0x40001234);

        // RET
        let insn = DecodedInsn {
            word: 0x000000_00,
            len: 3,
        };

        exec_ret(&mut cpu, insn).unwrap();
        assert_eq!(cpu.pc(), 0x40001234);
    }
}
