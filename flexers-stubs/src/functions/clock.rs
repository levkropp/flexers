/// Clock management ROM functions
/// Simplified for emulation

use flexers_core::cpu::XtensaCpu;

/// CPU frequencies
const APB_CLK_FREQ: u32 = 80_000_000; // 80 MHz
const CPU_CLK_FREQ_80M: u32 = 80_000_000;
const CPU_CLK_FREQ_160M: u32 = 160_000_000;
const CPU_CLK_FREQ_240M: u32 = 240_000_000;

/// rtc_clk_cpu_freq_get() - Get current CPU frequency
/// a2 = frequency (output, in Hz)
pub fn exec_rtc_clk_cpu_freq_get(cpu: &mut XtensaCpu) -> Result<(), String> {
    // Default to 160MHz
    cpu.set_register(2, CPU_CLK_FREQ_160M);
    Ok(())
}

/// rtc_clk_cpu_freq_set(freq_mhz) - Set CPU frequency
/// a2 = freq_mhz (input: 80, 160, or 240)
pub fn exec_rtc_clk_cpu_freq_set(cpu: &mut XtensaCpu) -> Result<(), String> {
    let freq_mhz = cpu.get_register(2);

    // Validate frequency
    match freq_mhz {
        80 | 160 | 240 => {
            // In emulation, we don't actually change frequency
            // Just return success
            cpu.set_register(2, 0);
        }
        _ => {
            // Invalid frequency
            cpu.set_register(2, 1); // Error
        }
    }

    Ok(())
}

/// periph_module_enable(periph) - Enable peripheral module clock
/// a2 = periph (input, peripheral module ID)
pub fn exec_periph_module_enable(cpu: &mut XtensaCpu) -> Result<(), String> {
    let _periph = cpu.get_register(2);
    // In emulation, all peripherals are always enabled
    Ok(())
}

/// periph_module_disable(periph) - Disable peripheral module clock
/// a2 = periph (input)
pub fn exec_periph_module_disable(cpu: &mut XtensaCpu) -> Result<(), String> {
    let _periph = cpu.get_register(2);
    // In emulation, this is a no-op
    Ok(())
}

/// rtc_clk_apb_freq_get() - Get APB clock frequency
/// a2 = frequency (output, in Hz)
pub fn exec_rtc_clk_apb_freq_get(cpu: &mut XtensaCpu) -> Result<(), String> {
    cpu.set_register(2, APB_CLK_FREQ);
    Ok(())
}
