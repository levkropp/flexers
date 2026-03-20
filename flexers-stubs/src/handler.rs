use flexers_core::cpu::XtensaCpu;

/// Trait for ROM function stub handlers
pub trait RomStubHandler: Send + Sync {
    /// Execute the ROM function stub
    ///
    /// Arguments are in registers:
    /// - a2: arg1 (also used for return value)
    /// - a3: arg2
    /// - a4: arg3
    /// - a5: arg4
    /// - a6: arg5
    /// - a7: arg6
    ///
    /// Returns the value to place in a2 (return value)
    fn call(&self, cpu: &mut XtensaCpu) -> u32;

    /// Function name (for debugging)
    fn name(&self) -> &str;
}
