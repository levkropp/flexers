use crate::cpu::XtensaCpu;
use crate::decode::{DecodedInsn, reg_r, reg_s, reg_t, imm8, l32r_offset};
use super::{StopReason, ExecError};

/// L32R: Load 32-bit PC-relative
pub fn exec_l32r(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let offset = l32r_offset(insn);

    // L32R uses PC & ~3 as base
    let base = cpu.pc() & !3;
    let addr = (base as i32 + offset) as u32;

    let val = cpu.memory().read_u32(addr);
    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// Execute LSAI format (Load/Store with immediate offset)
pub fn exec_lsai(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let op1 = (insn.word >> 12) & 0xF; // 'r' field at bits [12:15]

    match op1 {
        0 => exec_l8ui(cpu, insn),
        1 => exec_l16ui(cpu, insn),
        2 => exec_l32i(cpu, insn),
        4 => exec_s8i(cpu, insn),
        5 => exec_s16i(cpu, insn),
        6 => exec_s32i(cpu, insn),
        9 => exec_l16si(cpu, insn),
        10 => exec_movi_n(cpu, insn), // MOVI.N in op1=10
        11 => exec_addi(cpu, insn),   // ADDI in op1=11
        12 => exec_addmi(cpu, insn),  // ADDMI in op1=12
        _ => Err(ExecError::IllegalInstruction(insn.word)),
    }
}

/// L8UI: Load 8-bit unsigned immediate offset
fn exec_l8ui(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn);

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.memory().read_u8(addr) as u32;
    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// L16UI: Load 16-bit unsigned immediate offset
fn exec_l16ui(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn) << 1; // Scaled by 2

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.memory().read_u16(addr) as u32;
    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// L16SI: Load 16-bit signed immediate offset
fn exec_l16si(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn) << 1; // Scaled by 2

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.memory().read_u16(addr) as i16 as i32 as u32; // Sign-extend
    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// L32I: Load 32-bit immediate offset
fn exec_l32i(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn) << 2; // Scaled by 4

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.memory().read_u32(addr);
    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// S8I: Store 8-bit immediate offset
fn exec_s8i(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn);

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.get_register(t) as u8;
    cpu.memory().write_u8(addr, val);

    Ok(StopReason::Continue)
}

/// S16I: Store 16-bit immediate offset
fn exec_s16i(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn) << 1; // Scaled by 2

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.get_register(t) as u16;
    cpu.memory().write_u16(addr, val);

    Ok(StopReason::Continue)
}

/// S32I: Store 32-bit immediate offset
fn exec_s32i(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn) << 2; // Scaled by 4

    let addr = cpu.get_register(s).wrapping_add(imm);
    let val = cpu.get_register(t);
    cpu.memory().write_u32(addr, val);

    Ok(StopReason::Continue)
}

/// MOVI.N: Move immediate narrow (part of LSAI space)
fn exec_movi_n(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let imm = ((insn.word >> 4) & 0xF) | ((insn.word >> 8) & 0xF0);

    // Sign extend from 8 bits if needed
    let val = if imm & 0x80 != 0 {
        (imm as i8 as i32) as u32
    } else {
        imm
    };

    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// ADDI: Add immediate
fn exec_addi(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = imm8(insn) as i8 as i32; // Sign extend

    let result = (cpu.get_register(s) as i32).wrapping_add(imm) as u32;
    cpu.set_register(t, result);

    Ok(StopReason::Continue)
}

/// ADDMI: Add immediate with shift (scaled by 256)
fn exec_addmi(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let s = reg_s(insn);
    let imm = (imm8(insn) as i8 as i32) << 8; // Sign extend and scale

    let result = (cpu.get_register(s) as i32).wrapping_add(imm) as u32;
    cpu.set_register(t, result);

    Ok(StopReason::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_l32i() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem.clone());

        // Write test value to SRAM
        let addr = 0x3FFA_0000;
        mem.write_u32(addr + 16, 0xDEADBEEF);

        // Set base register
        cpu.set_register(1, addr);

        // L32I a2, a1, 16 (offset 4 * 4 = 16)
        // Format: op0=2, t=2, s=1, r=2 (op1), imm8=4
        let insn = DecodedInsn {
            word: 0x042122,
            len: 3,
        };

        exec_l32i(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(2), 0xDEADBEEF);
    }

    #[test]
    fn test_s32i() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem.clone());

        let addr = 0x3FFA_0000;
        cpu.set_register(1, addr);
        cpu.set_register(2, 0xCAFEBABE);

        // S32I a2, a1, 8 (offset 2 * 4 = 8)
        // Format: op0=2, t=2, s=1, r=6 (op1), imm8=2
        let insn = DecodedInsn {
            word: 0x026122,
            len: 3,
        };

        exec_s32i(&mut cpu, insn).unwrap();
        assert_eq!(mem.read_u32(addr + 8), 0xCAFEBABE);
    }

    #[test]
    fn test_l32r() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem.clone());

        // Set PC to 0x40000100
        cpu.set_pc(0x40000100);

        // Write test value at PC - 16
        mem.write_u32(0x40000100 - 16, 0x12345678);

        // L32R a1, -16 (offset = -4 words = -16 bytes)
        let insn = DecodedInsn {
            word: 0xFFFC_11, // offset in bits [8:23]
            len: 3,
        };

        exec_l32r(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(1), 0x12345678);
    }
}
