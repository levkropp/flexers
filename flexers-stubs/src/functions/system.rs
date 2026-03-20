use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// Execute abort() - abnormal termination
pub fn exec_abort(cpu: &mut XtensaCpu) -> Result<(), String> {
    // In emulation, we just return an error to stop execution
    Err(format!("abort() called at PC=0x{:08X}", cpu.pc()))
}

/// Execute exit() - normal termination
pub fn exec_exit(cpu: &mut XtensaCpu) -> Result<(), String> {
    let exit_code = cpu.get_ar(2);
    Err(format!("exit({}) called at PC=0x{:08X}", exit_code, cpu.pc()))
}

/// Execute _exit() - immediate termination
pub fn exec__exit(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_exit(cpu)
}

/// Execute atexit() - register exit handler
pub fn exec_atexit(_cpu: &mut XtensaCpu) -> Result<(), String> {
    // In emulation, we just accept the registration but don't execute handlers
    // Return 0 (success)
    _cpu.set_ar(2, 0);
    Ok(())
}

/// Execute getenv() - get environment variable
pub fn exec_getenv(cpu: &mut XtensaCpu) -> Result<(), String> {
    // In emulation, no environment variables exist
    // Return NULL
    cpu.set_ar(2, 0);
    Ok(())
}

/// Execute setenv() - set environment variable
pub fn exec_setenv(cpu: &mut XtensaCpu) -> Result<(), String> {
    // In emulation, we accept but don't store
    // Return 0 (success)
    cpu.set_ar(2, 0);
    Ok(())
}

/// Execute unsetenv() - unset environment variable
pub fn exec_unsetenv(cpu: &mut XtensaCpu) -> Result<(), String> {
    // In emulation, we accept but don't do anything
    // Return 0 (success)
    cpu.set_ar(2, 0);
    Ok(())
}

/// Execute system() - execute system command
pub fn exec_system(cpu: &mut XtensaCpu) -> Result<(), String> {
    // In emulation, we don't execute system commands
    // Return -1 (failure)
    cpu.set_ar(2, -1i32 as u32);
    Ok(())
}

// ROM stub handlers
pub struct Abort;
impl RomStubHandler for Abort {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_abort(cpu).ok(); // Will return Err but we ignore it here
        0
    }
    fn name(&self) -> &str { "abort" }
}

pub struct Exit;
impl RomStubHandler for Exit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_exit(cpu).ok();
        0
    }
    fn name(&self) -> &str { "exit" }
}

pub struct Atexit;
impl RomStubHandler for Atexit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_atexit(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "atexit" }
}

pub struct Getenv;
impl RomStubHandler for Getenv {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_getenv(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "getenv" }
}

pub struct Setenv;
impl RomStubHandler for Setenv {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_setenv(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "setenv" }
}
