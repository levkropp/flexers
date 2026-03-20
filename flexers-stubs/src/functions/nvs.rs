/// NVS (Non-Volatile Storage) ROM stubs for ESP32 emulation
///
/// This module implements the ESP32 NVS API as ROM stubs, bridging
/// firmware calls to the NvsStorage manager.

use crate::handler::RomStubHandler;
use flexers_core::cpu::XtensaCpu;

use super::nvs_manager::{
    NVS_STORAGE, NvsOpenMode, ESP_OK, ESP_ERR_NVS_BASE,
    ESP_ERR_NVS_NOT_FOUND, ESP_ERR_NVS_INVALID_HANDLE, ESP_ERR_NVS_READONLY,
    ESP_ERR_NVS_INVALID_NAME, ESP_ERR_NVS_INVALID_LENGTH,
};

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

/// Helper: Read u64 from memory (little-endian, two u32 reads)
fn read_u64(cpu: &XtensaCpu, ptr: u32) -> u64 {
    let low = cpu.memory().read_u32(ptr) as u64;
    let high = cpu.memory().read_u32(ptr + 4) as u64;
    (high << 32) | low
}

/// Helper: Write u64 to memory (little-endian, two u32 writes)
fn write_u64(cpu: &mut XtensaCpu, ptr: u32, value: u64) {
    let low = (value & 0xFFFFFFFF) as u32;
    let high = (value >> 32) as u32;
    cpu.memory().write_u32(ptr, low);
    cpu.memory().write_u32(ptr + 4, high);
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
// NVS Initialization
//=============================================================================

/// nvs_flash_init() - Initialize default NVS partition
pub struct NvsFlashInit;

impl RomStubHandler for NvsFlashInit {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.init() {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_flash_init"
    }
}

/// nvs_flash_init_partition() - Initialize specific NVS partition
pub struct NvsFlashInitPartition;

impl RomStubHandler for NvsFlashInitPartition {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _partition_label = read_c_string(cpu, cpu.get_ar(2), 64);

        // For now, treat all partitions the same
        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.init() {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_flash_init_partition"
    }
}

/// nvs_flash_deinit() - Deinitialize NVS partition
pub struct NvsFlashDeinit;

impl RomStubHandler for NvsFlashDeinit {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.deinit() {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_flash_deinit"
    }
}

/// nvs_flash_erase() - Erase all NVS data
pub struct NvsFlashErase;

impl RomStubHandler for NvsFlashErase {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.erase() {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_flash_erase"
    }
}

/// nvs_flash_erase_partition() - Erase specific NVS partition
pub struct NvsFlashErasePartition;

impl RomStubHandler for NvsFlashErasePartition {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _partition_label = read_c_string(cpu, cpu.get_ar(2), 64);

        // For now, treat all partitions the same
        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.erase() {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_flash_erase_partition"
    }
}

//=============================================================================
// NVS Handle Management
//=============================================================================

/// nvs_open() - Open namespace and return handle
pub struct NvsOpen;

impl RomStubHandler for NvsOpen {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let namespace_ptr = cpu.get_ar(2);
        let mode = cpu.get_ar(3);
        let out_handle_ptr = cpu.get_ar(4);

        let namespace = read_c_string(cpu, namespace_ptr, 64);
        let open_mode = NvsOpenMode::from_u32(mode);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.open(&namespace, open_mode) {
                    Ok(handle) => {
                        cpu.memory().write_u32(out_handle_ptr, handle);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_open"
    }
}

/// nvs_open_from_partition() - Open namespace from specific partition
pub struct NvsOpenFromPartition;

impl RomStubHandler for NvsOpenFromPartition {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _partition_label = read_c_string(cpu, cpu.get_ar(2), 64);
        let namespace_ptr = cpu.get_ar(3);
        let mode = cpu.get_ar(4);
        let out_handle_ptr = cpu.get_ar(5);

        let namespace = read_c_string(cpu, namespace_ptr, 64);
        let open_mode = NvsOpenMode::from_u32(mode);

        // For now, ignore partition label
        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.open(&namespace, open_mode) {
                    Ok(handle) => {
                        cpu.memory().write_u32(out_handle_ptr, handle);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_open_from_partition"
    }
}

/// nvs_close() - Close handle
pub struct NvsClose;

impl RomStubHandler for NvsClose {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.close(handle) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_close"
    }
}

//=============================================================================
// Integer Get/Set Operations
//=============================================================================

/// nvs_set_u8() - Set uint8_t value
pub struct NvsSetU8;

impl RomStubHandler for NvsSetU8 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value = cpu.get_ar(4) as u8;

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_u8(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_u8"
    }
}

/// nvs_get_u8() - Get uint8_t value
pub struct NvsGetU8;

impl RomStubHandler for NvsGetU8 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_u8(handle, &key) {
                    Ok(value) => {
                        cpu.memory().write_u8(out_value_ptr, value);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_u8"
    }
}

/// nvs_set_i8() - Set int8_t value
pub struct NvsSetI8;

impl RomStubHandler for NvsSetI8 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value = cpu.get_ar(4) as i8;

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_i8(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_i8"
    }
}

/// nvs_get_i8() - Get int8_t value
pub struct NvsGetI8;

impl RomStubHandler for NvsGetI8 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_i8(handle, &key) {
                    Ok(value) => {
                        cpu.memory().write_u8(out_value_ptr, value as u8);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_i8"
    }
}

/// nvs_set_u16() - Set uint16_t value
pub struct NvsSetU16;

impl RomStubHandler for NvsSetU16 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value = cpu.get_ar(4) as u16;

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_u16(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_u16"
    }
}

/// nvs_get_u16() - Get uint16_t value
pub struct NvsGetU16;

impl RomStubHandler for NvsGetU16 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_u16(handle, &key) {
                    Ok(value) => {
                        cpu.memory().write_u16(out_value_ptr, value);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_u16"
    }
}

/// nvs_set_i16() - Set int16_t value
pub struct NvsSetI16;

impl RomStubHandler for NvsSetI16 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value = cpu.get_ar(4) as i16;

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_i16(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_i16"
    }
}

/// nvs_get_i16() - Get int16_t value
pub struct NvsGetI16;

impl RomStubHandler for NvsGetI16 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_i16(handle, &key) {
                    Ok(value) => {
                        cpu.memory().write_u16(out_value_ptr, value as u16);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_i16"
    }
}

/// nvs_set_u32() - Set uint32_t value
pub struct NvsSetU32;

impl RomStubHandler for NvsSetU32 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_u32(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_u32"
    }
}

/// nvs_get_u32() - Get uint32_t value
pub struct NvsGetU32;

impl RomStubHandler for NvsGetU32 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_u32(handle, &key) {
                    Ok(value) => {
                        cpu.memory().write_u32(out_value_ptr, value);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_u32"
    }
}

/// nvs_set_i32() - Set int32_t value
pub struct NvsSetI32;

impl RomStubHandler for NvsSetI32 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value = cpu.get_ar(4) as i32;

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_i32(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_i32"
    }
}

/// nvs_get_i32() - Get int32_t value
pub struct NvsGetI32;

impl RomStubHandler for NvsGetI32 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_i32(handle, &key) {
                    Ok(value) => {
                        cpu.memory().write_u32(out_value_ptr, value as u32);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_i32"
    }
}

/// nvs_set_u64() - Set uint64_t value
pub struct NvsSetU64;

impl RomStubHandler for NvsSetU64 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);
        let value = read_u64(cpu, value_ptr);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_u64(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_u64"
    }
}

/// nvs_get_u64() - Get uint64_t value
pub struct NvsGetU64;

impl RomStubHandler for NvsGetU64 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_u64(handle, &key) {
                    Ok(value) => {
                        write_u64(cpu, out_value_ptr, value);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_u64"
    }
}

/// nvs_set_i64() - Set int64_t value
pub struct NvsSetI64;

impl RomStubHandler for NvsSetI64 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);
        let value = read_u64(cpu, value_ptr) as i64;

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_i64(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_i64"
    }
}

/// nvs_get_i64() - Get int64_t value
pub struct NvsGetI64;

impl RomStubHandler for NvsGetI64 {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_i64(handle, &key) {
                    Ok(value) => {
                        write_u64(cpu, out_value_ptr, value as u64);
                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_i64"
    }
}

//=============================================================================
// String Operations
//=============================================================================

/// nvs_set_str() - Set string value
pub struct NvsSetStr;

impl RomStubHandler for NvsSetStr {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value_ptr = cpu.get_ar(4);

        let key = read_c_string(cpu, key_ptr, 64);
        let value = read_c_string(cpu, value_ptr, 4096);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_str(handle, &key, value) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_str"
    }
}

/// nvs_get_str() - Get string value
pub struct NvsGetStr;

impl RomStubHandler for NvsGetStr {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);
        let length_ptr = cpu.get_ar(5);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_str(handle, &key) {
                    Ok(value) => {
                        // If out_value_ptr is NULL, just return required length
                        if out_value_ptr == 0 {
                            if length_ptr != 0 {
                                cpu.memory().write_u32(length_ptr, (value.len() + 1) as u32);
                            }
                            return ESP_OK as u32;
                        }

                        // Get buffer size
                        let buf_size = if length_ptr != 0 {
                            cpu.memory().read_u32(length_ptr) as usize
                        } else {
                            4096
                        };

                        // Write string to buffer
                        let written = write_c_string(cpu, out_value_ptr, &value, buf_size);

                        // Update length
                        if length_ptr != 0 {
                            cpu.memory().write_u32(length_ptr, written as u32);
                        }

                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_str"
    }
}

//=============================================================================
// Blob Operations
//=============================================================================

/// nvs_set_blob() - Set binary blob
pub struct NvsSetBlob;

impl RomStubHandler for NvsSetBlob {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let value_ptr = cpu.get_ar(4);
        let length = cpu.get_ar(5) as usize;

        let key = read_c_string(cpu, key_ptr, 64);

        // Read blob data from memory
        let mut blob = vec![0u8; length];
        for i in 0..length {
            blob[i] = cpu.memory().read_u8(value_ptr + i as u32);
        }

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.set_blob(handle, &key, blob) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_set_blob"
    }
}

/// nvs_get_blob() - Get binary blob
pub struct NvsGetBlob;

impl RomStubHandler for NvsGetBlob {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);
        let out_value_ptr = cpu.get_ar(4);
        let length_ptr = cpu.get_ar(5);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(storage) => {
                match storage.get_blob(handle, &key) {
                    Ok(blob) => {
                        // If out_value_ptr is NULL, just return required length
                        if out_value_ptr == 0 {
                            if length_ptr != 0 {
                                cpu.memory().write_u32(length_ptr, blob.len() as u32);
                            }
                            return ESP_OK as u32;
                        }

                        // Get buffer size
                        let buf_size = if length_ptr != 0 {
                            cpu.memory().read_u32(length_ptr) as usize
                        } else {
                            blob.len()
                        };

                        // Write blob to buffer
                        let copy_size = blob.len().min(buf_size);
                        for i in 0..copy_size {
                            cpu.memory().write_u8(out_value_ptr + i as u32, blob[i]);
                        }

                        // Update length
                        if length_ptr != 0 {
                            cpu.memory().write_u32(length_ptr, copy_size as u32);
                        }

                        ESP_OK as u32
                    }
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_get_blob"
    }
}

//=============================================================================
// Commit & Erase Operations
//=============================================================================

/// nvs_commit() - Persist changes to flash
pub struct NvsCommit;

impl RomStubHandler for NvsCommit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.commit(handle) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_commit"
    }
}

/// nvs_erase_key() - Delete key from namespace
pub struct NvsEraseKey;

impl RomStubHandler for NvsEraseKey {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);
        let key_ptr = cpu.get_ar(3);

        let key = read_c_string(cpu, key_ptr, 64);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.erase_key(handle, &key) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_erase_key"
    }
}

/// nvs_erase_all() - Erase all keys in namespace
pub struct NvsEraseAll;

impl RomStubHandler for NvsEraseAll {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let handle = cpu.get_ar(2);

        match NVS_STORAGE.lock() {
            Ok(mut storage) => {
                match storage.erase_all(handle) {
                    Ok(_) => ESP_OK as u32,
                    Err(e) => e as u32,
                }
            }
            Err(_) => ESP_ERR_NVS_BASE as u32,
        }
    }

    fn name(&self) -> &str {
        "nvs_erase_all"
    }
}
