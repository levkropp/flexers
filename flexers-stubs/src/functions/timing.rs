use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// ets_delay_us stub (microsecond delay)
pub struct EtsDelayUs;

impl RomStubHandler for EtsDelayUs {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let us = cpu.get_register(2);

        // Convert microseconds to CPU cycles
        // ESP32 runs at 160 MHz (default) = 160 cycles per microsecond
        let cycles = us * 160;

        // Advance CPU cycle counter
        cpu.inc_cycles(cycles as u64);

        0  // No return value
    }

    fn name(&self) -> &str {
        "ets_delay_us"
    }
}

/// ets_get_cpu_frequency stub
pub struct EtsGetCpuFrequency;

impl RomStubHandler for EtsGetCpuFrequency {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        160  // ESP32 default CPU frequency in MHz
    }

    fn name(&self) -> &str {
        "ets_get_cpu_frequency"
    }
}

/// ets_update_cpu_frequency stub
pub struct EtsUpdateCpuFrequency;

impl RomStubHandler for EtsUpdateCpuFrequency {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Update CPU frequency (for emulation, we ignore this)
        0
    }

    fn name(&self) -> &str {
        "ets_update_cpu_frequency"
    }
}
