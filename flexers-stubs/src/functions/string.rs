/// String manipulation ROM functions
/// Standard C string library functions

use flexers_core::cpu::XtensaCpu;

/// strcpy(dest, src) - Copy string
/// a2 = dest (input/output)
/// a3 = src (input)
pub fn exec_strcpy(cpu: &mut XtensaCpu) -> Result<(), String> {
    let dest = cpu.get_register(2);
    let src = cpu.get_register(3);

    let mut i = 0;
    loop {
        let byte = cpu.memory().read_u8(src + i);
        cpu.memory().write_u8(dest + i, byte);

        if byte == 0 {
            break;
        }
        i += 1;

        // Prevent infinite loops
        if i > 4096 {
            return Err("strcpy: string too long".to_string());
        }
    }

    // Return dest
    cpu.set_register(2, dest);
    Ok(())
}

/// strncpy(dest, src, n) - Copy at most n bytes
/// a2 = dest (input/output)
/// a3 = src (input)
/// a4 = n (input)
pub fn exec_strncpy(cpu: &mut XtensaCpu) -> Result<(), String> {
    let dest = cpu.get_register(2);
    let src = cpu.get_register(3);
    let n = cpu.get_register(4);

    for i in 0..n {
        let byte = cpu.memory().read_u8(src + i);
        cpu.memory().write_u8(dest + i, byte);

        if byte == 0 {
            // Null terminator found, pad rest with zeros
            for j in (i + 1)..n {
                cpu.memory().write_u8(dest + j, 0);
            }
            break;
        }
    }

    cpu.set_register(2, dest);
    Ok(())
}

/// strlen(str) - Get string length
/// a2 = str (input)
/// a2 = length (output)
pub fn exec_strlen(cpu: &mut XtensaCpu) -> Result<(), String> {
    let str_ptr = cpu.get_register(2);

    let mut len = 0;
    loop {
        let byte = cpu.memory().read_u8(str_ptr + len);
        if byte == 0 {
            break;
        }
        len += 1;

        // Prevent infinite loops
        if len > 4096 {
            return Err("strlen: string too long".to_string());
        }
    }

    cpu.set_register(2, len);
    Ok(())
}

/// strnlen(str, maxlen) - Get string length with limit
/// a2 = str (input)
/// a3 = maxlen (input)
/// a2 = length (output)
pub fn exec_strnlen(cpu: &mut XtensaCpu) -> Result<(), String> {
    let str_ptr = cpu.get_register(2);
    let maxlen = cpu.get_register(3);

    let mut len = 0;
    while len < maxlen {
        let byte = cpu.memory().read_u8(str_ptr + len);
        if byte == 0 {
            break;
        }
        len += 1;
    }

    cpu.set_register(2, len);
    Ok(())
}

/// strcmp(s1, s2) - Compare strings
/// a2 = s1 (input)
/// a3 = s2 (input)
/// a2 = result (output): <0, 0, or >0
pub fn exec_strcmp(cpu: &mut XtensaCpu) -> Result<(), String> {
    let s1 = cpu.get_register(2);
    let s2 = cpu.get_register(3);

    let mut i = 0;
    loop {
        let c1 = cpu.memory().read_u8(s1 + i);
        let c2 = cpu.memory().read_u8(s2 + i);

        if c1 != c2 {
            cpu.set_register(2, (c1 as i8 - c2 as i8) as u32);
            return Ok(());
        }

        if c1 == 0 {
            // Both strings equal
            cpu.set_register(2, 0);
            return Ok(());
        }

        i += 1;

        if i > 4096 {
            return Err("strcmp: string too long".to_string());
        }
    }
}

/// strncmp(s1, s2, n) - Compare at most n bytes
/// a2 = s1 (input)
/// a3 = s2 (input)
/// a4 = n (input)
/// a2 = result (output)
pub fn exec_strncmp(cpu: &mut XtensaCpu) -> Result<(), String> {
    let s1 = cpu.get_register(2);
    let s2 = cpu.get_register(3);
    let n = cpu.get_register(4);

    for i in 0..n {
        let c1 = cpu.memory().read_u8(s1 + i);
        let c2 = cpu.memory().read_u8(s2 + i);

        if c1 != c2 {
            cpu.set_register(2, (c1 as i8 - c2 as i8) as u32);
            return Ok(());
        }

        if c1 == 0 {
            cpu.set_register(2, 0);
            return Ok(());
        }
    }

    cpu.set_register(2, 0);
    Ok(())
}

/// strcat(dest, src) - Concatenate strings
/// a2 = dest (input/output)
/// a3 = src (input)
pub fn exec_strcat(cpu: &mut XtensaCpu) -> Result<(), String> {
    let dest = cpu.get_register(2);
    let src = cpu.get_register(3);

    // Find end of dest
    let mut dest_len = 0;
    loop {
        let byte = cpu.memory().read_u8(dest + dest_len);
        if byte == 0 {
            break;
        }
        dest_len += 1;

        if dest_len > 4096 {
            return Err("strcat: string too long".to_string());
        }
    }

    // Copy src to end of dest
    let mut i = 0;
    loop {
        let byte = cpu.memory().read_u8(src + i);
        cpu.memory().write_u8(dest + dest_len + i, byte);

        if byte == 0 {
            break;
        }
        i += 1;

        if i > 4096 {
            return Err("strcat: string too long".to_string());
        }
    }

    cpu.set_register(2, dest);
    Ok(())
}

/// strncat(dest, src, n) - Concatenate at most n bytes
/// a2 = dest (input/output)
/// a3 = src (input)
/// a4 = n (input)
pub fn exec_strncat(cpu: &mut XtensaCpu) -> Result<(), String> {
    let dest = cpu.get_register(2);
    let src = cpu.get_register(3);
    let n = cpu.get_register(4);

    // Find end of dest
    let mut dest_len = 0;
    loop {
        let byte = cpu.memory().read_u8(dest + dest_len);
        if byte == 0 {
            break;
        }
        dest_len += 1;

        if dest_len > 4096 {
            return Err("strncat: string too long".to_string());
        }
    }

    // Copy at most n bytes from src
    for i in 0..n {
        let byte = cpu.memory().read_u8(src + i);
        if byte == 0 {
            cpu.memory().write_u8(dest + dest_len + i, 0);
            break;
        }
        cpu.memory().write_u8(dest + dest_len + i, byte);

        // Ensure null termination
        if i == n - 1 {
            cpu.memory().write_u8(dest + dest_len + i + 1, 0);
        }
    }

    cpu.set_register(2, dest);
    Ok(())
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
    fn test_strcpy() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let dest = 0x3FFE_0000;
        let src = 0x3FFE_1000;

        write_string(&cpu, src, "Hello");

        cpu.set_register(2, dest);
        cpu.set_register(3, src);
        exec_strcpy(&mut cpu).unwrap();

        assert_eq!(read_string(&cpu, dest, 100), "Hello");
        assert_eq!(cpu.get_register(2), dest);
    }

    #[test]
    fn test_strlen() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let str_addr = 0x3FFE_0000;
        write_string(&cpu, str_addr, "Hello World");

        cpu.set_register(2, str_addr);
        exec_strlen(&mut cpu).unwrap();

        assert_eq!(cpu.get_register(2), 11);
    }

    #[test]
    fn test_strcmp() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let s1 = 0x3FFE_0000;
        let s2 = 0x3FFE_1000;

        // Equal strings
        write_string(&cpu, s1, "hello");
        write_string(&cpu, s2, "hello");
        cpu.set_register(2, s1);
        cpu.set_register(3, s2);
        exec_strcmp(&mut cpu).unwrap();
        assert_eq!(cpu.get_register(2), 0);

        // Different strings
        write_string(&cpu, s1, "hello");
        write_string(&cpu, s2, "world");
        cpu.set_register(2, s1);
        cpu.set_register(3, s2);
        exec_strcmp(&mut cpu).unwrap();
        assert_ne!(cpu.get_register(2), 0);
    }

    #[test]
    fn test_strcat() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        let dest = 0x3FFE_0000;
        let src = 0x3FFE_1000;

        write_string(&cpu, dest, "Hello ");
        write_string(&cpu, src, "World");

        cpu.set_register(2, dest);
        cpu.set_register(3, src);
        exec_strcat(&mut cpu).unwrap();

        assert_eq!(read_string(&cpu, dest, 100), "Hello World");
    }
}

// ROM Stub Handler implementations

use crate::handler::RomStubHandler;

pub struct Strcpy;
impl RomStubHandler for Strcpy {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strcpy(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strcpy" }
}

pub struct Strlen;
impl RomStubHandler for Strlen {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strlen(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strlen" }
}

pub struct Strcmp;
impl RomStubHandler for Strcmp {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strcmp(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strcmp" }
}

pub struct Strcat;
impl RomStubHandler for Strcat {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strcat(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strcat" }
}

pub struct Strncpy;
impl RomStubHandler for Strncpy {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strncpy(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strncpy" }
}

pub struct Strncmp;
impl RomStubHandler for Strncmp {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        exec_strncmp(cpu).ok();
        cpu.get_register(2)
    }
    fn name(&self) -> &str { "strncmp" }
}
