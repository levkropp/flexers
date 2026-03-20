/// NVS (Non-Volatile Storage) manager for ESP32 emulation
///
/// This module provides an in-memory key-value storage system that emulates
/// the ESP32's NVS (Non-Volatile Storage) functionality. It supports:
/// - Multiple isolated namespaces
/// - Various data types (u8, i8, u16, i16, u32, i32, u64, i64, strings, blobs)
/// - Handle-based access with automatic lifecycle management
/// - Optional persistence to host filesystem

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

/// NVS error codes (matching ESP-IDF)
pub const ESP_OK: i32 = 0;
pub const ESP_ERR_NVS_BASE: i32 = 0x1100;
pub const ESP_ERR_NVS_NOT_INITIALIZED: i32 = ESP_ERR_NVS_BASE + 1;
pub const ESP_ERR_NVS_NOT_FOUND: i32 = ESP_ERR_NVS_BASE + 2;
pub const ESP_ERR_NVS_INVALID_HANDLE: i32 = ESP_ERR_NVS_BASE + 6;
pub const ESP_ERR_NVS_READONLY: i32 = ESP_ERR_NVS_BASE + 7;
pub const ESP_ERR_NVS_INVALID_NAME: i32 = ESP_ERR_NVS_BASE + 8;
pub const ESP_ERR_NVS_INVALID_LENGTH: i32 = ESP_ERR_NVS_BASE + 9;
pub const ESP_ERR_NVS_NO_FREE_PAGES: i32 = ESP_ERR_NVS_BASE + 10;

/// NVS open mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NvsOpenMode {
    ReadOnly,
    ReadWrite,
}

impl NvsOpenMode {
    pub fn from_u32(mode: u32) -> Self {
        if mode == 1 {
            NvsOpenMode::ReadWrite
        } else {
            NvsOpenMode::ReadOnly
        }
    }
}

/// NVS value types
#[derive(Debug, Clone)]
pub enum NvsValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    String(String),
    Blob(Vec<u8>),
}

impl NvsValue {
    pub fn as_u8(&self) -> Result<u8, i32> {
        match self {
            NvsValue::U8(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_i8(&self) -> Result<i8, i32> {
        match self {
            NvsValue::I8(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_u16(&self) -> Result<u16, i32> {
        match self {
            NvsValue::U16(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_i16(&self) -> Result<i16, i32> {
        match self {
            NvsValue::I16(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_u32(&self) -> Result<u32, i32> {
        match self {
            NvsValue::U32(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_i32(&self) -> Result<i32, i32> {
        match self {
            NvsValue::I32(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_u64(&self) -> Result<u64, i32> {
        match self {
            NvsValue::U64(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_i64(&self) -> Result<i64, i32> {
        match self {
            NvsValue::I64(v) => Ok(*v),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_string(&self) -> Result<&str, i32> {
        match self {
            NvsValue::String(s) => Ok(s.as_str()),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }

    pub fn as_blob(&self) -> Result<&[u8], i32> {
        match self {
            NvsValue::Blob(b) => Ok(b.as_slice()),
            _ => Err(ESP_ERR_NVS_INVALID_LENGTH),
        }
    }
}

/// NVS partition (namespace)
#[derive(Debug)]
pub struct NvsPartition {
    /// Namespace name
    name: String,

    /// Key-value store
    data: HashMap<String, NvsValue>,

    /// Dirty flag (changes not yet committed)
    dirty: bool,
}

impl NvsPartition {
    pub fn new(name: String) -> Self {
        Self {
            name,
            data: HashMap::new(),
            dirty: false,
        }
    }

    pub fn set_u8(&mut self, key: &str, value: u8) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::U8(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_u8(&self, key: &str) -> Result<u8, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_u8()
    }

    pub fn set_i8(&mut self, key: &str, value: i8) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::I8(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_i8(&self, key: &str) -> Result<i8, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_i8()
    }

    pub fn set_u16(&mut self, key: &str, value: u16) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::U16(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_u16(&self, key: &str) -> Result<u16, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_u16()
    }

    pub fn set_i16(&mut self, key: &str, value: i16) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::I16(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_i16(&self, key: &str) -> Result<i16, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_i16()
    }

    pub fn set_u32(&mut self, key: &str, value: u32) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::U32(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_u32(&self, key: &str) -> Result<u32, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_u32()
    }

    pub fn set_i32(&mut self, key: &str, value: i32) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::I32(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_i32(&self, key: &str) -> Result<i32, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_i32()
    }

    pub fn set_u64(&mut self, key: &str, value: u64) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::U64(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_u64(&self, key: &str) -> Result<u64, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_u64()
    }

    pub fn set_i64(&mut self, key: &str, value: i64) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::I64(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_i64(&self, key: &str) -> Result<i64, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_i64()
    }

    pub fn set_str(&mut self, key: &str, value: String) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::String(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_str(&self, key: &str) -> Result<&str, i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_string()
    }

    pub fn set_blob(&mut self, key: &str, value: Vec<u8>) -> Result<(), i32> {
        self.data.insert(key.to_string(), NvsValue::Blob(value));
        self.dirty = true;
        Ok(())
    }

    pub fn get_blob(&self, key: &str) -> Result<&[u8], i32> {
        self.data
            .get(key)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)?
            .as_blob()
    }

    pub fn erase_key(&mut self, key: &str) -> Result<(), i32> {
        self.data.remove(key).ok_or(ESP_ERR_NVS_NOT_FOUND)?;
        self.dirty = true;
        Ok(())
    }

    pub fn erase_all(&mut self) -> Result<(), i32> {
        self.data.clear();
        self.dirty = true;
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), i32> {
        // In Phase 11A, we just clear the dirty flag
        // In Phase 11B, we could persist to host filesystem
        self.dirty = false;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

/// NVS handle information
#[derive(Debug)]
pub struct NvsHandle {
    /// Handle ID
    handle_id: u32,

    /// Namespace name
    namespace: String,

    /// Open mode
    mode: NvsOpenMode,
}

/// NVS Storage Manager
pub struct NvsStorage {
    /// Initialized state
    initialized: bool,

    /// Partitions: namespace → partition
    partitions: HashMap<String, NvsPartition>,

    /// Open handles: handle_id → handle info
    handles: HashMap<u32, NvsHandle>,

    /// Next handle ID
    next_handle_id: u32,
}

impl NvsStorage {
    pub fn new() -> Self {
        Self {
            initialized: false,
            partitions: HashMap::new(),
            handles: HashMap::new(),
            next_handle_id: 1,
        }
    }

    pub fn init(&mut self) -> Result<(), i32> {
        self.initialized = true;
        Ok(())
    }

    pub fn deinit(&mut self) -> Result<(), i32> {
        // Commit all dirty partitions
        for partition in self.partitions.values_mut() {
            if partition.is_dirty() {
                partition.commit()?;
            }
        }

        // Close all handles
        self.handles.clear();

        // Clear partitions
        self.partitions.clear();

        self.initialized = false;
        Ok(())
    }

    pub fn erase(&mut self) -> Result<(), i32> {
        // Close all handles
        self.handles.clear();

        // Clear all partitions
        self.partitions.clear();

        Ok(())
    }

    pub fn open(&mut self, namespace: &str, mode: NvsOpenMode) -> Result<u32, i32> {
        if !self.initialized {
            return Err(ESP_ERR_NVS_NOT_INITIALIZED);
        }

        if namespace.is_empty() || namespace.len() > 15 {
            return Err(ESP_ERR_NVS_INVALID_NAME);
        }

        // Create partition if it doesn't exist
        if !self.partitions.contains_key(namespace) {
            self.partitions.insert(
                namespace.to_string(),
                NvsPartition::new(namespace.to_string()),
            );
        }

        // Create handle
        let handle_id = self.next_handle_id;
        self.next_handle_id += 1;

        self.handles.insert(
            handle_id,
            NvsHandle {
                handle_id,
                namespace: namespace.to_string(),
                mode,
            },
        );

        Ok(handle_id)
    }

    pub fn close(&mut self, handle_id: u32) -> Result<(), i32> {
        let handle = self.handles.remove(&handle_id)
            .ok_or(ESP_ERR_NVS_INVALID_HANDLE)?;

        // Auto-commit if dirty
        if let Some(partition) = self.partitions.get_mut(&handle.namespace) {
            if partition.is_dirty() {
                partition.commit()?;
            }
        }

        Ok(())
    }

    fn get_partition(&self, handle_id: u32) -> Result<&NvsPartition, i32> {
        let handle = self.handles.get(&handle_id)
            .ok_or(ESP_ERR_NVS_INVALID_HANDLE)?;

        self.partitions.get(&handle.namespace)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)
    }

    fn get_partition_mut(&mut self, handle_id: u32) -> Result<&mut NvsPartition, i32> {
        let handle = self.handles.get(&handle_id)
            .ok_or(ESP_ERR_NVS_INVALID_HANDLE)?;

        let namespace = handle.namespace.clone();
        self.partitions.get_mut(&namespace)
            .ok_or(ESP_ERR_NVS_NOT_FOUND)
    }

    fn check_write_permission(&self, handle_id: u32) -> Result<(), i32> {
        let handle = self.handles.get(&handle_id)
            .ok_or(ESP_ERR_NVS_INVALID_HANDLE)?;

        if handle.mode == NvsOpenMode::ReadOnly {
            return Err(ESP_ERR_NVS_READONLY);
        }

        Ok(())
    }

    pub fn set_u8(&mut self, handle_id: u32, key: &str, value: u8) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_u8(key, value)
    }

    pub fn get_u8(&self, handle_id: u32, key: &str) -> Result<u8, i32> {
        self.get_partition(handle_id)?.get_u8(key)
    }

    pub fn set_i8(&mut self, handle_id: u32, key: &str, value: i8) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_i8(key, value)
    }

    pub fn get_i8(&self, handle_id: u32, key: &str) -> Result<i8, i32> {
        self.get_partition(handle_id)?.get_i8(key)
    }

    pub fn set_u16(&mut self, handle_id: u32, key: &str, value: u16) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_u16(key, value)
    }

    pub fn get_u16(&self, handle_id: u32, key: &str) -> Result<u16, i32> {
        self.get_partition(handle_id)?.get_u16(key)
    }

    pub fn set_i16(&mut self, handle_id: u32, key: &str, value: i16) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_i16(key, value)
    }

    pub fn get_i16(&self, handle_id: u32, key: &str) -> Result<i16, i32> {
        self.get_partition(handle_id)?.get_i16(key)
    }

    pub fn set_u32(&mut self, handle_id: u32, key: &str, value: u32) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_u32(key, value)
    }

    pub fn get_u32(&self, handle_id: u32, key: &str) -> Result<u32, i32> {
        self.get_partition(handle_id)?.get_u32(key)
    }

    pub fn set_i32(&mut self, handle_id: u32, key: &str, value: i32) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_i32(key, value)
    }

    pub fn get_i32(&self, handle_id: u32, key: &str) -> Result<i32, i32> {
        self.get_partition(handle_id)?.get_i32(key)
    }

    pub fn set_u64(&mut self, handle_id: u32, key: &str, value: u64) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_u64(key, value)
    }

    pub fn get_u64(&self, handle_id: u32, key: &str) -> Result<u64, i32> {
        self.get_partition(handle_id)?.get_u64(key)
    }

    pub fn set_i64(&mut self, handle_id: u32, key: &str, value: i64) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_i64(key, value)
    }

    pub fn get_i64(&self, handle_id: u32, key: &str) -> Result<i64, i32> {
        self.get_partition(handle_id)?.get_i64(key)
    }

    pub fn set_str(&mut self, handle_id: u32, key: &str, value: String) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_str(key, value)
    }

    pub fn get_str(&self, handle_id: u32, key: &str) -> Result<String, i32> {
        Ok(self.get_partition(handle_id)?.get_str(key)?.to_string())
    }

    pub fn set_blob(&mut self, handle_id: u32, key: &str, value: Vec<u8>) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.set_blob(key, value)
    }

    pub fn get_blob(&self, handle_id: u32, key: &str) -> Result<Vec<u8>, i32> {
        Ok(self.get_partition(handle_id)?.get_blob(key)?.to_vec())
    }

    pub fn erase_key(&mut self, handle_id: u32, key: &str) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.erase_key(key)
    }

    pub fn erase_all(&mut self, handle_id: u32) -> Result<(), i32> {
        self.check_write_permission(handle_id)?;
        self.get_partition_mut(handle_id)?.erase_all()
    }

    pub fn commit(&mut self, handle_id: u32) -> Result<(), i32> {
        self.get_partition_mut(handle_id)?.commit()
    }
}

lazy_static! {
    pub static ref NVS_STORAGE: Arc<Mutex<NvsStorage>> =
        Arc::new(Mutex::new(NvsStorage::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nvs_init() {
        let mut storage = NvsStorage::new();
        assert!(!storage.initialized);

        assert_eq!(storage.init(), Ok(()));
        assert!(storage.initialized);
    }

    #[test]
    fn test_nvs_open_close() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle = storage.open("test", NvsOpenMode::ReadWrite).unwrap();
        assert!(handle > 0);

        assert_eq!(storage.close(handle), Ok(()));
    }

    #[test]
    fn test_nvs_set_get_u32() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle = storage.open("test", NvsOpenMode::ReadWrite).unwrap();

        storage.set_u32(handle, "counter", 42).unwrap();
        let value = storage.get_u32(handle, "counter").unwrap();
        assert_eq!(value, 42);

        storage.close(handle).unwrap();
    }

    #[test]
    fn test_nvs_set_get_string() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle = storage.open("wifi", NvsOpenMode::ReadWrite).unwrap();

        storage.set_str(handle, "ssid", "MyNetwork".to_string()).unwrap();
        let value = storage.get_str(handle, "ssid").unwrap();
        assert_eq!(value, "MyNetwork");

        storage.close(handle).unwrap();
    }

    #[test]
    fn test_nvs_namespaces_isolated() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle1 = storage.open("wifi", NvsOpenMode::ReadWrite).unwrap();
        let handle2 = storage.open("app", NvsOpenMode::ReadWrite).unwrap();

        storage.set_u32(handle1, "config", 100).unwrap();
        storage.set_u32(handle2, "config", 200).unwrap();

        assert_eq!(storage.get_u32(handle1, "config").unwrap(), 100);
        assert_eq!(storage.get_u32(handle2, "config").unwrap(), 200);

        storage.close(handle1).unwrap();
        storage.close(handle2).unwrap();
    }

    #[test]
    fn test_nvs_readonly_mode() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle_rw = storage.open("test", NvsOpenMode::ReadWrite).unwrap();
        storage.set_u32(handle_rw, "value", 123).unwrap();
        storage.close(handle_rw).unwrap();

        let handle_ro = storage.open("test", NvsOpenMode::ReadOnly).unwrap();
        assert_eq!(storage.get_u32(handle_ro, "value").unwrap(), 123);
        assert_eq!(storage.set_u32(handle_ro, "value", 456), Err(ESP_ERR_NVS_READONLY));

        storage.close(handle_ro).unwrap();
    }

    #[test]
    fn test_nvs_erase_key() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle = storage.open("test", NvsOpenMode::ReadWrite).unwrap();

        storage.set_u32(handle, "temp", 99).unwrap();
        assert_eq!(storage.get_u32(handle, "temp").unwrap(), 99);

        storage.erase_key(handle, "temp").unwrap();
        assert_eq!(storage.get_u32(handle, "temp"), Err(ESP_ERR_NVS_NOT_FOUND));

        storage.close(handle).unwrap();
    }

    #[test]
    fn test_nvs_blob() {
        let mut storage = NvsStorage::new();
        storage.init().unwrap();

        let handle = storage.open("test", NvsOpenMode::ReadWrite).unwrap();

        let data = vec![1, 2, 3, 4, 5];
        storage.set_blob(handle, "binary", data.clone()).unwrap();

        let retrieved = storage.get_blob(handle, "binary").unwrap();
        assert_eq!(retrieved, data);

        storage.close(handle).unwrap();
    }
}
