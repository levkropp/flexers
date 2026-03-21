/// VFS (Virtual File System) manager for ESP32 emulation
///
/// This module provides a central registry that routes file operations to appropriate
/// filesystem backends (SPIFFS, FAT, LittleFS, etc.) based on mount point path prefixes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

use super::spiffs_manager::SpiffsManager;

/// VFS error codes
pub const VFS_OK: i32 = 0;
pub const VFS_ERR_INVALID_ARG: i32 = -22;
pub const VFS_ERR_NO_MEM: i32 = -12;
pub const VFS_ERR_EXISTS: i32 = -17;
pub const VFS_ERR_NOT_FOUND: i32 = -2;

/// Filesystem types supported by VFS
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VfsFilesystemType {
    Spiffs,
    // Future: Fat, LittleFS, etc.
}

/// Backend reference for a mounted filesystem
#[derive(Debug, Clone)]
pub enum VfsBackend {
    Spiffs { manager_id: usize },
    // Future: Fat, LittleFS
}

/// Information about a VFS mount point
#[derive(Debug, Clone)]
pub struct VfsMountPoint {
    /// Mount path (e.g., "/spiffs", "/data")
    pub base_path: String,

    /// Partition label (e.g., "storage", "user_data")
    pub partition_label: String,

    /// Filesystem type
    pub fs_type: VfsFilesystemType,

    /// Backend manager reference
    pub backend: VfsBackend,
}

/// VFS manager - central registry for all mounted filesystems
pub struct VfsManager {
    /// Registered mount points: path prefix → mount point info
    mounts: HashMap<String, VfsMountPoint>,

    /// SPIFFS manager instances
    spiffs_instances: Vec<Arc<Mutex<SpiffsManager>>>,

    /// Initialized state
    initialized: bool,
}

impl VfsManager {
    pub fn new() -> Self {
        Self {
            mounts: HashMap::new(),
            spiffs_instances: Vec::new(),
            initialized: true,
        }
    }

    /// Register a SPIFFS filesystem at the specified mount point
    pub fn register_spiffs(&mut self, base_path: &str, partition_label: &str) -> Result<(), i32> {
        // Validate base_path
        if base_path.is_empty() {
            return Err(VFS_ERR_INVALID_ARG);
        }

        // Disallow root mount
        if base_path == "/" {
            return Err(VFS_ERR_INVALID_ARG);
        }

        // Check if already mounted
        if self.mounts.contains_key(base_path) {
            return Err(VFS_ERR_EXISTS);
        }

        // Create unique host root for this partition
        let host_root = format!("/tmp/esp32-spiffs/{}", partition_label);

        // Create new SPIFFS manager instance
        let mut manager = SpiffsManager::new_with_root(host_root.into());

        // Mount the filesystem
        manager.mount(base_path, partition_label)?;

        // Store in instances vector
        let manager_id = self.spiffs_instances.len();
        self.spiffs_instances.push(Arc::new(Mutex::new(manager)));

        // Register mount point
        self.mounts.insert(base_path.to_string(), VfsMountPoint {
            base_path: base_path.to_string(),
            partition_label: partition_label.to_string(),
            fs_type: VfsFilesystemType::Spiffs,
            backend: VfsBackend::Spiffs { manager_id },
        });

        Ok(())
    }

    /// Unregister a filesystem at the specified mount point
    pub fn unregister(&mut self, base_path: &str) -> Result<(), i32> {
        let mount_point = self.mounts.remove(base_path)
            .ok_or(VFS_ERR_NOT_FOUND)?;

        // Unmount the filesystem
        match &mount_point.backend {
            VfsBackend::Spiffs { manager_id } => {
                if let Some(manager) = self.spiffs_instances.get(*manager_id) {
                    manager.lock().unwrap().unmount()?;
                }
            }
        }

        Ok(())
    }

    /// Route a path to the appropriate mount point and extract the relative path
    ///
    /// Returns: (mount_point, relative_path)
    ///
    /// Example:
    /// - Input: "/spiffs/data/file.txt"
    /// - Mount: "/spiffs" → SPIFFS manager #1
    /// - Output: (mount_point, "data/file.txt")
    pub fn route_path(&self, path: &str) -> Option<(&VfsMountPoint, String)> {
        // Find longest matching prefix
        let mut best_match: Option<(&String, &VfsMountPoint)> = None;

        for (mount_path, mount_point) in &self.mounts {
            if path.starts_with(mount_path) {
                // Check for exact match or path separator after mount point
                let after_mount = &path[mount_path.len()..];
                if after_mount.is_empty() || after_mount.starts_with('/') {
                    if best_match.is_none() || mount_path.len() > best_match.unwrap().0.len() {
                        best_match = Some((mount_path, mount_point));
                    }
                }
            }
        }

        best_match.map(|(mount_path, mount_point)| {
            let relative_path = path.strip_prefix(mount_path)
                .unwrap_or("")
                .trim_start_matches('/');
            (mount_point, relative_path.to_string())
        })
    }

    /// Get a reference to a SPIFFS manager by ID
    pub fn get_spiffs_manager(&self, manager_id: usize) -> Option<Arc<Mutex<SpiffsManager>>> {
        self.spiffs_instances.get(manager_id).cloned()
    }

    /// Get list of all mount points
    pub fn get_mounts(&self) -> Vec<&VfsMountPoint> {
        self.mounts.values().collect()
    }

    /// Check if a path is mounted
    pub fn is_mounted(&self, base_path: &str) -> bool {
        self.mounts.contains_key(base_path)
    }
}

lazy_static! {
    pub static ref VFS_MANAGER: Arc<Mutex<VfsManager>> =
        Arc::new(Mutex::new(VfsManager::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_register_single_mount() {
        let mut vfs = VfsManager::new();
        assert!(vfs.register_spiffs("/spiffs", "storage").is_ok());
        assert!(vfs.is_mounted("/spiffs"));
    }

    #[test]
    fn test_vfs_register_multiple_mounts() {
        let mut vfs = VfsManager::new();
        assert!(vfs.register_spiffs("/spiffs", "storage").is_ok());
        assert!(vfs.register_spiffs("/data", "user_data").is_ok());
        assert!(vfs.register_spiffs("/web", "www").is_ok());

        assert!(vfs.is_mounted("/spiffs"));
        assert!(vfs.is_mounted("/data"));
        assert!(vfs.is_mounted("/web"));
    }

    #[test]
    fn test_vfs_route_exact_match() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();

        let (mount_point, rel_path) = vfs.route_path("/spiffs/file.txt").unwrap();
        assert_eq!(mount_point.base_path, "/spiffs");
        assert_eq!(rel_path, "file.txt");
    }

    #[test]
    fn test_vfs_route_nested_path() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();

        let (mount_point, rel_path) = vfs.route_path("/spiffs/data/subdir/file.txt").unwrap();
        assert_eq!(mount_point.base_path, "/spiffs");
        assert_eq!(rel_path, "data/subdir/file.txt");
    }

    #[test]
    fn test_vfs_route_longest_prefix() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "main").unwrap();
        vfs.register_spiffs("/spiffs/data", "sub").unwrap();

        // Should match longer prefix
        let (mount_point, rel_path) = vfs.route_path("/spiffs/data/file.txt").unwrap();
        assert_eq!(mount_point.base_path, "/spiffs/data");
        assert_eq!(mount_point.partition_label, "sub");
        assert_eq!(rel_path, "file.txt");

        // Should match shorter prefix
        let (mount_point, rel_path) = vfs.route_path("/spiffs/other/file.txt").unwrap();
        assert_eq!(mount_point.base_path, "/spiffs");
        assert_eq!(mount_point.partition_label, "main");
        assert_eq!(rel_path, "other/file.txt");
    }

    #[test]
    fn test_vfs_route_no_match() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();

        assert!(vfs.route_path("/other/file.txt").is_none());
        assert!(vfs.route_path("/data/file.txt").is_none());
    }

    #[test]
    fn test_vfs_unregister_mount() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();
        assert!(vfs.is_mounted("/spiffs"));

        vfs.unregister("/spiffs").unwrap();
        assert!(!vfs.is_mounted("/spiffs"));
    }

    #[test]
    fn test_vfs_duplicate_mount_fails() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();

        // Duplicate mount should fail
        assert_eq!(vfs.register_spiffs("/spiffs", "other"), Err(VFS_ERR_EXISTS));
    }

    #[test]
    fn test_vfs_partition_label_tracking() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();
        vfs.register_spiffs("/data", "user_data").unwrap();

        let (mount_point, _) = vfs.route_path("/spiffs/file.txt").unwrap();
        assert_eq!(mount_point.partition_label, "storage");

        let (mount_point, _) = vfs.route_path("/data/file.txt").unwrap();
        assert_eq!(mount_point.partition_label, "user_data");
    }

    #[test]
    fn test_vfs_root_mount_disallowed() {
        let mut vfs = VfsManager::new();
        assert_eq!(vfs.register_spiffs("/", "root"), Err(VFS_ERR_INVALID_ARG));
    }

    #[test]
    fn test_vfs_empty_path_invalid() {
        let mut vfs = VfsManager::new();
        assert_eq!(vfs.register_spiffs("", "empty"), Err(VFS_ERR_INVALID_ARG));
    }

    #[test]
    fn test_vfs_get_mounts() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();
        vfs.register_spiffs("/data", "user_data").unwrap();

        let mounts = vfs.get_mounts();
        assert_eq!(mounts.len(), 2);
    }

    #[test]
    fn test_vfs_route_with_trailing_slash() {
        let mut vfs = VfsManager::new();
        vfs.register_spiffs("/spiffs", "storage").unwrap();

        let (mount_point, rel_path) = vfs.route_path("/spiffs/").unwrap();
        assert_eq!(mount_point.base_path, "/spiffs");
        assert_eq!(rel_path, "");
    }
}
