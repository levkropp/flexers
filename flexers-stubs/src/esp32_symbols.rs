/// ESP32 ROM symbols (embedded data)
/// Format: (function_name, address, num_args)
///
/// NOTE: These addresses are placeholders and should be updated from
/// esp32.rom.ld in ESP-IDF repository for real ESP32 ROM addresses.
///
/// For Phase 3, we use placeholder addresses in the ROM range (0x4000_0000+)
pub const ESP32_ROM_SYMBOLS: &[(&str, u32, u8)] = &[
    // I/O functions
    ("esp_rom_printf", 0x40007ABC, 2),
    ("ets_putc", 0x40007CDE, 1),
    ("ets_install_putc1", 0x40007D12, 1),

    // Timing functions
    ("ets_delay_us", 0x40008534, 1),
    ("ets_get_cpu_frequency", 0x40008550, 0),
    ("ets_update_cpu_frequency", 0x40008564, 1),

    // Memory functions
    ("memcpy", 0x4000C2C4, 3),
    ("memset", 0x4000C2E0, 3),
    ("memcmp", 0x4000C2FC, 3),
    ("memmove", 0x4000C318, 3),

    // UART functions
    ("uart_tx_one_char", 0x40009200, 1),
    ("uart_rx_one_char", 0x40009214, 1),
    ("uart_div_modify", 0x40009238, 2),

    // Boot/system functions
    ("Cache_Read_Enable", 0x40009A44, 4),
    ("Cache_Read_Disable", 0x40009A60, 0),
    ("rtc_get_reset_reason", 0x40008B94, 1),
    ("software_reset", 0x40008AB8, 0),
];
