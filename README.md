# Flexers: Rust ESP32 Emulator

A high-performance ESP32 emulator written in Rust, designed as a native replacement for the C-based Flexe emulator.

## Overview

Flexers provides:
- **Native Rust integration** - No FFI overhead, seamless integration with Rust applications
- **Memory safety** - Eliminates entire classes of bugs (null pointers, use-after-free, data races)
- **Better debugging** - Clear error types, full backtraces, thread-safe by design
- **Easier maintenance** - Unified toolchain, no manual bindings generation

## Architecture

- **flexers-core**: CPU core, memory subsystem, instruction execution
- **flexers-periph**: Peripheral emulation (UART, timers, GPIO, SPI, etc.)
- **flexers-stubs**: ROM function stubs and display integration
- **flexers-session**: High-level API for managing emulation sessions

## Roadmap

- **Phase 1** (Weeks 1-3): Core CPU & Memory Foundation ✅ (In Progress)
- **Phase 2** (Weeks 4-5): Peripherals & I/O
- **Phase 3** (Weeks 6-7): ROM Stubs & Symbols
- **Phase 4** (Week 8): Display Integration
- **Phase 5** (Week 9): Cyders Integration
- **Phase 6** (Future): JIT Compilation (if needed)

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
