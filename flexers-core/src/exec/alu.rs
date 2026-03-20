use crate::cpu::XtensaCpu;
use crate::decode::{DecodedInsn, reg_r, reg_s, reg_t, imm8_se, shift_amount};
use super::{StopReason, ExecError};

/// Execute RRR format arithmetic/logic instructions
pub fn exec_rrr(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let op2 = (insn.word >> 20) & 0xF;

    match op2 {
        0 => exec_add(cpu, insn),
        1 => exec_addx(cpu, insn),
        2 => exec_sub(cpu, insn),
        3 => exec_subx(cpu, insn),
        4 => exec_and(cpu, insn),
        5 => exec_or(cpu, insn),
        6 => exec_xor(cpu, insn),
        8 => exec_mul16(cpu, insn),
        10 => exec_sll(cpu, insn),
        11 => exec_srl(cpu, insn),
        12 => exec_sra(cpu, insn),
        13 => exec_slli(cpu, insn),
        14 => exec_srli(cpu, insn),
        15 => exec_srai(cpu, insn),
        _ => Err(ExecError::IllegalInstruction(insn.word)),
    }
}

/// ADD: Add two registers
fn exec_add(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s).wrapping_add(cpu.get_register(t));
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// ADDX2/4/8: Add with shift
fn exec_addx(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);
    let op2 = (insn.word >> 20) & 0xF;

    let shift_amt = op2 - 1; // ADDX2=1, ADDX4=2, ADDX8=3
    let result = (cpu.get_register(s) << shift_amt).wrapping_add(cpu.get_register(t));
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SUB: Subtract two registers
fn exec_sub(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s).wrapping_sub(cpu.get_register(t));
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SUBX2/4/8: Subtract with shift
fn exec_subx(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);
    let op2 = (insn.word >> 20) & 0xF;

    let shift_amt = op2 - 3; // SUBX2=3, SUBX4=4, SUBX8=5
    let result = (cpu.get_register(s) << shift_amt).wrapping_sub(cpu.get_register(t));
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// AND: Bitwise AND
fn exec_and(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s) & cpu.get_register(t);
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// OR: Bitwise OR
fn exec_or(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s) | cpu.get_register(t);
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// XOR: Bitwise XOR
fn exec_xor(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s) ^ cpu.get_register(t);
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// MUL16S/U: 16-bit multiply
fn exec_mul16(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);
    let t = reg_t(insn);

    // For now, treat as 32-bit multiply (lower 32 bits of result)
    let result = cpu.get_register(s).wrapping_mul(cpu.get_register(t));
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SLL: Shift left logical
fn exec_sll(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let s = reg_s(insn);

    let shift = cpu.read_special_register(3) & 0x1F; // SAR register
    let result = cpu.get_register(s) << shift;
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SRL: Shift right logical
fn exec_srl(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let t = reg_t(insn);

    let shift = cpu.read_special_register(3) & 0x1F; // SAR register
    let result = cpu.get_register(t) >> shift;
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SRA: Shift right arithmetic
fn exec_sra(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let t = reg_t(insn);

    let shift = cpu.read_special_register(3) & 0x1F; // SAR register
    let result = ((cpu.get_register(t) as i32) >> shift) as u32;
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SLLI: Shift left logical immediate
fn exec_slli(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let t = reg_t(insn);
    let sa = shift_amount(insn);

    let result = cpu.get_register(t) << sa;
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SRLI: Shift right logical immediate
fn exec_srli(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let t = reg_t(insn);
    let sa = shift_amount(insn);

    let result = cpu.get_register(t) >> sa;
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// SRAI: Shift right arithmetic immediate
fn exec_srai(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = reg_r(insn);
    let t = reg_t(insn);
    let sa = shift_amount(insn);

    let result = ((cpu.get_register(t) as i32) >> sa) as u32;
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// ADDI: Add immediate
pub fn exec_addi(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8_se(insn);

    let result = (cpu.get_register(s) as i32).wrapping_add(imm) as u32;
    cpu.set_register(t, result);

    Ok(StopReason::Continue)
}

/// MOVI: Move immediate (12-bit signed)
pub fn exec_movi(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let imm = ((insn.word >> 8) & 0xFFF) as i32;
    // Sign extend from 12 bits
    let se = (imm << 20) >> 20;

    cpu.set_register(t, se as u32);

    Ok(StopReason::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_add() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_register(1, 10);
        cpu.set_register(2, 20);

        // ADD a3, a1, a2 (result = 30)
        // Format: RRR with op0=0, op1=0, op2=0 (ADD)
        // Encoding: t=2 (bits 4-7), s=1 (bits 8-11), r=3 (bits 12-15)
        let insn = DecodedInsn {
            word: 0x003120, // op0=0, t=2, s=1, r=3, op1=0, op2=0
            len: 3,
        };

        exec_add(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(3), 30);
    }

    #[test]
    fn test_and() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_register(1, 0xFF);
        cpu.set_register(2, 0xF0);

        // AND a3, a1, a2 (result = 0xF0)
        // op2=4 for AND
        let insn = DecodedInsn {
            word: 0x403120, // op0=0, t=2, s=1, r=3, op2=4
            len: 3,
        };

        exec_and(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(3), 0xF0);
    }

    #[test]
    fn test_slli() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_register(1, 0x10);

        // SLLI a2, a1, 4 (result = 0x100)
        // op2=13 for SLLI, sa=4 in bits [8:12]
        // Format: op0=0, t=1(a1), SA=4 (bits 8-12), r=2(a2), op1=1, op2=13
        // Nibbles: op0=0, t=1, SA_low=4, r=2 (SA_high=0 overlaps r[0]), op1=1, op2=D
        let insn = DecodedInsn {
            word: 0xD12410,
            len: 3,
        };

        exec_slli(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(2), 0x100);
    }
}
