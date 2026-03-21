/// SPIFFS (SPI Flash File System) manager for ESP32 emulation
///
/// This module provides a filesystem emulation using the host filesystem as a backend.
/// Files created by the firmware are stored on the host for easy inspection and debugging.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

/// SPIFFS error codes (matching ESP-IDF VFS errors)
pub const SPIFFS_OK: i32 = 0;
pub const SPIFFS_ERR_NOT_FOUND: i32 = -2;        // ENOENT
pub const SPIFFS_ERR_IO: i32 = -5;               // EIO
pub const SPIFFS_ERR_BAD_FD: i32 = -9;           // EBADF
pub const SPIFFS_ERR_NO_MEM: i32 = -12;          // ENOMEM
pub const SPIFFS_ERR_NOT_MOUNTED: i32 = -19;     // ENODEV
pub const SPIFFS_ERR_INVALID: i32 = -22;         // EINVAL
pub const SPIFFS_ERR_FILE_EXISTS: i32 = -17;     // EEXIST
pub const SPIFFS_ERR_NOT_DIR: i32 = -20;         // ENOTDIR
pub const SPIFFS_ERR_IS_DIR: i32 = -21;          // EISDIR

/// File open flags (POSIX-compatible)
pub const O_RDONLY: i32 = 0x0000;
pub const O_WRONLY: i32 = 0x0001;
pub const O_RDWR: i32 = 0x0002;
pub const O_CREAT: i32 = 0x0040;
pub const O_EXCL: i32 = 0x0080;
pub const O_TRUNC: i32 = 0x0200;
pub const O_APPEND: i32 = 0x0400;

/// Seek modes
pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

/// SPIFFS file handle
#[derive(Debug)]
pub struct SpiffsFile {
    /// File descriptor (fake, returned to firmware)
    fd: i32,

    /// Host file handle
    file: File,

    /// Path relative to SPIFFS root
    path: PathBuf,

    /// Open flags
    flags: i32,

    /// Current position (for directory iteration)
    dir_position: usize,
}

/// SPIFFS filesystem manager
pub struct SpiffsManager {
    /// Root directory on host filesystem
    root: PathBuf,

    /// Mounted state
    mounted: bool,

    /// Open file handles: fd → SpiffsFile
    files: HashMap<i32, SpiffsFile>,

    /// Next file descriptor
    next_fd: i32,

    /// Partition label
    partition_label: String,

    /// Max files (for resource limiting)
    max_files: usize,
}

impl SpiffsManager {
    pub fn new() -> Self {
        Self::new_with_root(PathBuf::from("/tmp/esp32-spiffs"))
    }

    pub fn new_with_root(root: PathBuf) -> Self {
        Self {
            root,
            mounted: false,
            files: HashMap::new(),
            next_fd: 100, // Start at 100 to avoid conflicts with stdio (0,1,2)
            partition_label: String::from("storage"),
            max_files: 10,
        }
    }

    pub fn mount(&mut self, base_path: &str, partition_label: &str) -> Result<(), i32> {
        if self.mounted {
            return Ok(()); // Already mounted
        }

        // Set partition label
        self.partition_label = partition_label.to_string();

        // Create host directory for this partition
        // The root should already include the partition name from new_with_root()
        if let Err(_) = std::fs::create_dir_all(&self.root) {
            return Err(SPIFFS_ERR_IO);
        }

        self.mounted = true;
        Ok(())
    }

    pub fn unmount(&mut self) -> Result<(), i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        // Close all open files
        self.files.clear();

        self.mounted = false;
        Ok(())
    }

    pub fn format(&mut self) -> Result<(), i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        // Close all files
        self.files.clear();

        // Remove all files in the root directory
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = std::fs::remove_file(path);
                    } else if path.is_dir() {
                        let _ = std::fs::remove_dir_all(path);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn open(&mut self, path: &str, flags: i32, mode: i32) -> Result<i32, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        if self.files.len() >= self.max_files {
            return Err(SPIFFS_ERR_NO_MEM);
        }

        // Remove leading slash if present
        let path = path.trim_start_matches('/');

        // Build full path
        let full_path = self.root.join(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if let Err(_) = std::fs::create_dir_all(parent) {
                return Err(SPIFFS_ERR_IO);
            }
        }

        // Translate flags to OpenOptions
        let mut options = OpenOptions::new();

        let access_mode = flags & 0x0003;
        match access_mode {
            O_RDONLY => { options.read(true); }
            O_WRONLY => { options.write(true); }
            O_RDWR => { options.read(true).write(true); }
            _ => { options.read(true); }
        }

        if flags & O_CREAT != 0 {
            options.create(true);
        }

        if flags & O_EXCL != 0 {
            options.create_new(true);
        }

        if flags & O_TRUNC != 0 {
            options.truncate(true);
        }

        if flags & O_APPEND != 0 {
            options.append(true);
        }

        // Open file
        let file = match options.open(&full_path) {
            Ok(f) => f,
            Err(e) => {
                return match e.kind() {
                    std::io::ErrorKind::NotFound => Err(SPIFFS_ERR_NOT_FOUND),
                    std::io::ErrorKind::AlreadyExists => Err(SPIFFS_ERR_FILE_EXISTS),
                    std::io::ErrorKind::PermissionDenied => Err(SPIFFS_ERR_INVALID),
                    _ => Err(SPIFFS_ERR_IO),
                };
            }
        };

        // Allocate file descriptor
        let fd = self.next_fd;
        self.next_fd += 1;

        // Store file handle
        self.files.insert(fd, SpiffsFile {
            fd,
            file,
            path: full_path,
            flags,
            dir_position: 0,
        });

        Ok(fd)
    }

    pub fn close(&mut self, fd: i32) -> Result<(), i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        self.files.remove(&fd)
            .ok_or(SPIFFS_ERR_BAD_FD)?;

        Ok(())
    }

    pub fn read(&mut self, fd: i32, buf: &mut [u8]) -> Result<usize, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let file = self.files.get_mut(&fd)
            .ok_or(SPIFFS_ERR_BAD_FD)?;

        match file.file.read(buf) {
            Ok(n) => Ok(n),
            Err(_) => Err(SPIFFS_ERR_IO),
        }
    }

    pub fn write(&mut self, fd: i32, data: &[u8]) -> Result<usize, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let file = self.files.get_mut(&fd)
            .ok_or(SPIFFS_ERR_BAD_FD)?;

        match file.file.write(data) {
            Ok(n) => {
                // Flush to ensure data is written
                let _ = file.file.flush();
                Ok(n)
            }
            Err(_) => Err(SPIFFS_ERR_IO),
        }
    }

    pub fn lseek(&mut self, fd: i32, offset: i64, whence: i32) -> Result<i64, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let file = self.files.get_mut(&fd)
            .ok_or(SPIFFS_ERR_BAD_FD)?;

        let seek_from = match whence {
            SEEK_SET => SeekFrom::Start(offset as u64),
            SEEK_CUR => SeekFrom::Current(offset),
            SEEK_END => SeekFrom::End(offset),
            _ => return Err(SPIFFS_ERR_INVALID),
        };

        match file.file.seek(seek_from) {
            Ok(pos) => Ok(pos as i64),
            Err(_) => Err(SPIFFS_ERR_IO),
        }
    }

    pub fn remove(&mut self, path: &str) -> Result<(), i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let path = path.trim_start_matches('/');
        let full_path = self.root.join(path);

        if !full_path.exists() {
            return Err(SPIFFS_ERR_NOT_FOUND);
        }

        if full_path.is_dir() {
            return Err(SPIFFS_ERR_IS_DIR);
        }

        match std::fs::remove_file(&full_path) {
            Ok(_) => Ok(()),
            Err(_) => Err(SPIFFS_ERR_IO),
        }
    }

    pub fn rename(&mut self, old_path: &str, new_path: &str) -> Result<(), i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let old_path = old_path.trim_start_matches('/');
        let new_path = new_path.trim_start_matches('/');

        let full_old_path = self.root.join(old_path);
        let full_new_path = self.root.join(new_path);

        if !full_old_path.exists() {
            return Err(SPIFFS_ERR_NOT_FOUND);
        }

        // Create parent directory for new path if needed
        if let Some(parent) = full_new_path.parent() {
            if let Err(_) = std::fs::create_dir_all(parent) {
                return Err(SPIFFS_ERR_IO);
            }
        }

        match std::fs::rename(&full_old_path, &full_new_path) {
            Ok(_) => Ok(()),
            Err(_) => Err(SPIFFS_ERR_IO),
        }
    }

    pub fn stat(&self, path: &str) -> Result<FileStat, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let path = path.trim_start_matches('/');
        let full_path = self.root.join(path);

        if !full_path.exists() {
            return Err(SPIFFS_ERR_NOT_FOUND);
        }

        let metadata = match std::fs::metadata(&full_path) {
            Ok(m) => m,
            Err(_) => return Err(SPIFFS_ERR_IO),
        };

        Ok(FileStat {
            size: metadata.len() as i32,
            is_dir: metadata.is_dir(),
        })
    }

    pub fn opendir(&mut self, path: &str) -> Result<i32, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let path = path.trim_start_matches('/');
        let full_path = if path.is_empty() {
            self.root.clone()
        } else {
            self.root.join(path)
        };

        if !full_path.exists() || !full_path.is_dir() {
            return Err(SPIFFS_ERR_NOT_DIR);
        }

        // Use a special file handle for directory
        let fd = self.next_fd;
        self.next_fd += 1;

        // Create a dummy file handle for directory
        self.files.insert(fd, SpiffsFile {
            fd,
            file: File::open(&full_path).map_err(|_| SPIFFS_ERR_IO)?,
            path: full_path,
            flags: O_RDONLY,
            dir_position: 0,
        });

        Ok(fd)
    }

    pub fn readdir(&mut self, fd: i32) -> Result<Option<DirEntry>, i32> {
        if !self.mounted {
            return Err(SPIFFS_ERR_NOT_MOUNTED);
        }

        let file_handle = self.files.get_mut(&fd)
            .ok_or(SPIFFS_ERR_BAD_FD)?;

        // Read directory entries
        let entries: Vec<_> = match std::fs::read_dir(&file_handle.path) {
            Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
            Err(_) => return Err(SPIFFS_ERR_IO),
        };

        if file_handle.dir_position >= entries.len() {
            return Ok(None); // End of directory
        }

        let entry = &entries[file_handle.dir_position];
        file_handle.dir_position += 1;

        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.path().is_dir();

        Ok(Some(DirEntry {
            name,
            is_dir,
            size: 0, // Could be filled if needed
        }))
    }

    pub fn closedir(&mut self, fd: i32) -> Result<(), i32> {
        self.close(fd)
    }

    pub fn info(&self) -> SpiffsInfo {
        let total_bytes = 1024 * 1024; // 1 MB virtual size
        let used_bytes = self.calculate_used_space();

        SpiffsInfo {
            total_bytes,
            used_bytes,
            block_size: 4096,
            page_size: 256,
        }
    }

    fn calculate_used_space(&self) -> usize {
        let mut total = 0;

        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        total += metadata.len() as usize;
                    }
                }
            }
        }

        total
    }
}

#[derive(Debug, Clone)]
pub struct FileStat {
    pub size: i32,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct SpiffsInfo {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub block_size: usize,
    pub page_size: usize,
}

lazy_static! {
    pub static ref SPIFFS_MANAGER: Arc<Mutex<SpiffsManager>> =
        Arc::new(Mutex::new(SpiffsManager::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spiffs_mount_unmount() {
        let mut mgr = SpiffsManager::new();
        assert!(!mgr.mounted);

        mgr.mount("/spiffs", "storage").unwrap();
        assert!(mgr.mounted);

        mgr.unmount().unwrap();
        assert!(!mgr.mounted);
    }

    #[test]
    fn test_spiffs_file_open_close() {
        let mut mgr = SpiffsManager::new();
        mgr.mount("/spiffs", "storage").unwrap();

        let fd = mgr.open("test.txt", O_CREAT | O_WRONLY, 0).unwrap();
        assert!(fd > 0);

        mgr.close(fd).unwrap();
    }

    #[test]
    fn test_spiffs_read_write() {
        let mut mgr = SpiffsManager::new();
        mgr.mount("/spiffs", "storage").unwrap();

        // Write data
        let fd = mgr.open("data.txt", O_CREAT | O_WRONLY | O_TRUNC, 0).unwrap();
        let data = b"Hello, SPIFFS!";
        let written = mgr.write(fd, data).unwrap();
        assert_eq!(written, data.len());
        mgr.close(fd).unwrap();

        // Read data back
        let fd = mgr.open("data.txt", O_RDONLY, 0).unwrap();
        let mut buf = vec![0u8; 100];
        let read = mgr.read(fd, &mut buf).unwrap();
        assert_eq!(read, data.len());
        assert_eq!(&buf[..read], data);
        mgr.close(fd).unwrap();
    }

    #[test]
    fn test_spiffs_seek() {
        let mut mgr = SpiffsManager::new();
        mgr.mount("/spiffs", "storage").unwrap();

        // Write data
        let fd = mgr.open("seek.txt", O_CREAT | O_RDWR | O_TRUNC, 0).unwrap();
        mgr.write(fd, b"0123456789").unwrap();

        // Seek and read
        mgr.lseek(fd, 5, SEEK_SET).unwrap();
        let mut buf = vec![0u8; 5];
        let read = mgr.read(fd, &mut buf).unwrap();
        assert_eq!(&buf[..read], b"56789");

        mgr.close(fd).unwrap();
    }

    #[test]
    fn test_spiffs_remove() {
        let mut mgr = SpiffsManager::new();
        mgr.mount("/spiffs", "storage").unwrap();

        // Create file
        let fd = mgr.open("remove.txt", O_CREAT | O_WRONLY, 0).unwrap();
        mgr.close(fd).unwrap();

        // Remove file
        mgr.remove("remove.txt").unwrap();

        // File should not exist
        assert_eq!(mgr.open("remove.txt", O_RDONLY, 0), Err(SPIFFS_ERR_NOT_FOUND));
    }

    #[test]
    fn test_spiffs_rename() {
        let mut mgr = SpiffsManager::new();
        mgr.mount("/spiffs", "storage").unwrap();

        // Create file
        let fd = mgr.open("old.txt", O_CREAT | O_WRONLY, 0).unwrap();
        mgr.write(fd, b"test").unwrap();
        mgr.close(fd).unwrap();

        // Rename
        mgr.rename("old.txt", "new.txt").unwrap();

        // Old file should not exist
        assert_eq!(mgr.open("old.txt", O_RDONLY, 0), Err(SPIFFS_ERR_NOT_FOUND));

        // New file should exist
        let fd = mgr.open("new.txt", O_RDONLY, 0).unwrap();
        mgr.close(fd).unwrap();
    }
}
