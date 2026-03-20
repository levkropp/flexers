use flexers_core::{cpu::XtensaCpu, memory::Memory, run_batch};
use flexers_stubs::{
    dispatcher::RomStubDispatcher,
    symbol_table::SymbolTable,
    functions::{io::*, timing::*, boot::*},
};
use std::sync::{Arc, Mutex};

#[test]
fn test_rom_printf_call() {
    // Setup
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    // Load symbol table
    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());

    // Create ROM dispatcher
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(EspRomPrintf);
    let dispatcher = Arc::new(Mutex::new(dispatcher));

    // Attach to CPU
    cpu.set_rom_stub_dispatcher(dispatcher);

    // Write format string to memory
    let fmt_str = b"Hello %d\n\0";
    for (i, &byte) in fmt_str.iter().enumerate() {
        cpu.memory().write_u8(0x3FFA_0000 + i as u32, byte);
    }

    // Set up registers for call
    cpu.set_register(2, 0x3FFA_0000);  // a2 = format string pointer
    cpu.set_register(3, 42);           // a3 = first argument
    cpu.set_register(0, 0x4000_1000);  // a0 = return address

    // Set PC to esp_rom_printf
    cpu.set_pc(0x40007ABC);

    // Execute one instruction (ROM stub)
    let result = run_batch(&mut cpu, 1);
    assert!(result.is_ok());

    // Verify returned to caller
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_delay_us_timing() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(EtsDelayUs);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // Call ets_delay_us(1000) - 1ms delay
    cpu.set_register(2, 1000);  // a2 = microseconds
    cpu.set_register(0, 0x4000_1000);  // a0 = return address
    cpu.set_pc(0x40008534);  // ets_delay_us address

    let cycles_before = cpu.cycle_count();
    run_batch(&mut cpu, 1).unwrap();
    let cycles_after = cpu.cycle_count();

    // Should have advanced by ~160,000 cycles (1000 us * 160 MHz) + 1 for the ROM call itself
    assert_eq!(cycles_after - cycles_before, 160_001);

    // Verify returned to caller
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_memcpy_stub() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(Memcpy);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // Write source data
    let src_data = b"Hello, World!";
    for (i, &byte) in src_data.iter().enumerate() {
        cpu.memory().write_u8(0x3FFA_0000 + i as u32, byte);
    }

    // Set up memcpy call: memcpy(dest, src, n)
    cpu.set_register(2, 0x3FFA_1000);  // a2 = dest
    cpu.set_register(3, 0x3FFA_0000);  // a3 = src
    cpu.set_register(4, 13);           // a4 = n (length)
    cpu.set_register(0, 0x4000_1000);  // a0 = return address
    cpu.set_pc(0x4000C2C4);  // memcpy address

    run_batch(&mut cpu, 1).unwrap();

    // Verify data was copied
    for (i, &expected) in src_data.iter().enumerate() {
        let actual = cpu.memory().read_u8(0x3FFA_1000 + i as u32);
        assert_eq!(actual, expected, "Mismatch at byte {}", i);
    }

    // Verify return value (dest pointer in a2)
    assert_eq!(cpu.get_register(2), 0x3FFA_1000);

    // Verify returned to caller
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_memset_stub() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(Memset);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // Set up memset call: memset(dest, val, n)
    cpu.set_register(2, 0x3FFA_0000);  // a2 = dest
    cpu.set_register(3, 0xAA);         // a3 = value
    cpu.set_register(4, 100);          // a4 = n (length)
    cpu.set_register(0, 0x4000_1000);  // a0 = return address
    cpu.set_pc(0x4000C2E0);  // memset address

    run_batch(&mut cpu, 1).unwrap();

    // Verify memory was set
    for i in 0..100 {
        let actual = cpu.memory().read_u8(0x3FFA_0000 + i);
        assert_eq!(actual, 0xAA, "Mismatch at byte {}", i);
    }

    // Verify return value (dest pointer in a2)
    assert_eq!(cpu.get_register(2), 0x3FFA_0000);

    // Verify returned to caller
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_get_cpu_frequency() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(EtsGetCpuFrequency);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // Call ets_get_cpu_frequency()
    cpu.set_register(0, 0x4000_1000);  // a0 = return address
    cpu.set_pc(0x40008550);  // ets_get_cpu_frequency address

    run_batch(&mut cpu, 1).unwrap();

    // Verify return value (160 MHz in a2)
    assert_eq!(cpu.get_register(2), 160);

    // Verify returned to caller
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_cache_enable_stub() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(CacheReadEnable);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // Call Cache_Read_Enable()
    cpu.set_register(0, 0x4000_1000);  // a0 = return address
    cpu.set_pc(0x40009A44);  // Cache_Read_Enable address

    run_batch(&mut cpu, 1).unwrap();

    // Verify returned successfully
    assert_eq!(cpu.get_register(2), 0);  // Return value should be 0 (success)
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_rtc_get_reset_reason() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(RtcGetResetReason);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // Call rtc_get_reset_reason()
    cpu.set_register(0, 0x4000_1000);  // a0 = return address
    cpu.set_pc(0x40008B94);  // rtc_get_reset_reason address

    run_batch(&mut cpu, 1).unwrap();

    // Verify return value (1 = POWERON_RESET)
    assert_eq!(cpu.get_register(2), 1);
    assert_eq!(cpu.pc(), 0x4000_1000);
}

#[test]
fn test_multiple_rom_calls() {
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);
    dispatcher.register(EtsDelayUs);
    dispatcher.register(EtsGetCpuFrequency);
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    // First call: ets_get_cpu_frequency()
    cpu.set_register(0, 0x4000_1000);
    cpu.set_pc(0x40008550);
    run_batch(&mut cpu, 1).unwrap();
    assert_eq!(cpu.get_register(2), 160);

    // Second call: ets_delay_us(100)
    cpu.set_register(2, 100);
    cpu.set_register(0, 0x4000_1000);
    cpu.set_pc(0x40008534);
    let cycles_before = cpu.cycle_count();
    run_batch(&mut cpu, 1).unwrap();
    let cycles_after = cpu.cycle_count();
    assert_eq!(cycles_after - cycles_before, 16_001);  // 100 us * 160 MHz + 1 for ROM call
}
