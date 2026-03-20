/// Number conversion ROM functions
/// Standard C library conversion functions

use flexers_core::cpu::XtensaCpu;

/// atoi(str) - Convert string to integer
/// a2 = str (input)
/// a2 = value (output)
pub fn exec_atoi(cpu: &mut XtensaCpu) -> Result<(), String> {
    let str_ptr = cpu.get_register(2);

    let mut i = 0;
    let mut result: i32 = 0;
    let mut sign = 1;
    let mut started = false;

    loop {
        let byte = cpu.memory().read_u8(str_ptr + i);

        if byte == 0 {
            break;
        }

        // Skip leading whitespace
        if !started && (byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r') {
            i += 1;
            continue;
        }

        // Handle sign
        if !started && byte == b'-' {
            sign = -1;
            started = true;
            i += 1;
            continue;
        }

        if !started && byte == b'+' {
            started = true;
            i += 1;
            continue;
        }

        // Parse digits
        if byte >= b'0' && byte <= b'9' {
            started = true;
            result = result.wrapping_mul(10).wrapping_add((byte - b'0') as i32);
            i += 1;
        } else {
            // Stop at first non-digit
            break;
        }

        if i > 64 {
            break; // Reasonable limit
        }
    }

    cpu.set_register(2, (result * sign) as u32);
    Ok(())
}

/// atol(str) - Convert string to long (same as atoi for 32-bit)
pub fn exec_atol(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_atoi(cpu)
}

/// atoll(str) - Convert string to long long (same as atoi for 32-bit)
pub fn exec_atoll(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_atoi(cpu)
}

/// itoa(value, str, base) - Convert integer to string
/// a2 = value (input)
/// a3 = str (input/output)
/// a4 = base (input, typically 10 or 16)
pub fn exec_itoa(cpu: &mut XtensaCpu) -> Result<(), String> {
    let value = cpu.get_register(2) as i32;
    let str_ptr = cpu.get_register(3);
    let base = cpu.get_register(4).max(2).min(36) as i32;

    let mut num = value;
    let mut negative = false;

    if num < 0 && base == 10 {
        negative = true;
        num = -num;
    }

    let num = num as u32;
    let mut digits = Vec::new();

    if num == 0 {
        digits.push(b'0');
    } else {
        let mut n = num;
        while n > 0 {
            let digit = (n % base as u32) as u8;
            let ch = if digit < 10 {
                b'0' + digit
            } else {
                b'a' + digit - 10
            };
            digits.push(ch);
            n /= base as u32;
        }
    }

    // Write to memory (reverse order)
    let mut idx = 0;

    if negative {
        cpu.memory().write_u8(str_ptr + idx, b'-');
        idx += 1;
    }

    for &digit in digits.iter().rev() {
        cpu.memory().write_u8(str_ptr + idx, digit);
        idx += 1;
    }

    cpu.memory().write_u8(str_ptr + idx, 0); // Null terminator

    cpu.set_register(2, str_ptr);
    Ok(())
}

/// ltoa(value, str, base) - Convert long to string (same as itoa)
pub fn exec_ltoa(cpu: &mut XtensaCpu) -> Result<(), String> {
    exec_itoa(cpu)
}

/// strtol(str, endptr, base) - Convert string to long
/// a2 = str (input)
/// a3 = endptr (output, pointer to end of conversion)
/// a4 = base (input)
/// a2 = value (output)
pub fn exec_strtol(cpu: &mut XtensaCpu) -> Result<(), String> {
    let str_ptr = cpu.get_register(2);
    let endptr_ptr = cpu.get_register(3);
    let base = cpu.get_register(4);

    let mut i = 0;
    let mut result: i32 = 0;
    let mut sign = 1;
    let mut started = false;
    let actual_base = if base == 0 { 10 } else { base as i32 };

    loop {
        let byte = cpu.memory().read_u8(str_ptr + i);

        if byte == 0 {
            break;
        }

        // Skip leading whitespace
        if !started && (byte == b' ' || byte == b'\t') {
            i += 1;
            continue;
        }

        // Handle sign
        if !started && byte == b'-' {
            sign = -1;
            started = true;
            i += 1;
            continue;
        }

        if !started && byte == b'+' {
            started = true;
            i += 1;
            continue;
        }

        // Parse digits
        let digit_value = if byte >= b'0' && byte <= b'9' {
            (byte - b'0') as i32
        } else if byte >= b'a' && byte <= b'z' {
            (byte - b'a') as i32 + 10
        } else if byte >= b'A' && byte <= b'Z' {
            (byte - b'A') as i32 + 10
        } else {
            break; // Non-digit
        };

        if digit_value >= actual_base {
            break; // Invalid digit for this base
        }

        started = true;
        result = result.wrapping_mul(actual_base).wrapping_add(digit_value);
        i += 1;

        if i > 64 {
            break;
        }
    }

    // Set endptr if provided
    if endptr_ptr != 0 {
        cpu.memory().write_u32(endptr_ptr, str_ptr + i);
    }

    cpu.set_register(2, (result * sign) as u32);
    Ok(())
}

/// strtoul(str, endptr, base) - Convert string to unsigned long
pub fn exec_strtoul(cpu: &mut XtensaCpu) -> Result<(), String> {
    // For simplicity, same as strtol but interpret as unsigned
    exec_strtol(cpu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexers_core::memory::Memory;
    use std::sync::Arc;

    fn write_string(cpu: &XtensaCpu, addr: u32, s: &str) {
        for (i, &byte) in s.as_bytes().iter().enumerate() {
            cpu.memory().write_u8(addr + i as u32, byte);
        }
        cpu.memory().write_u8(addr + s.len() as u32, 0);
    }

    fn read_string(cpu: &XtensaCpu, addr: u32, max_len: u32) -> String {
        let mut result = Vec::new();
        for i in 0..max_len {
            let byte = cpu.memory().read_u8(addr + i);
            if byte == 0 {
                break;
            }
            result.push(byte);
        }
        String::from_utf8(result).unwrap()
    }

    #[test]
    fn test_atoi_positive() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;
        write_string(&cpu, str_addr, "12345");

        cpu.set_register(2, str_addr);
        exec_atoi(&mut cpu).unwrap();

        assert_eq!(cpu.get_register(2) as i32, 12345);
    }

    #[test]
    fn test_atoi_negative() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;
        write_string(&cpu, str_addr, "-42");

        cpu.set_register(2, str_addr);
        exec_atoi(&mut cpu).unwrap();

        assert_eq!(cpu.get_register(2) as i32, -42);
    }

    #[test]
    fn test_atoi_whitespace() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;
        write_string(&cpu, str_addr, "  123");

        cpu.set_register(2, str_addr);
        exec_atoi(&mut cpu).unwrap();

        assert_eq!(cpu.get_register(2) as i32, 123);
    }

    #[test]
    fn test_itoa_decimal() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;

        cpu.set_register(2, 12345);
        cpu.set_register(3, str_addr);
        cpu.set_register(4, 10); // Base 10

        exec_itoa(&mut cpu).unwrap();

        assert_eq!(read_string(&cpu, str_addr, 100), "12345");
    }

    #[test]
    fn test_itoa_hex() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;

        cpu.set_register(2, 255);
        cpu.set_register(3, str_addr);
        cpu.set_register(4, 16); // Base 16

        exec_itoa(&mut cpu).unwrap();

        assert_eq!(read_string(&cpu, str_addr, 100), "ff");
    }

    #[test]
    fn test_itoa_negative() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;

        cpu.set_register(2, (-42i32) as u32);
        cpu.set_register(3, str_addr);
        cpu.set_register(4, 10);

        exec_itoa(&mut cpu).unwrap();

        assert_eq!(read_string(&cpu, str_addr, 100), "-42");
    }

    #[test]
    fn test_strtol() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;
        write_string(&cpu, str_addr, "  -123abc");

        cpu.set_register(2, str_addr);
        cpu.set_register(3, 0); // No endptr
        cpu.set_register(4, 10);

        exec_strtol(&mut cpu).unwrap();

        assert_eq!(cpu.get_register(2) as i32, -123);
    }
}

// ROM Stub Handler implementations

use crate::handler::RomStubHandler;

pub struct Atoi;
impl RomStubHandler for Atoi {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_atoi(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "atoi" }
}

pub struct Itoa;
impl RomStubHandler for Itoa {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_itoa(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "itoa" }
}

pub struct Strtol;
impl RomStubHandler for Strtol {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strtol(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strtol" }
}
