# Flexers: Rust ESP32 Emulator

A high-performance ESP32 emulator written in Rust, designed as a native replacement for the C-based Flexe emulator.

## Overview

Flexers provides:
- **Native Rust integration** - No FFI overhead, seamless integration with Rust applications
- **Memory safety** - Eliminates entire classes of bugs (null pointers, use-after-free, data races)
- **Better debugging** - Clear error types, full backtraces, thread-safe by design
- **Easier maintenance** - Unified toolchain, no manual bindings generation

## Architecture

- **flexers-core**: CPU core (Xtensa LX6), memory subsystem, instruction execution (30+ instructions)
- **flexers-periph**: Peripheral emulation (UART, timers, GPIO, SPI flash, interrupt controller)
- **flexers-stubs**: ROM function stubs (16 implemented: printf, memcpy, timing, boot)
- **flexers-session**: High-level API for managing emulation sessions

## Current Status

- **Phase 1** ✅ COMPLETE: Core CPU & Memory Foundation (28 tests)
- **Phase 2** ✅ COMPLETE: Peripherals & I/O (25 tests)
- **Phase 3** ✅ COMPLETE: ROM Stubs & Symbols (12 tests)
- **Phase 4** ✅ COMPLETE: Flash SPI Emulation (13 tests)
- **Phase 5** (Next): Memory-mapped flash, real firmware testing, advanced peripherals

**Total**: 83 tests passing, ~7,140 lines of Rust code

## Target

Full LVGL demo capability with display rendering and touch input support.

## Building

```bash
cargo build --release
```

## Testing

```bash
# Unit tests
cargo test --all

# Integration tests
cargo test --test phase1_boot

# Benchmarks
cargo bench
```

## License

MIT OR Apache-2.0
