/// GPIO initialization ROM functions
/// Simplified for emulation

use flexers_core::cpu::XtensaCpu;

/// esp_rom_gpio_pad_select_gpio(gpio_num) - Configure GPIO pad for GPIO function
/// a2 = gpio_num (input)
pub fn exec_gpio_pad_select_gpio(cpu: &mut XtensaCpu) -> Result<(), String> {
    let _gpio_num = cpu.get_register(2);
    // In emulation, this is a no-op
    // Real implementation would configure GPIO mux
    cpu.set_register(2, 0); // Return success
    Ok(())
}

/// gpio_matrix_in(gpio_num, signal_idx, inv) - Connect GPIO input to peripheral
/// a2 = gpio_num (input)
/// a3 = signal_idx (input)
/// a4 = inv (input, invert signal)
pub fn exec_gpio_matrix_in(cpu: &mut XtensaCpu) -> Result<(), String> {
    let _gpio_num = cpu.get_register(2);
    let _signal_idx = cpu.get_register(3);
    let _inv = cpu.get_register(4);
    // In emulation, this is a no-op
    Ok(())
}

/// gpio_matrix_out(gpio_num, signal_idx, out_inv, oen_inv) - Connect peripheral output to GPIO
/// a2 = gpio_num (input)
/// a3 = signal_idx (input)
/// a4 = out_inv (input)
/// a5 = oen_inv (input)
pub fn exec_gpio_matrix_out(cpu: &mut XtensaCpu) -> Result<(), String> {
    let _gpio_num = cpu.get_register(2);
    let _signal_idx = cpu.get_register(3);
    let _out_inv = cpu.get_register(4);
    let _oen_inv = cpu.get_register(5);
    // In emulation, this is a no-op
    Ok(())
}

/// rtc_gpio_init(gpio_num) - Initialize RTC GPIO
/// a2 = gpio_num (input)
pub fn exec_rtc_gpio_init(cpu: &mut XtensaCpu) -> Result<(), String> {
    let _gpio_num = cpu.get_register(2);
    // In emulation, this is a no-op
    cpu.set_register(2, 0); // Return success
    Ok(())
}
