---
layout: post
title: "Phase 4 Complete: Flash SPI Emulation"
date: 2026-03-20
author: Flexers Team
tags: [development, flash, spi, peripherals]
---

# Phase 4 Complete: Flash SPI Emulation

We're excited to announce the completion of **Phase 4: Flash SPI Emulation**! The ESP32 emulator now has full SPI flash controller support, enabling realistic flash memory operations.

## What's New

### SPI Flash Controller

The heart of Phase 4 is the new **SPI Flash Controller** (`flexers-periph/src/spi_flash.rs`), which provides:

- **Complete register interface**: SPI_CMD, SPI_ADDR, SPI_CTRL, and W0-W15 data buffer
- **6 flash commands**: READ, WRITE, ERASE (sector/32K/64K), WRITE_ENABLE, READ_STATUS
- **Write protection**: Enforces WRITE_ENABLE latch before any write/erase operation
- **Realistic flash behavior**: Write operations can only change bits from 1→0 (cannot change 0→1)
- **64-byte transfers**: Full 16-word (64-byte) data buffer support

### Flash Commands Implemented

```rust
// Read up to 64 bytes from flash
mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x1000);
mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0003); // READ cmd + execute

// Write with protection
mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x80000006); // WRITE_ENABLE
mem.write_u32(SPI1_BASE + SPI_W0_REG, 0xDEADBEEF);
mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x1000);
mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x80000002); // WRITE cmd

// Erase sector (4KB)
mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x80000006); // WRITE_ENABLE
mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x0000);
mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x80000020); // ERASE_SECTOR cmd
```

### Architecture Highlights

**Register-Based Interface**:
```
Firmware → Write SPI1_BASE+CMD_REG → Memory MMIO dispatch →
PeripheralBus → SpiFlash handler → Flash operation → Interrupt
```

**Flash Storage**:
- Internal `Arc<Mutex<Vec<u8>>>` backing store
- File I/O support: `load_from_file()` and `save_to_file()`
- Ready for memory-mapped integration (Phase 5)

### Integration Points

The SPI flash controller integrates seamlessly with existing infrastructure:

1. **Peripheral Bus**: Registered at 0x3FF43000 (SPI1_BASE)
2. **Interrupt Controller**: Can raise SPI1 interrupts
3. **ROM Stubs**: Updated `Cache_Read_Enable/Disable` with debug logging

## Testing

Phase 4 includes comprehensive test coverage:

### Unit Tests (7 tests)
- ✅ Flash read operations
- ✅ Flash write with protection
- ✅ Sector erase (4KB)
- ✅ Write protection enforcement
- ✅ Register access
- ✅ Status command
- ✅ Large buffer transfers (64 bytes)

### Integration Tests (6 tests)
- ✅ Multi-step command sequences (WRITE_ENABLE → WRITE → READ)
- ✅ Erase-write-read cycles
- ✅ Multiple location writes
- ✅ 64-byte transfer verification
- ✅ Write protection validation
- ✅ Register persistence

**All 83 tests passing** (70 existing + 13 new)

## Performance

The implementation prioritizes correctness and simplicity:

- **Instant operations**: No cycle-accurate timing (can be added later)
- **O(1) register access**: Direct match statement dispatch
- **O(n) flash operations**: Linear in bytes transferred (up to 64)
- **Minimal overhead**: Pure Rust, no FFI, no unnecessary allocations

## What's Next: Phase 5

With Phase 4 complete, we're ready to move into **Phase 5: Advanced Features**:

1. **Memory-Mapped Flash**: Integrate flash_store with Memory page table for direct reads from 0x3F400000
2. **Real Firmware Testing**: Load and execute actual ESP-IDF binaries
3. **Additional Peripherals**: General-purpose SPI, I2C, RMT, LEDC
4. **WiFi/Bluetooth Stubs**: Basic initialization and networking stubs
5. **LVGL Demo**: Full display integration and touch input

## Stats

**Phase 4 by the numbers:**
- **2 new files**: spi_flash.rs (430 lines), flash_integration_test.rs (260 lines)
- **4 files modified**: interrupt.rs, lib.rs, boot.rs
- **13 new tests**: 7 unit + 6 integration
- **100% pass rate**: All 83 tests passing
- **~690 lines added**: High-quality, well-tested code

## Try It Out

The SPI flash controller is available now in the `main` branch:

```bash
# Clone the repo
git clone https://github.com/levkropp/flexers.git
cd flexers

# Run SPI flash tests
cargo test --package flexers-periph --lib spi_flash
cargo test --package flexers-core --test flash_integration_test

# Run all tests
cargo test --all
```

## Documentation

For detailed implementation information, see:
- [PHASE4_COMPLETE.md](https://github.com/levkropp/flexers/blob/main/PHASE4_COMPLETE.md) - Full implementation summary
- [STATUS.md](https://github.com/levkropp/flexers/blob/main/STATUS.md) - Project status and roadmap
- [README.md](https://github.com/levkropp/flexers/blob/main/README.md) - Quick start guide

## Acknowledgments

Thanks to everyone following along with the Flexers development! Phase 4 brings us one step closer to full ESP32 firmware emulation.

Stay tuned for Phase 5 updates!

---

*Questions or feedback? Open an issue on [GitHub](https://github.com/levkropp/flexers/issues) or start a [discussion](https://github.com/levkropp/flexers/discussions).*
