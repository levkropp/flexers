use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// Cache_Read_Enable stub (enable flash cache)
pub struct CacheReadEnable;

impl RomStubHandler for CacheReadEnable {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        // Arguments:
        // a2 = odd_rom (0 or 1)
        // a3 = even_rom (0 or 1)
        // a4 = odd_cache_size
        // a5 = even_cache_size

        // For emulation:
        // - Flash regions already mapped in page table
        // - No actual cache management needed
        // - Just return success

        #[cfg(debug_assertions)]
        {
            let odd_rom = cpu.get_register(2);
            let even_rom = cpu.get_register(3);
            println!("[Cache_Read_Enable] odd_rom={}, even_rom={}", odd_rom, even_rom);
        }

        0  // Success
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

        #[cfg(debug_assertions)]
        {
            println!("[Cache_Read_Disable] called");
        }

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
