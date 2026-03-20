use crate::cpu::XtensaCpu;
use crate::decode::{DecodedInsn, reg_r, reg_s, reg_t, branch_offset};
use super::{StopReason, ExecError};

/// Execute branch instructions (op0 = 6 or 7)
pub fn exec_branch(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let op0 = insn.word & 0xF;
    let op1 = (insn.word >> 12) & 0xF;

    match (op0, op1) {
        (6, 0) => exec_beqz(cpu, insn),
        (6, 1) => exec_bnez(cpu, insn),
        (6, 2) => exec_bltz(cpu, insn),
        (6, 3) => exec_bgez(cpu, insn),
        (7, 0) => exec_beq(cpu, insn),
        (7, 1) => exec_bne(cpu, insn),
        (7, 2) => exec_blt(cpu, insn),
        (7, 3) => exec_bltu(cpu, insn),
        (7, 4) => exec_bge(cpu, insn),
        (7, 5) => exec_bgeu(cpu, insn),
        (7, 6) => exec_bany(cpu, insn),
        (7, 8) => exec_ball(cpu, insn),
        (7, 9) => exec_bbc(cpu, insn),
        (7, 10) => exec_bbs(cpu, insn),
        _ => Err(ExecError::IllegalInstruction(insn.word)),
    }
}

/// BEQZ: Branch if equal to zero
fn exec_beqz(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let offset = branch_offset(insn);

    if cpu.get_register(s) == 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BNEZ: Branch if not equal to zero
fn exec_bnez(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let offset = branch_offset(insn);

    if cpu.get_register(s) != 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BLTZ: Branch if less than zero
fn exec_bltz(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let offset = branch_offset(insn);

    if (cpu.get_register(s) as i32) < 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BGEZ: Branch if greater than or equal to zero
fn exec_bgez(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let offset = branch_offset(insn);

    if (cpu.get_register(s) as i32) >= 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BEQ: Branch if equal
fn exec_beq(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if cpu.get_register(s) == cpu.get_register(t) {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BNE: Branch if not equal
fn exec_bne(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if cpu.get_register(s) != cpu.get_register(t) {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BLT: Branch if less than (signed)
fn exec_blt(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if (cpu.get_register(s) as i32) < (cpu.get_register(t) as i32) {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BLTU: Branch if less than (unsigned)
fn exec_bltu(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if cpu.get_register(s) < cpu.get_register(t) {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BGE: Branch if greater than or equal (signed)
fn exec_bge(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if (cpu.get_register(s) as i32) >= (cpu.get_register(t) as i32) {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BGEU: Branch if greater than or equal (unsigned)
fn exec_bgeu(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if cpu.get_register(s) >= cpu.get_register(t) {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BANY: Branch if any bit set
fn exec_bany(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    if (cpu.get_register(s) & cpu.get_register(t)) != 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BALL: Branch if all bits set
fn exec_ball(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let offset = branch_offset(insn);

    let mask = cpu.get_register(t);
    if (cpu.get_register(s) & mask) == mask {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BBC: Branch if bit clear
fn exec_bbc(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let bit = cpu.get_register(t) & 0x1F;
    let offset = branch_offset(insn);

    if (cpu.get_register(s) & (1 << bit)) == 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

/// BBS: Branch if bit set
fn exec_bbs(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let s = reg_s(insn);
    let t = reg_t(insn);
    let bit = cpu.get_register(t) & 0x1F;
    let offset = branch_offset(insn);

    if (cpu.get_register(s) & (1 << bit)) != 0 {
        let new_pc = (cpu.pc() as i32 + 4 + offset) as u32;
        cpu.set_pc(new_pc);
        Ok(StopReason::PcWritten)
    } else {
        Ok(StopReason::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_beqz_taken() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_pc(0x40000000);
        cpu.set_register(1, 0); // Zero

        // BEQZ a1, +16
        let insn = DecodedInsn {
            word: 0x10_01_16, // offset=16, s=1
            len: 3,
        };

        let result = exec_beqz(&mut cpu, insn).unwrap();
        assert_eq!(result, StopReason::PcWritten);
        assert_eq!(cpu.pc(), 0x40000014); // PC + 4 + 16
    }

    #[test]
    fn test_beqz_not_taken() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_pc(0x40000000);
        cpu.set_register(1, 42); // Non-zero

        // BEQZ a1, +16
        let insn = DecodedInsn {
            word: 0x10_01_16,
            len: 3,
        };

        let result = exec_beqz(&mut cpu, insn).unwrap();
        assert_eq!(result, StopReason::Continue);
    }

    #[test]
    fn test_bne() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_pc(0x40000000);
        cpu.set_register(1, 10);
        cpu.set_register(2, 20);

        // BNE a1, a2, +8
        let insn = DecodedInsn {
            word: 0x08_17_21, // offset=8, t=2, s=1, op1=1
            len: 3,
        };

        let result = exec_bne(&mut cpu, insn).unwrap();
        assert_eq!(result, StopReason::PcWritten);
        assert_eq!(cpu.pc(), 0x4000000C); // PC + 4 + 8
    }
}
