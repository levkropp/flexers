/// Example: Using ROM Stubs in the Flexers ESP32 Emulator
///
/// This example demonstrates how to:
/// 1. Create a CPU and memory subsystem
/// 2. Attach the ROM stub dispatcher
/// 3. Write a simple firmware that calls ROM functions
/// 4. Execute the firmware and see ROM function calls in action

use flexers_core::{cpu::XtensaCpu, memory::Memory, run_batch};
use flexers_stubs::create_esp32_dispatcher;
use std::sync::{Arc, Mutex};

fn main() {
    println!("=== Flexers ROM Stubs Example ===\n");

    // Step 1: Create CPU and memory
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    // Step 2: Attach ROM stub dispatcher (enables ROM function calls)
    let dispatcher = create_esp32_dispatcher();
    cpu.set_rom_stub_dispatcher(Arc::new(Mutex::new(dispatcher)));

    println!("✓ CPU initialized with ROM stub support\n");

    // Step 3: Write a simple "firmware" that calls ROM functions
    // This simulates what real ESP-IDF firmware would do

    // Example 1: Call ets_get_cpu_frequency()
    // Address: 0x40008550
    println!("Example 1: Calling ets_get_cpu_frequency()");

    // Set up the call
    cpu.set_register(0, 0x4000_1000); // a0 = return address
    cpu.set_pc(0x40008550); // Jump to ets_get_cpu_frequency

    // Execute the ROM stub
    run_batch(&mut cpu, 1).expect("Failed to execute ROM stub");

    // Get the return value from a2
    let freq = cpu.get_register(2);
    println!("  CPU frequency: {} MHz", freq);
    println!("  Return address: 0x{:08X}\n", cpu.pc());

    // Example 2: Call ets_delay_us(1000) - 1ms delay
    println!("Example 2: Calling ets_delay_us(1000)");

    let cycles_before = cpu.cycle_count();

    cpu.set_register(2, 1000); // a2 = microseconds
    cpu.set_register(0, 0x4000_1000); // a0 = return address
    cpu.set_pc(0x40008534); // Jump to ets_delay_us

    run_batch(&mut cpu, 1).expect("Failed to execute ROM stub");

    let cycles_after = cpu.cycle_count();
    let cycles_elapsed = cycles_after - cycles_before;

    println!("  Delay: 1000 us");
    println!("  Cycles elapsed: {} (expected ~160,001)", cycles_elapsed);
    println!("  Return address: 0x{:08X}\n", cpu.pc());

    // Example 3: Call memset() to zero out memory
    println!("Example 3: Calling memset(0x3FFA_0000, 0xAA, 16)");

    cpu.set_register(2, 0x3FFA_0000); // a2 = dest
    cpu.set_register(3, 0xAA); // a3 = value
    cpu.set_register(4, 16); // a4 = count
    cpu.set_register(0, 0x4000_1000); // a0 = return address
    cpu.set_pc(0x4000C2E0); // Jump to memset

    run_batch(&mut cpu, 1).expect("Failed to execute ROM stub");

    // Verify memory was set
    print!("  Memory at 0x3FFA_0000: ");
    for i in 0..16 {
        print!("{:02X} ", cpu.memory().read_u8(0x3FFA_0000 + i));
    }
    println!("\n");

    // Example 4: Call esp_rom_printf()
    println!("Example 4: Calling esp_rom_printf()");

    // Write format string to memory
    let fmt_str = b"Hello from ROM! Value = %d\n\0";
    for (i, &byte) in fmt_str.iter().enumerate() {
        cpu.memory().write_u8(0x3FFA_1000 + i as u32, byte);
    }

    cpu.set_register(2, 0x3FFA_1000); // a2 = format string
    cpu.set_register(3, 42); // a3 = value to print
    cpu.set_register(0, 0x4000_1000); // a0 = return address
    cpu.set_pc(0x40007ABC); // Jump to esp_rom_printf

    print!("  Output: ");
    run_batch(&mut cpu, 1).expect("Failed to execute ROM stub");
    println!();

    // Summary
    println!("=== Summary ===");
    println!("✓ All ROM function calls executed successfully");
    println!("✓ Total cycles: {}", cpu.cycle_count());
    println!("\nThe emulator can now run real ESP-IDF firmware that depends on ROM functions!");
}
