/// SPIFFS (SPI Flash File System) ROM stubs for ESP32 emulation
///
/// This module implements ESP-IDF VFS/SPIFFS API as ROM stubs, bridging
/// firmware file operations to the SpiffsManager.

use crate::handler::RomStubHandler;
use flexers_core::cpu::XtensaCpu;

use super::spiffs_manager::{
    SPIFFS_MANAGER, SPIFFS_OK, SPIFFS_ERR_NOT_FOUND, SPIFFS_ERR_IO,
    SPIFFS_ERR_BAD_FD, SPIFFS_ERR_NOT_MOUNTED, SPIFFS_ERR_INVALID,
    O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_EXCL, O_TRUNC, O_APPEND,
    SEEK_SET, SEEK_CUR, SEEK_END,
};

/// ESP-IDF error codes
const ESP_OK: i32 = 0;
const ESP_FAIL: i32 = -1;
const ESP_ERR_NO_MEM: i32 = 0x101;
const ESP_ERR_INVALID_ARG: i32 = 0x102;
const ESP_ERR_INVALID_STATE: i32 = 0x103;
const ESP_ERR_NOT_FOUND: i32 = 0x105;

/// Helper: Read C string from memory
fn read_c_string(cpu: &XtensaCpu, ptr: u32, max_len: usize) -> String {
    if ptr == 0 {
        return String::new();
    }

    let mut bytes = Vec::new();
    for i in 0..max_len {
        let byte = cpu.memory().read_u8(ptr + i as u32);
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }

    String::from_utf8_lossy(&bytes).to_string()
}

/// Helper: Write C string to memory
fn write_c_string(cpu: &mut XtensaCpu, ptr: u32, value: &str, max_len: usize) -> usize {
    let bytes = value.as_bytes();
    let write_len = bytes.len().min(max_len - 1);

    for (i, &byte) in bytes.iter().take(write_len).enumerate() {
        cpu.memory().write_u8(ptr + i as u32, byte);
    }

    // Null terminator
    cpu.memory().write_u8(ptr + write_len as u32, 0);

    write_len + 1
}

//=============================================================================
// VFS Registration
//=============================================================================

/// esp_vfs_spiffs_register() - Register SPIFFS with VFS
pub struct EspVfsSpiffsRegister;

impl RomStubHandler for EspVfsSpiffsRegister {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let conf_ptr = cpu.get_ar(2);

        // Read configuration struct
        let base_path_ptr = cpu.memory().read_u32(conf_ptr);
        let partition_label_ptr = cpu.memory().read_u32(conf_ptr + 4);

        let base_path = read_c_string(cpu, base_path_ptr, 32);
        let partition_label = read_c_string(cpu, partition_label_ptr, 32);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.mount(&base_path, &partition_label) {
                    Ok(_) => ESP_OK as u32,
                    Err(_) => ESP_FAIL as u32,
                }
            }
            Err(_) => ESP_FAIL as u32,
        }
    }

    fn name(&self) -> &str {
        "esp_vfs_spiffs_register"
    }
}

/// esp_vfs_spiffs_unregister() - Unregister SPIFFS from VFS
pub struct EspVfsSpiffsUnregister;

impl RomStubHandler for EspVfsSpiffsUnregister {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _partition_label_ptr = cpu.get_ar(2);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.unmount() {
                    Ok(_) => ESP_OK as u32,
                    Err(_) => ESP_FAIL as u32,
                }
            }
            Err(_) => ESP_FAIL as u32,
        }
    }

    fn name(&self) -> &str {
        "esp_vfs_spiffs_unregister"
    }
}

/// esp_spiffs_format() - Format SPIFFS filesystem
pub struct EspSpiffsFormat;

impl RomStubHandler for EspSpiffsFormat {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _partition_label_ptr = cpu.get_ar(2);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.format() {
                    Ok(_) => ESP_OK as u32,
                    Err(_) => ESP_FAIL as u32,
                }
            }
            Err(_) => ESP_FAIL as u32,
        }
    }

    fn name(&self) -> &str {
        "esp_spiffs_format"
    }
}

/// esp_spiffs_info() - Get SPIFFS filesystem info
pub struct EspSpiffsInfo;

impl RomStubHandler for EspSpiffsInfo {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _partition_label_ptr = cpu.get_ar(2);
        let total_bytes_ptr = cpu.get_ar(3);
        let used_bytes_ptr = cpu.get_ar(4);

        match SPIFFS_MANAGER.lock() {
            Ok(mgr) => {
                let info = mgr.info();

                if total_bytes_ptr != 0 {
                    cpu.memory().write_u32(total_bytes_ptr, info.total_bytes as u32);
                }

                if used_bytes_ptr != 0 {
                    cpu.memory().write_u32(used_bytes_ptr, info.used_bytes as u32);
                }

                ESP_OK as u32
            }
            Err(_) => ESP_FAIL as u32,
        }
    }

    fn name(&self) -> &str {
        "esp_spiffs_info"
    }
}

//=============================================================================
// File Operations
//=============================================================================

/// open() - Open file
pub struct SpiffsOpen;

impl RomStubHandler for SpiffsOpen {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let path_ptr = cpu.get_ar(2);
        let flags = cpu.get_ar(3) as i32;
        let mode = cpu.get_ar(4) as i32;

        let path = read_c_string(cpu, path_ptr, 256);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.open(&path, flags, mode) {
                    Ok(fd) => fd as u32,
                    Err(e) => {
                        // Return -1 on error (standard POSIX behavior)
                        // errno would be set in real system
                        0xFFFFFFFF // -1 as u32
                    }
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "open"
    }
}

/// close() - Close file
pub struct SpiffsClose;

impl RomStubHandler for SpiffsClose {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let fd = cpu.get_ar(2) as i32;

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.close(fd) {
                    Ok(_) => 0,
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "close"
    }
}

/// read() - Read from file
pub struct SpiffsRead;

impl RomStubHandler for SpiffsRead {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let fd = cpu.get_ar(2) as i32;
        let buf_ptr = cpu.get_ar(3);
        let count = cpu.get_ar(4) as usize;

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                let mut buf = vec![0u8; count];
                match mgr.read(fd, &mut buf) {
                    Ok(n) => {
                        // Write data to firmware memory
                        for i in 0..n {
                            cpu.memory().write_u8(buf_ptr + i as u32, buf[i]);
                        }
                        n as u32
                    }
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "read"
    }
}

/// write() - Write to file
pub struct SpiffsWrite;

impl RomStubHandler for SpiffsWrite {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let fd = cpu.get_ar(2) as i32;
        let buf_ptr = cpu.get_ar(3);
        let count = cpu.get_ar(4) as usize;

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                // Read data from firmware memory
                let mut buf = vec![0u8; count];
                for i in 0..count {
                    buf[i] = cpu.memory().read_u8(buf_ptr + i as u32);
                }

                match mgr.write(fd, &buf) {
                    Ok(n) => n as u32,
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "write"
    }
}

/// lseek() - Seek in file
pub struct SpiffsLseek;

impl RomStubHandler for SpiffsLseek {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let fd = cpu.get_ar(2) as i32;
        let offset = cpu.get_ar(3) as i64; // May need to handle 64-bit
        let whence = cpu.get_ar(4) as i32;

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.lseek(fd, offset, whence) {
                    Ok(pos) => pos as u32,
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "lseek"
    }
}

/// unlink() - Delete file
pub struct SpiffsUnlink;

impl RomStubHandler for SpiffsUnlink {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let path_ptr = cpu.get_ar(2);
        let path = read_c_string(cpu, path_ptr, 256);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.remove(&path) {
                    Ok(_) => 0,
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "unlink"
    }
}

/// rename() - Rename file
pub struct SpiffsRename;

impl RomStubHandler for SpiffsRename {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let old_path_ptr = cpu.get_ar(2);
        let new_path_ptr = cpu.get_ar(3);

        let old_path = read_c_string(cpu, old_path_ptr, 256);
        let new_path = read_c_string(cpu, new_path_ptr, 256);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.rename(&old_path, &new_path) {
                    Ok(_) => 0,
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "rename"
    }
}

/// stat() - Get file info
pub struct SpiffsStat;

impl RomStubHandler for SpiffsStat {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let path_ptr = cpu.get_ar(2);
        let stat_ptr = cpu.get_ar(3);

        let path = read_c_string(cpu, path_ptr, 256);

        match SPIFFS_MANAGER.lock() {
            Ok(mgr) => {
                match mgr.stat(&path) {
                    Ok(stat) => {
                        // Write stat struct to memory
                        // struct stat layout (simplified):
                        // - st_mode (u32) at offset 0
                        // - st_size (i32) at offset 16
                        let mode = if stat.is_dir { 0x4000 } else { 0x8000 };
                        cpu.memory().write_u32(stat_ptr, mode);
                        cpu.memory().write_u32(stat_ptr + 16, stat.size as u32);
                        0
                    }
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "stat"
    }
}

//=============================================================================
// Directory Operations
//=============================================================================

/// opendir() - Open directory
pub struct SpiffsOpendir;

impl RomStubHandler for SpiffsOpendir {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let path_ptr = cpu.get_ar(2);
        let path = read_c_string(cpu, path_ptr, 256);

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.opendir(&path) {
                    Ok(fd) => fd as u32,
                    Err(_) => 0,
                }
            }
            Err(_) => 0,
        }
    }

    fn name(&self) -> &str {
        "opendir"
    }
}

/// readdir() - Read directory entry
pub struct SpiffsReaddir;

impl RomStubHandler for SpiffsReaddir {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let dir_fd = cpu.get_ar(2) as i32;

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.readdir(dir_fd) {
                    Ok(Some(entry)) => {
                        // Allocate space for dirent struct in firmware memory
                        // For simplicity, we'll use a static buffer at a known address
                        // In a real implementation, we'd allocate from heap

                        // dirent struct layout:
                        // - d_ino (u32) at offset 0
                        // - d_type (u8) at offset 16
                        // - d_name (char[256]) at offset 17

                        let dirent_ptr = 0x3FFB0000; // Static buffer in DRAM

                        cpu.memory().write_u32(dirent_ptr, 1); // d_ino
                        let d_type = if entry.is_dir { 4 } else { 8 }; // DT_DIR or DT_REG
                        cpu.memory().write_u8(dirent_ptr + 16, d_type);

                        // Write filename
                        write_c_string(cpu, dirent_ptr + 17, &entry.name, 256);

                        dirent_ptr
                    }
                    Ok(None) => 0, // End of directory
                    Err(_) => 0,
                }
            }
            Err(_) => 0,
        }
    }

    fn name(&self) -> &str {
        "readdir"
    }
}

/// closedir() - Close directory
pub struct SpiffsClosedir;

impl RomStubHandler for SpiffsClosedir {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let dir_fd = cpu.get_ar(2) as i32;

        match SPIFFS_MANAGER.lock() {
            Ok(mut mgr) => {
                match mgr.closedir(dir_fd) {
                    Ok(_) => 0,
                    Err(_) => 0xFFFFFFFF,
                }
            }
            Err(_) => 0xFFFFFFFF,
        }
    }

    fn name(&self) -> &str {
        "closedir"
    }
}

/// fstat() - Get file info from file descriptor
pub struct SpiffsFstat;

impl RomStubHandler for SpiffsFstat {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let fd = cpu.get_ar(2) as i32;
        let stat_ptr = cpu.get_ar(3);

        // For simplicity, we'll just set some default values
        // A full implementation would track file metadata

        let mode = 0x8000; // Regular file
        let size = 0;      // Unknown size

        cpu.memory().write_u32(stat_ptr, mode);
        cpu.memory().write_u32(stat_ptr + 16, size);

        0
    }

    fn name(&self) -> &str {
        "fstat"
    }
}
