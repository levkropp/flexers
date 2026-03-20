use crate::memory::Memory;

/// Decoded instruction with length information
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DecodedInsn {
    /// Instruction word (16 or 24 bits)
    pub word: u32,
    /// Instruction length in bytes (2 or 3)
    pub len: u8,
}

/// Fetch error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchError {
    /// Invalid address (unmapped or misaligned)
    InvalidAddress(u32),
}

/// Fetch instruction from memory
/// Xtensa uses variable-length encoding: narrow (16-bit) or wide (24-bit)
/// Bottom 4 bits of byte0 determine instruction length:
/// - 0..7 (op0 < 8): Wide (24-bit) instruction
/// - 8..15 (op0 >= 8): Narrow (16-bit) instruction
#[inline(always)]
pub fn fetch(mem: &Memory, pc: u32) -> Result<DecodedInsn, FetchError> {
    // Read first byte to determine instruction length
    let byte0 = mem.read_u8(pc);
    let byte1 = mem.read_u8(pc + 1);

    // Check bits [0:3] (op0 field)
    // If op0 >= 8, it's a narrow (16-bit) instruction
    if byte0 & 0x0F >= 8 {
        // Narrow (16-bit) instruction
        let word = (byte0 as u32) | ((byte1 as u32) << 8);
        Ok(DecodedInsn { word, len: 2 })
    } else {
        // Wide (24-bit) instruction
        let byte2 = mem.read_u8(pc + 2);
        let word = (byte0 as u32) | ((byte1 as u32) << 8) | ((byte2 as u32) << 16);
        Ok(DecodedInsn { word, len: 3 })
    }
}

/// Fast decode for performance-critical paths
/// This version assumes valid memory and doesn't do error checking
#[inline(always)]
pub unsafe fn fetch_unchecked(mem: &Memory, pc: u32) -> DecodedInsn {
    let byte0 = mem.read_u8(pc);
    let byte1 = mem.read_u8(pc + 1);

    if byte0 & 0x0F >= 8 {
        // Narrow
        let word = (byte0 as u32) | ((byte1 as u32) << 8);
        DecodedInsn { word, len: 2 }
    } else {
        // Wide
        let byte2 = mem.read_u8(pc + 2);
        let word = (byte0 as u32) | ((byte1 as u32) << 8) | ((byte2 as u32) << 16);
        DecodedInsn { word, len: 3 }
    }
}

/// Extract opcode field (bits [0:3])
#[inline(always)]
pub fn opcode(insn: DecodedInsn) -> u32 {
    insn.word & 0xF
}

/// Extract register field 't' (bits [4:7])
#[inline(always)]
pub fn reg_t(insn: DecodedInsn) -> u32 {
    (insn.word >> 4) & 0xF
}

/// Extract register field 's' (bits [8:11])
#[inline(always)]
pub fn reg_s(insn: DecodedInsn) -> u32 {
    (insn.word >> 8) & 0xF
}

/// Extract register field 'r' (bits [12:15])
#[inline(always)]
pub fn reg_r(insn: DecodedInsn) -> u32 {
    (insn.word >> 12) & 0xF
}

/// Extract immediate field for various formats
#[inline(always)]
pub fn imm8(insn: DecodedInsn) -> u32 {
    (insn.word >> 16) & 0xFF
}

/// Extract signed immediate (sign-extended)
#[inline(always)]
pub fn imm8_se(insn: DecodedInsn) -> i32 {
    let val = ((insn.word >> 16) & 0xFF) as i32;
    // Sign extend from 8 bits
    (val << 24) >> 24
}

/// Extract 12-bit immediate
#[inline(always)]
pub fn imm12(insn: DecodedInsn) -> u32 {
    (insn.word >> 12) & 0xFFF
}

/// Extract offset for L32R (16-bit signed, scaled by 4)
#[inline(always)]
pub fn l32r_offset(insn: DecodedInsn) -> i32 {
    let val = ((insn.word >> 8) & 0xFFFF) as i32;
    // Sign extend from 16 bits
    let se = (val << 16) >> 16;
    // Scale by 4 (word offset)
    se << 2
}

/// Extract offset for CALL (18-bit signed, scaled by 4)
#[inline(always)]
pub fn call_offset(insn: DecodedInsn) -> i32 {
    let val = ((insn.word >> 6) & 0x3FFFF) as i32;
    // Sign extend from 18 bits
    let se = (val << 14) >> 14;
    // Scale by 4
    se << 2
}

/// Extract offset for branches (8-bit signed)
#[inline(always)]
pub fn branch_offset(insn: DecodedInsn) -> i32 {
    imm8_se(insn)
}

/// Extract shift amount (SA field)
#[inline(always)]
pub fn shift_amount(insn: DecodedInsn) -> u32 {
    (insn.word >> 8) & 0x1F
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    #[test]
    fn test_narrow_decode() {
        let mem = Memory::new();
        // Create a narrow instruction: 0x123C (op0 = 0xC >= 8)
        mem.write_u8(0x40000000, 0x3C);
        mem.write_u8(0x40000001, 0x12);

        let insn = fetch(&mem, 0x40000000).unwrap();
        assert_eq!(insn.len, 2);
        assert_eq!(insn.word, 0x123C);
    }

    #[test]
    fn test_wide_decode() {
        let mem = Memory::new();
        // Create a wide instruction: 0x123456 (op0 = 0x6 < 8)
        mem.write_u8(0x40000000, 0x56);
        mem.write_u8(0x40000001, 0x34);
        mem.write_u8(0x40000002, 0x12);

        let insn = fetch(&mem, 0x40000000).unwrap();
        assert_eq!(insn.len, 3);
        assert_eq!(insn.word, 0x123456);
    }

    #[test]
    fn test_field_extraction() {
        let insn = DecodedInsn {
            word: 0x123456,
            len: 3,
        };

        assert_eq!(opcode(insn), 0x6);
        assert_eq!(reg_t(insn), 0x5);
        assert_eq!(reg_s(insn), 0x4);
        assert_eq!(reg_r(insn), 0x3);
    }

    #[test]
    fn test_immediate_extraction() {
        let insn = DecodedInsn {
            word: 0xAB_CD_EF,
            len: 3,
        };

        assert_eq!(imm8(insn), 0xAB);
    }

    #[test]
    fn test_sign_extension() {
        // Positive value
        let insn1 = DecodedInsn {
            word: 0x7F_00_00,
            len: 3,
        };
        assert_eq!(imm8_se(insn1), 0x7F);

        // Negative value (0x80 = -128)
        let insn2 = DecodedInsn {
            word: 0x80_00_00,
            len: 3,
        };
        assert_eq!(imm8_se(insn2), -128);
    }
}
