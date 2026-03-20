use crate::cpu::XtensaCpu;
use crate::decode::DecodedInsn;

pub mod alu;
pub mod load_store;
pub mod branch;
pub mod call;
pub mod special;

/// Execution result - indicates what happened after instruction execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Continue to next instruction (PC += len)
    Continue,
    /// PC was written by instruction (branch/jump)
    PcWritten,
    /// CPU halted (WAITI or error)
    Halted,
}

/// Execution error types
#[derive(Debug)]
pub enum ExecError {
    /// Illegal or unimplemented instruction
    IllegalInstruction(u32),
    /// Memory access fault
    MemoryFault(u32),
    /// Divide by zero
    DivideByZero,
    /// Privilege violation
    PrivilegeViolation,
    /// ROM stub error
    RomStubError(String),
}

/// Main instruction dispatcher
pub fn execute(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    match insn.len {
        2 => execute_narrow(cpu, insn),
        3 => execute_wide(cpu, insn),
        _ => unreachable!(),
    }
}

/// Execute narrow (16-bit) instructions
fn execute_narrow(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // Most narrow instructions are in the QRST space
    // For now, treat as illegal - we'll implement specific narrow instructions later
    Err(ExecError::IllegalInstruction(insn.word))
}

/// Execute wide (24-bit) instructions
fn execute_wide(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let op0 = insn.word & 0xF;

    match op0 {
        0 => execute_qrst(cpu, insn),
        1 => load_store::exec_l32r(cpu, insn),
        2 => load_store::exec_lsai(cpu, insn),
        5 => call::exec_call(cpu, insn),
        6 => branch::exec_branch(cpu, insn),
        7 => branch::exec_branch(cpu, insn),
        _ => {
            // Try other instruction categories
            execute_other(cpu, insn)
        }
    }
}

/// Execute QRST format instructions (op0 = 0)
fn execute_qrst(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    let op1 = (insn.word >> 16) & 0xF;
    let op2 = (insn.word >> 20) & 0xF;

    match op1 {
        0 => {
            // RST0: Special register operations
            match op2 {
                0 => special::exec_rst0(cpu, insn),
                _ => Err(ExecError::IllegalInstruction(insn.word)),
            }
        }
        _ => {
            // RRR format: arithmetic/logic operations
            alu::exec_rrr(cpu, insn)
        }
    }
}

/// Execute other instruction categories
fn execute_other(cpu: &mut XtensaCpu, insn: DecodedInsn) -> Result<StopReason, ExecError> {
    // For now, return illegal instruction
    // We'll add more categories as needed
    Err(ExecError::IllegalInstruction(insn.word))
}
