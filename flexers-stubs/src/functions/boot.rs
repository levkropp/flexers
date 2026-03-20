use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// Cache_Read_Enable stub (enable flash cache)
pub struct CacheReadEnable;

impl RomStubHandler for CacheReadEnable {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // On ESP32, this maps flash to address space
        // For emulation, we already have flash mapped
        // Just return success
        0
    }

    fn name(&self) -> &str {
        "Cache_Read_Enable"
    }
}

/// Cache_Read_Disable stub
pub struct CacheReadDisable;

impl RomStubHandler for CacheReadDisable {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Disable flash cache
        // For emulation, we ignore this
        0
    }

    fn name(&self) -> &str {
        "Cache_Read_Disable"
    }
}

/// rtc_get_reset_reason stub
pub struct RtcGetResetReason;

impl RomStubHandler for RtcGetResetReason {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        1  // POWERON_RESET
    }

    fn name(&self) -> &str {
        "rtc_get_reset_reason"
    }
}

/// software_reset stub
pub struct SoftwareReset;

impl RomStubHandler for SoftwareReset {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        // Reset the CPU to initial state
        cpu.halt();
        0
    }

    fn name(&self) -> &str {
        "software_reset"
    }
}
