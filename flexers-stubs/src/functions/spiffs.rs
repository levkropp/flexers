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

use super::vfs_manager::{VFS_MANAGER, VfsBackend};

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

        // Register with VFS layer
        match VFS_MANAGER.lock() {
            Ok(mut vfs) => {
                match vfs.register_spiffs(&base_path, &partition_label) {
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
        let base_path_ptr = cpu.get_ar(2);
        let base_path = read_c_string(cpu, base_path_ptr, 32);

        match VFS_MANAGER.lock() {
            Ok(mut vfs) => {
                match vfs.unregister(&base_path) {
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

        // Route through VFS
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                match vfs.route_path(&path) {
                    Some((mount_point, relative_path)) => {
                        match &mount_point.backend {
                            VfsBackend::Spiffs { manager_id } => {
                                if let Some(manager) = vfs.get_spiffs_manager(*manager_id) {
                                    match manager.lock() {
                                        Ok(mut mgr) => {
                                            match mgr.open(&relative_path, flags, mode) {
                                                Ok(fd) => fd as u32,
                                                Err(_) => 0xFFFFFFFF,
                                            }
                                        }
                                        Err(_) => 0xFFFFFFFF,
                                    }
                                } else {
                                    0xFFFFFFFF
                                }
                            }
                        }
                    }
                    None => {
                        // No mount point found - fallback to legacy SPIFFS_MANAGER
                        match SPIFFS_MANAGER.lock() {
                            Ok(mut mgr) => {
                                match mgr.open(&path, flags, mode) {
                                    Ok(fd) => fd as u32,
                                    Err(_) => 0xFFFFFFFF,
                                }
                            }
                            Err(_) => 0xFFFFFFFF,
                        }
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

        // Try all SPIFFS managers (file descriptor is unique across all)
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                // Try each SPIFFS instance
                for i in 0..vfs.get_mounts().len() {
                    if let Some(manager) = vfs.get_spiffs_manager(i) {
                        if let Ok(mut mgr) = manager.lock() {
                            if mgr.close(fd).is_ok() {
                                return 0;
                            }
                        }
                    }
                }
                // Fallback to legacy manager
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

        // Try all SPIFFS managers
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                for i in 0..vfs.get_mounts().len() {
                    if let Some(manager) = vfs.get_spiffs_manager(i) {
                        if let Ok(mut mgr) = manager.lock() {
                            let mut buf = vec![0u8; count];
                            if let Ok(n) = mgr.read(fd, &mut buf) {
                                for j in 0..n {
                                    cpu.memory().write_u8(buf_ptr + j as u32, buf[j]);
                                }
                                return n as u32;
                            }
                        }
                    }
                }
                // Fallback to legacy manager
                match SPIFFS_MANAGER.lock() {
                    Ok(mut mgr) => {
                        let mut buf = vec![0u8; count];
                        match mgr.read(fd, &mut buf) {
                            Ok(n) => {
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

        // Read data from firmware memory
        let mut buf = vec![0u8; count];
        for i in 0..count {
            buf[i] = cpu.memory().read_u8(buf_ptr + i as u32);
        }

        // Try all SPIFFS managers
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                for i in 0..vfs.get_mounts().len() {
                    if let Some(manager) = vfs.get_spiffs_manager(i) {
                        if let Ok(mut mgr) = manager.lock() {
                            if let Ok(n) = mgr.write(fd, &buf) {
                                return n as u32;
                            }
                        }
                    }
                }
                // Fallback to legacy manager
                match SPIFFS_MANAGER.lock() {
                    Ok(mut mgr) => {
                        match mgr.write(fd, &buf) {
                            Ok(n) => n as u32,
                            Err(_) => 0xFFFFFFFF,
                        }
                    }
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
        let offset = cpu.get_ar(3) as i64;
        let whence = cpu.get_ar(4) as i32;

        // Try all SPIFFS managers
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                for i in 0..vfs.get_mounts().len() {
                    if let Some(manager) = vfs.get_spiffs_manager(i) {
                        if let Ok(mut mgr) = manager.lock() {
                            if let Ok(pos) = mgr.lseek(fd, offset, whence) {
                                return pos as u32;
                            }
                        }
                    }
                }
                // Fallback to legacy manager
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

        // Route through VFS
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                match vfs.route_path(&path) {
                    Some((mount_point, relative_path)) => {
                        match &mount_point.backend {
                            VfsBackend::Spiffs { manager_id } => {
                                if let Some(manager) = vfs.get_spiffs_manager(*manager_id) {
                                    match manager.lock() {
                                        Ok(mut mgr) => {
                                            match mgr.remove(&relative_path) {
                                                Ok(_) => 0,
                                                Err(_) => 0xFFFFFFFF,
                                            }
                                        }
                                        Err(_) => 0xFFFFFFFF,
                                    }
                                } else {
                                    0xFFFFFFFF
                                }
                            }
                        }
                    }
                    None => {
                        // Fallback to legacy manager
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

        // Route through VFS
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                match vfs.route_path(&old_path) {
                    Some((mount_point, old_relative)) => {
                        // Both paths must be on same mount point
                        match vfs.route_path(&new_path) {
                            Some((new_mount, new_relative)) => {
                                if mount_point.base_path == new_mount.base_path {
                                    match &mount_point.backend {
                                        VfsBackend::Spiffs { manager_id } => {
                                            if let Some(manager) = vfs.get_spiffs_manager(*manager_id) {
                                                match manager.lock() {
                                                    Ok(mut mgr) => {
                                                        match mgr.rename(&old_relative, &new_relative) {
                                                            Ok(_) => 0,
                                                            Err(_) => 0xFFFFFFFF,
                                                        }
                                                    }
                                                    Err(_) => 0xFFFFFFFF,
                                                }
                                            } else {
                                                0xFFFFFFFF
                                            }
                                        }
                                    }
                                } else {
                                    0xFFFFFFFF // Cross-mount rename not supported
                                }
                            }
                            None => 0xFFFFFFFF,
                        }
                    }
                    None => {
                        // Fallback to legacy manager
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

        // Route through VFS
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                match vfs.route_path(&path) {
                    Some((mount_point, relative_path)) => {
                        match &mount_point.backend {
                            VfsBackend::Spiffs { manager_id } => {
                                if let Some(manager) = vfs.get_spiffs_manager(*manager_id) {
                                    match manager.lock() {
                                        Ok(mgr) => {
                                            match mgr.stat(&relative_path) {
                                                Ok(stat) => {
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
                                } else {
                                    0xFFFFFFFF
                                }
                            }
                        }
                    }
                    None => {
                        // Fallback to legacy manager
                        match SPIFFS_MANAGER.lock() {
                            Ok(mgr) => {
                                match mgr.stat(&path) {
                                    Ok(stat) => {
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

        // Route through VFS
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                match vfs.route_path(&path) {
                    Some((mount_point, relative_path)) => {
                        match &mount_point.backend {
                            VfsBackend::Spiffs { manager_id } => {
                                if let Some(manager) = vfs.get_spiffs_manager(*manager_id) {
                                    match manager.lock() {
                                        Ok(mut mgr) => {
                                            match mgr.opendir(&relative_path) {
                                                Ok(fd) => fd as u32,
                                                Err(_) => 0,
                                            }
                                        }
                                        Err(_) => 0,
                                    }
                                } else {
                                    0
                                }
                            }
                        }
                    }
                    None => {
                        // Fallback to legacy manager
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

        // Try all SPIFFS managers
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                for i in 0..vfs.get_mounts().len() {
                    if let Some(manager) = vfs.get_spiffs_manager(i) {
                        if let Ok(mut mgr) = manager.lock() {
                            if let Ok(result) = mgr.readdir(dir_fd) {
                                if let Some(entry) = result {
                                    let dirent_ptr = 0x3FFB0000;
                                    cpu.memory().write_u32(dirent_ptr, 1);
                                    let d_type = if entry.is_dir { 4 } else { 8 };
                                    cpu.memory().write_u8(dirent_ptr + 16, d_type);
                                    write_c_string(cpu, dirent_ptr + 17, &entry.name, 256);
                                    return dirent_ptr;
                                } else {
                                    return 0; // End of directory
                                }
                            }
                        }
                    }
                }
                // Fallback to legacy manager
                match SPIFFS_MANAGER.lock() {
                    Ok(mut mgr) => {
                        match mgr.readdir(dir_fd) {
                            Ok(Some(entry)) => {
                                let dirent_ptr = 0x3FFB0000;
                                cpu.memory().write_u32(dirent_ptr, 1);
                                let d_type = if entry.is_dir { 4 } else { 8 };
                                cpu.memory().write_u8(dirent_ptr + 16, d_type);
                                write_c_string(cpu, dirent_ptr + 17, &entry.name, 256);
                                dirent_ptr
                            }
                            Ok(None) => 0,
                            Err(_) => 0,
                        }
                    }
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

        // Try all SPIFFS managers
        match VFS_MANAGER.lock() {
            Ok(vfs) => {
                for i in 0..vfs.get_mounts().len() {
                    if let Some(manager) = vfs.get_spiffs_manager(i) {
                        if let Ok(mut mgr) = manager.lock() {
                            if mgr.closedir(dir_fd).is_ok() {
                                return 0;
                            }
                        }
                    }
                }
                // Fallback to legacy manager
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
