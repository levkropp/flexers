use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;

/// esp_rom_printf stub
pub struct EspRomPrintf;

impl RomStubHandler for EspRomPrintf {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        // Arguments:
        // a2 = format string pointer
        // a3-a7 = varargs

        let fmt_ptr = cpu.get_register(2);

        // Read format string from memory
        let mut fmt_bytes = Vec::new();
        let mut addr = fmt_ptr;
        loop {
            let byte = cpu.memory().read_u8(addr);
            if byte == 0 { break; }
            fmt_bytes.push(byte);
            addr += 1;

            // Safety: limit string length to prevent infinite loops
            if fmt_bytes.len() > 4096 {
                break;
            }
        }

        let fmt_str = String::from_utf8_lossy(&fmt_bytes);

        // Simple printf implementation (no full formatting yet)
        // For Phase 3, just print the string and first arg
        let output = if fmt_str.contains("%d") || fmt_str.contains("%u") {
            let arg1 = cpu.get_register(3);
            fmt_str.replace("%d", &arg1.to_string()).replace("%u", &arg1.to_string())
        } else if fmt_str.contains("%x") {
            let arg1 = cpu.get_register(3);
            fmt_str.replace("%x", &format!("{:x}", arg1))
        } else if fmt_str.contains("%s") {
            let str_ptr = cpu.get_register(3);
            let mut str_bytes = Vec::new();
            let mut str_addr = str_ptr;
            loop {
                let byte = cpu.memory().read_u8(str_addr);
                if byte == 0 { break; }
                str_bytes.push(byte);
                str_addr += 1;

                // Safety: limit string length
                if str_bytes.len() > 1024 {
                    break;
                }
            }
            let arg_str = String::from_utf8_lossy(&str_bytes);
            fmt_str.replace("%s", &arg_str)
        } else {
            fmt_str.to_string()
        };

        print!("[ROM_PRINTF] {}", output);

        // Return number of chars printed
        output.len() as u32
    }

    fn name(&self) -> &str {
        "esp_rom_printf"
    }
}

/// ets_putc stub (single character output)
pub struct EtsPutc;

impl RomStubHandler for EtsPutc {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ch = cpu.get_register(2) as u8;
        print!("{}", ch as char);
        0
    }

    fn name(&self) -> &str {
        "ets_putc"
    }
}

/// ets_install_putc1 stub (install custom putc handler)
pub struct EtsInstallPutc1;

impl RomStubHandler for EtsInstallPutc1 {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // For emulation, we ignore custom putc handlers
        // Just return success
        0
    }

    fn name(&self) -> &str {
        "ets_install_putc1"
    }
}

/// memcpy stub
pub struct Memcpy;

impl RomStubHandler for Memcpy {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let dest = cpu.get_register(2);
        let src = cpu.get_register(3);
        let n = cpu.get_register(4);

        // Copy bytes from src to dest
        for i in 0..n {
            let byte = cpu.memory().read_u8(src + i);
            cpu.memory().write_u8(dest + i, byte);
        }

        dest  // Return dest pointer
    }

    fn name(&self) -> &str {
        "memcpy"
    }
}

/// memset stub
pub struct Memset;

impl RomStubHandler for Memset {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let dest = cpu.get_register(2);
        let val = cpu.get_register(3) as u8;
        let n = cpu.get_register(4);

        // Set bytes at dest to val
        for i in 0..n {
            cpu.memory().write_u8(dest + i, val);
        }

        dest  // Return dest pointer
    }

    fn name(&self) -> &str {
        "memset"
    }
}

/// memcmp stub
pub struct Memcmp;

impl RomStubHandler for Memcmp {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ptr1 = cpu.get_register(2);
        let ptr2 = cpu.get_register(3);
        let n = cpu.get_register(4);

        // Compare bytes
        for i in 0..n {
            let byte1 = cpu.memory().read_u8(ptr1 + i);
            let byte2 = cpu.memory().read_u8(ptr2 + i);

            if byte1 < byte2 {
                return (-1i32) as u32;
            } else if byte1 > byte2 {
                return 1;
            }
        }

        0  // Equal
    }

    fn name(&self) -> &str {
        "memcmp"
    }
}

/// memmove stub (like memcpy but handles overlapping regions)
pub struct Memmove;

impl RomStubHandler for Memmove {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let dest = cpu.get_register(2);
        let src = cpu.get_register(3);
        let n = cpu.get_register(4);

        if dest == src || n == 0 {
            return dest;
        }

        // Use temporary buffer to handle overlapping regions
        let mut buffer = Vec::with_capacity(n as usize);
        for i in 0..n {
            buffer.push(cpu.memory().read_u8(src + i));
        }

        for i in 0..n {
            cpu.memory().write_u8(dest + i, buffer[i as usize]);
        }

        dest  // Return dest pointer
    }

    fn name(&self) -> &str {
        "memmove"
    }
}

/// uart_tx_one_char stub
pub struct UartTxOneChar;

impl RomStubHandler for UartTxOneChar {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ch = cpu.get_register(2) as u8;
        print!("{}", ch as char);
        0
    }

    fn name(&self) -> &str {
        "uart_tx_one_char"
    }
}

/// uart_rx_one_char stub
pub struct UartRxOneChar;

impl RomStubHandler for UartRxOneChar {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // For now, return -1 (no data available)
        // In a real implementation, this would read from UART FIFO
        (-1i32) as u32
    }

    fn name(&self) -> &str {
        "uart_rx_one_char"
    }
}

/// uart_div_modify stub
pub struct UartDivModify;

impl RomStubHandler for UartDivModify {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Modify UART divisor (for baud rate)
        // For emulation, we don't need to do anything
        0
    }

    fn name(&self) -> &str {
        "uart_div_modify"
    }
}
