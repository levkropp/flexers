use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// Execute abs() - absolute value
pub fn exec_abs(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val = cpu.get_ar(2) as i32;
    let result = val.abs();
    cpu.set_ar(2, result as u32);
    Ok(())
}

/// Execute labs() - long absolute value
pub fn exec_labs(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_abs(cpu) // Same as abs on 32-bit
}

/// Execute llabs() - long long absolute value
pub fn exec_llabs(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_abs(cpu) // Simplified for 32-bit emulation
}

/// Execute sqrt() - square root
pub fn exec_sqrt(cpu: &mut XtensaCpu) -> Result<(), String> {
    // Argument is passed as float bits in a2
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.sqrt();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute sqrtf() - square root float
pub fn exec_sqrtf(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.sqrt();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute pow() - power function
pub fn exec_pow(cpu: &mut XtensaCpu) -> Result<(), String> {
    let base_bits = cpu.get_ar(2);
    let exp_bits = cpu.get_ar(3);

    let base = f32::from_bits(base_bits);
    let exp = f32::from_bits(exp_bits);
    let result = base.powf(exp);

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute powf() - power function float
pub fn exec_powf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_pow(cpu)
}

/// Execute exp() - exponential function
pub fn exec_exp(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.exp();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute expf() - exponential function float
pub fn exec_expf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_exp(cpu)
}

/// Execute log() - natural logarithm
pub fn exec_log(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.ln();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute logf() - natural logarithm float
pub fn exec_logf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_log(cpu)
}

/// Execute log10() - base-10 logarithm
pub fn exec_log10(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.log10();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute log10f() - base-10 logarithm float
pub fn exec_log10f(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_log10(cpu)
}

/// Execute sin() - sine
pub fn exec_sin(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.sin();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute sinf() - sine float
pub fn exec_sinf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_sin(cpu)
}

/// Execute cos() - cosine
pub fn exec_cos(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.cos();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute cosf() - cosine float
pub fn exec_cosf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_cos(cpu)
}

/// Execute tan() - tangent
pub fn exec_tan(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.tan();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute tanf() - tangent float
pub fn exec_tanf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_tan(cpu)
}

/// Execute floor() - floor function
pub fn exec_floor(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.floor();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute floorf() - floor function float
pub fn exec_floorf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_floor(cpu)
}

/// Execute ceil() - ceiling function
pub fn exec_ceil(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.ceil();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute ceilf() - ceiling function float
pub fn exec_ceilf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_ceil(cpu)
}

/// Execute round() - round to nearest integer
pub fn exec_round(cpu: &mut XtensaCpu) -> Result<(), String> {
    let val_bits = cpu.get_ar(2);
    let val = f32::from_bits(val_bits);
    let result = val.round();

    cpu.set_ar(2, result.to_bits());
    Ok(())
}

/// Execute roundf() - round to nearest integer float
pub fn exec_roundf(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_round(cpu)
}

// ROM stub handlers for math functions
pub struct Abs;
impl RomStubHandler for Abs {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_abs(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "abs" }
}

pub struct Labs;
impl RomStubHandler for Labs {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_labs(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "labs" }
}

pub struct Sqrt;
impl RomStubHandler for Sqrt {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_sqrt(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "sqrt" }
}

pub struct Sqrtf;
impl RomStubHandler for Sqrtf {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_sqrtf(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "sqrtf" }
}

pub struct Pow;
impl RomStubHandler for Pow {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_pow(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "pow" }
}

pub struct Powf;
impl RomStubHandler for Powf {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_powf(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "powf" }
}

pub struct Sin;
impl RomStubHandler for Sin {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_sin(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "sin" }
}

pub struct Sinf;
impl RomStubHandler for Sinf {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_sinf(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "sinf" }
}

pub struct Cos;
impl RomStubHandler for Cos {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_cos(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "cos" }
}

pub struct Cosf;
impl RomStubHandler for Cosf {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_cosf(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "cosf" }
}

pub struct Floor;
impl RomStubHandler for Floor {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_floor(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "floor" }
}

pub struct Floorf;
impl RomStubHandler for Floorf {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_floorf(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "floorf" }
}

pub struct Ceil;
impl RomStubHandler for Ceil {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_ceil(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "ceil" }
}

pub struct Round;
impl RomStubHandler for Round {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_round(cpu).ok();
        cpu.get_ar(2)
    }
    fn name(&self) -> &str { "round" }
}
