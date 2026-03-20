# flexers-stubs

ROM function stubs for the Flexers ESP32 emulator.

## Overview

This crate provides ROM stub implementations for ESP32 ROM functions. Real ESP-IDF firmware depends heavily on ROM functions provided by Espressif's boot ROM. This crate emulates these functions to enable running real ESP32 binaries in the emulator.

## Features

- **Symbol Table**: Maps ROM addresses to function names
- **Stub Dispatcher**: Routes ROM function calls to Rust implementations
- **Core ROM Functions**:
  - I/O: `esp_rom_printf`, `ets_putc`, `uart_tx_one_char`, etc.
  - Memory: `memcpy`, `memset`, `memcmp`, `memmove`
  - Timing: `ets_delay_us`, `ets_get_cpu_frequency`
  - Boot: `Cache_Read_Enable`, `rtc_get_reset_reason`

## Usage

### Quick Start

```rust
use flexers_core::{cpu::XtensaCpu, memory::Memory};
use flexers_stubs::create_esp32_dispatcher;
use std::sync::{Arc, Mutex};

// Create CPU and memory
let mem = Arc::new(Memory::new());
let mut cpu = XtensaCpu::new(mem.clone());

// Create and attach ROM dispatcher
let dispatcher = create_esp32_dispatcher();
cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

// Now the CPU can execute ROM function calls!
```

### Custom Stub Handlers

You can implement your own ROM function stubs:

```rust
use flexers_core::cpu::XtensaCpu;
use flexers_stubs::handler::RomStubHandler;

struct MyCustomStub;

impl RomStubHandler for MyCustomStub {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        // Read arguments from registers a2-a7
        let arg1 = cpu.get_register(2);
        let arg2 = cpu.get_register(3);

        // Do something...

        // Return value (goes into a2)
        42
    }

    fn name(&self) -> &str {
        "my_custom_stub"
    }
}

// Register it
dispatcher.register(MyCustomStub);
```

## Architecture

### Call Flow

1. Firmware executes `CALL0 0x40007ABC` (ROM function address)
2. CPU execution loop detects PC in ROM range (0x4000_0000+)
3. ROM dispatcher looks up function name from symbol table
4. Dispatcher finds matching stub handler
5. Handler executes Rust implementation
6. Return value placed in a2, PC set to return address in a0

### Xtensa Calling Convention

ROM stubs follow the Xtensa Windowed ABI:

- **Arguments**: a2, a3, a4, a5, a6, a7 (up to 6 args)
- **Return value**: a2
- **Return address**: a0
- **Stack pointer**: a1

## Implemented ROM Functions

### I/O Functions

- `esp_rom_printf(const char* fmt, ...)` - Printf to console
- `ets_putc(char c)` - Output single character
- `uart_tx_one_char(uint8_t c)` - UART transmit

### Memory Functions

- `memcpy(void* dest, const void* src, size_t n)`
- `memset(void* dest, int val, size_t n)`
- `memcmp(const void* s1, const void* s2, size_t n)`
- `memmove(void* dest, const void* src, size_t n)`

### Timing Functions

- `ets_delay_us(uint32_t us)` - Microsecond delay
- `ets_get_cpu_frequency()` - Returns CPU frequency in MHz (160)

### Boot/System Functions

- `Cache_Read_Enable()` - Enable flash cache
- `rtc_get_reset_reason()` - Get reset reason (returns POWERON_RESET)

## Testing

Run tests with:

```bash
cargo test --package flexers-stubs
```

Integration tests:

```bash
cargo test --package flexers-core --test rom_stub_test
```

## Future Enhancements

- [ ] Full printf format string parsing (%d, %x, %s, %f, etc.)
- [ ] More ROM functions (100+ functions for full ESP-IDF support)
- [ ] Symbol versioning (different ESP32 ROM versions)
- [ ] Cryptographic functions (SHA, AES)
- [ ] Math library functions (div, mod, float ops)

## License

Same as parent project
