---
layout: post
title: "Phase 11A Complete: NVS (Non-Volatile Storage)"
date: 2026-03-20 15:00:00 -0000
categories: [storage, nvs]
author: Flexers Team
excerpt: "Full ESP-IDF NVS implementation with namespaces, type-safe storage, and JSON persistence."
---

# Phase 11A: NVS (Non-Volatile Storage) - Complete

**Status**: ✅ Complete
**Tests**: 134 passing (8 new NVS tests)

## Summary

Phase 11A implements ESP-IDF's NVS (Non-Volatile Storage) system, providing persistent key-value storage with namespaces and full type safety.

## Features Implemented

### Core NVS API
```c
// Initialize NVS
esp_err_t nvs_flash_init(void);

// Open namespace
nvs_handle_t handle;
nvs_open("storage", NVS_READWRITE, &handle);

// Store values
nvs_set_u32(handle, "counter", 42);
nvs_set_str(handle, "name", "ESP32");
nvs_set_blob(handle, "data", buffer, size);

// Read values
uint32_t value;
nvs_get_u32(handle, "counter", &value);

// Commit and close
nvs_commit(handle);
nvs_close(handle);
```

### Supported Data Types
- ✅ **Integers**: u8, i8, u16, i16, u32, i32, u64, i64
- ✅ **Strings**: Null-terminated C strings
- ✅ **Blobs**: Binary data of arbitrary size

### Namespace Isolation
- Multiple namespaces for organization
- Keys isolated per namespace
- No cross-namespace interference

### Persistence
- JSON file backend: `test_nvs.json`
- Automatic save on commit
- Load on initialization
- Easy inspection and debugging

## Implementation

### NVS Manager (`nvs_manager.rs`)
- **Namespace tracking**: HashMap per namespace
- **Handle management**: Unique IDs for open namespaces
- **Type-safe storage**: Separate methods per type
- **Persistence**: JSON serialization/deserialization

### Example JSON Output
```json
{
  "storage": {
    "counter": {"type": "u32", "value": 42},
    "name": {"type": "str", "value": "ESP32"},
    "enabled": {"type": "u8", "value": 1}
  },
  "config": {
    "timeout": {"type": "u32", "value": 5000}
  }
}
```

## Test Results

```
✅ test_nvs_init - Initialize NVS system
✅ test_nvs_open_close - Open/close namespaces
✅ test_nvs_set_get_u32 - Store/retrieve integers
✅ test_nvs_set_get_string - Store/retrieve strings
✅ test_nvs_blob - Store/retrieve binary data
✅ test_nvs_erase_key - Delete keys
✅ test_nvs_namespaces_isolated - Namespace isolation
✅ test_nvs_readonly_mode - Read-only access
```

## Real-World Usage

### WiFi Credentials
```c
nvs_handle_t wifi;
nvs_open("wifi", NVS_READWRITE, &wifi);
nvs_set_str(wifi, "ssid", "MyNetwork");
nvs_set_str(wifi, "password", "secret123");
nvs_commit(wifi);
nvs_close(wifi);
```

### Configuration Storage
```c
nvs_handle_t config;
nvs_open("config", NVS_READWRITE, &config);
nvs_set_u32(config, "baud_rate", 115200);
nvs_set_u8(config, "log_level", 3);
nvs_commit(config);
nvs_close(config);
```

### Binary Data
```c
uint8_t calibration[128];
nvs_handle_t cal;
nvs_open("calibration", NVS_READWRITE, &cal);
nvs_set_blob(cal, "sensor_cal", calibration, 128);
nvs_commit(cal);
nvs_close(cal);
```

## Impact

NVS enables:
- ✅ **WiFi credentials** - Store and retrieve network settings
- ✅ **User preferences** - Save configuration across reboots
- ✅ **Calibration data** - Store sensor calibration
- ✅ **Application state** - Persist runtime state

## Design Decisions

### JSON Backend
**Why**: Easy inspection, human-readable, simple debugging
**Alternative**: Binary format (more compact but opaque)
**Trade-off**: Slightly larger files, much easier development

### Namespace Isolation
**Why**: Matches ESP-IDF behavior, prevents key collisions
**Benefit**: Different subsystems can use same key names

### Type Safety
**Why**: Prevents type confusion bugs
**Benefit**: Read operations verify type matches what was stored

## Performance

- **Read**: ~1μs (in-memory HashMap lookup)
- **Write**: ~1μs (in-memory update)
- **Commit**: ~1ms (JSON serialization to file)
- **Init**: ~5ms (JSON deserialization from file)

## Next Steps

Phase 11A provides the foundation for persistent storage. Next:
- **Phase 11B**: SPIFFS filesystem
- **Phase 11C**: VFS layer for multi-partition support

---

**Implementation Date**: March 20, 2026
**Lines of Code**: ~400 new
**Tests Added**: 8 comprehensive NVS tests
**Files Created**: `nvs_manager.rs`, `nvs.rs`
