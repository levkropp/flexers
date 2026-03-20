use crate::cpu::XtensaCpu;
use crate::decode::{DecodedInsn, reg_s, reg_t};
use super::{StopReason, ExecError};

/// Execute RST0 format instructions (special register access, NOP, etc.)
pub fn exec_rst0(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let op1 = (insn.word >> 16) & 0xF;

    match op1 {
        0 => exec_st0(cpu, insn),
        1 => exec_and(cpu, insn),
        2 => exec_or(cpu, insn),
        3 => exec_xsr(cpu, insn),
        _ => Err(ExecError::IllegalInstruction(insn.word)),
    }
}

/// Execute ST0 format (RSR, WSR, etc.)
fn exec_st0(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = (insn.word >> 12) & 0xF;

    match r {
        0 => exec_rsr(cpu, insn),
        1 => exec_wsr(cpu, insn),
        2 => exec_nop(cpu, insn),
        3 => exec_rfr(cpu, insn),
        _ => Err(ExecError::IllegalInstruction(insn.word)),
    }
}

/// RSR: Read Special Register
fn exec_rsr(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let sr = reg_s(insn);

    let val = cpu.read_special_register(sr);
    cpu.set_register(t, val);

    Ok(StopReason::Continue)
}

/// WSR: Write Special Register
fn exec_wsr(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let sr = reg_s(insn);

    let val = cpu.get_register(t);
    cpu.write_special_register(sr, val);

    Ok(StopReason::Continue)
}

/// XSR: Exchange Special Register
fn exec_xsr(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let t = reg_t(insn);
    let sr = reg_s(insn);

    let old_sr = cpu.read_special_register(sr);
    let reg_val = cpu.get_register(t);

    cpu.write_special_register(sr, reg_val);
    cpu.set_register(t, old_sr);

    Ok(StopReason::Continue)
}

/// NOP variants
fn exec_nop(_cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // NOP - do nothing
    Ok(StopReason::Continue)
}

/// AND (part of RST0 space)
fn exec_and(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = (insn.word >> 12) & 0xF;
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s) & cpu.get_register(t);
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// OR (part of RST0 space)
fn exec_or(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = (insn.word >> 12) & 0xF;
    let s = reg_s(insn);
    let t = reg_t(insn);

    let result = cpu.get_register(s) | cpu.get_register(t);
    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// RFR: Read Floating-Point Register (stub)
fn exec_rfr(_cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Floating-point not implemented yet
    Err(ExecError::IllegalInstruction(_insn.word))
}

/// WAITI: Wait for interrupt
pub fn exec_waiti(cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // For now, just halt the CPU
    // A full implementation would check for pending interrupts
    cpu.halt();
    Ok(StopReason::Halted)
}

/// EXTUI: Extract unsigned immediate
pub fn exec_extui(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let r = (insn.word >> 12) & 0xF;
    let t = reg_t(insn);
    let shiftimm = reg_s(insn); // Actually shift amount
    let maskimm = (insn.word >> 16) & 0xF; // Mask size - 1

    let val = cpu.get_register(t);
    let shifted = val >> shiftimm;
    let mask = (1u32 << (maskimm + 1)) - 1;
    let result = shifted & mask;

    cpu.set_register(r, result);

    Ok(StopReason::Continue)
}

/// MEMW: Memory barrier
pub fn exec_memw(_cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Memory barrier - no-op in interpreter
    Ok(StopReason::Continue)
}

/// ISYNC: Instruction synchronization
pub fn exec_isync(_cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Instruction sync - no-op in interpreter
    Ok(StopReason::Continue)
}

/// DSYNC: Data synchronization
pub fn exec_dsync(_cpu: &mut XtensaCpu, _insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Data sync - no-op in interpreter
    Ok(StopReason::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use std::sync::Arc;

    #[test]
    fn test_rsr() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // Set PS register
        cpu.write_special_register(83, 0x12345678);

        // RSR a2, PS (sr=83)
        let insn = DecodedInsn {
            word: 0x00_02_530, // t=2, sr=83, r=0, op1=0
            len: 3,
        };

        exec_rsr(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(2), 0x12345678);
    }

    #[test]
    fn test_wsr() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_register(3, 0xABCDEF00);

        // WSR a3, SAR (sr=3)
        let insn = DecodedInsn {
            word: 0x00_13_30, // t=3, sr=3, r=1, op1=0
            len: 3,
        };

        exec_wsr(&mut cpu, insn).unwrap();
        assert_eq!(cpu.read_special_register(3), 0xABCDEF00 & 0x1F); // SAR masks to 5 bits
    }

    #[test]
    fn test_xsr() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.set_register(4, 0x1111);
        cpu.write_special_register(96, 0x2222); // CCOUNT

        // XSR a4, CCOUNT (sr=96)
        let insn = DecodedInsn {
            word: 0x30_04_600, // t=4, sr=96
            len: 3,
        };

        exec_xsr(&mut cpu, insn).unwrap();
        assert_eq!(cpu.get_register(4), 0x2222);
        assert_eq!(cpu.read_special_register(96), 0x1111);
    }
}
