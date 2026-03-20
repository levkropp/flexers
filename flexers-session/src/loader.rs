use std::fs;
use std::path::Path;
use flexers_core::memory::Memory;

#[derive(Debug)]
pub enum LoadError {
    IoError(std::io::Error),
    InvalidMagic(u8),
    InvalidFormat(String),
    MemoryError(String),
}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::IoError(e)
    }
}

pub struct FirmwareInfo {
    pub entry_point: u32,
    pub segment_count: u8,
    pub segments: Vec<SegmentInfo>,
}

pub struct SegmentInfo {
    pub address: u32,
    pub size: u32,
}

/// Load ESP32 firmware binary and return firmware information
///
/// ESP32 Binary Format:
/// ```text
/// Offset | Size | Field
/// -------|------|-------
/// 0      | 1    | Magic byte (0xE9)
/// 1      | 1    | Segment count
/// 2      | 1    | SPI mode
/// 3      | 1    | SPI speed/size
/// 4      | 4    | Entry point address (little-endian u32)
/// 8      | N    | Segments (variable length)
/// ```
///
/// Each Segment:
/// ```text
/// Offset | Size | Field
/// -------|------|-------
/// 0      | 4    | Load address (u32, little-endian)
/// 4      | 4    | Data length (u32, little-endian)
/// 8      | N    | Data bytes
/// ```
pub fn load_firmware(path: &Path, mem: &Memory) -> Result<FirmwareInfo, LoadError> {
    let data = fs::read(path)?;

    // Validate minimum size (8 bytes for header)
    if data.len() < 8 {
        return Err(LoadError::InvalidFormat(
            "File too small for ESP32 binary".to_string()
        ));
    }

    // Check magic byte
    let magic = data[0];
    if magic != 0xE9 {
        return Err(LoadError::InvalidMagic(magic));
    }

    // Parse header
    let segment_count = data[1];
    let entry_point = u32::from_le_bytes([
        data[4], data[5], data[6], data[7]
    ]);

    // Load segments
    let mut offset = 8;
    let mut segments = Vec::new();

    for _ in 0..segment_count {
        // Parse segment header (8 bytes)
        if offset + 8 > data.len() {
            return Err(LoadError::InvalidFormat(
                "Truncated segment header".to_string()
            ));
        }

        let addr = u32::from_le_bytes([
            data[offset], data[offset+1], data[offset+2], data[offset+3]
        ]);
        let len = u32::from_le_bytes([
            data[offset+4], data[offset+5], data[offset+6], data[offset+7]
        ]) as usize;
        offset += 8;

        // Parse segment data
        if offset + len > data.len() {
            return Err(LoadError::InvalidFormat(
                format!("Truncated segment data (expected {} bytes)", len)
            ));
        }

        let segment_data = &data[offset..offset+len];

        // Write to memory
        for (i, &byte) in segment_data.iter().enumerate() {
            mem.write_u8(addr + i as u32, byte);
        }

        segments.push(SegmentInfo {
            address: addr,
            size: len as u32,
        });

        offset += len;
    }

    Ok(FirmwareInfo {
        entry_point,
        segment_count,
        segments,
    })
}
