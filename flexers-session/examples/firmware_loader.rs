/*
 * Firmware Loader Example
 *
 * Demonstrates how to load and run ESP32 firmware binaries with the Flexers emulator.
 *
 * Usage:
 *   cargo run --example firmware_loader [firmware.bin]
 *
 * If no firmware path is provided, uses the test firmware.
 */

use flexers_core::{cpu::XtensaCpu, memory::Memory, run_batch};
use flexers_session::loader::load_firmware;
use flexers_stubs::registry::create_esp32_dispatcher;
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn main() {
    println!("Flexers Firmware Loader Example\n");
    println!("=================================\n");

    // Get firmware path from command line or use default
    let firmware_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "test-firmware/minimal_test.bin".to_string());

    let firmware_path = Path::new(&firmware_path);

    // Check if firmware exists
    if !firmware_path.exists() {
        eprintln!("Error: Firmware file not found: {:?}", firmware_path);
        eprintln!("\nTo create test firmware:");
        eprintln!("  cd test-firmware");
        eprintln!("  python generate_test_binary.py");
        std::process::exit(1);
    }

    println!("Loading firmware: {}", firmware_path.display());

    // Step 1: Create memory subsystem
    let mem = Arc::new(Memory::new());
    println!("✓ Memory subsystem initialized");

    // Step 2: Create CPU
    let mut cpu = XtensaCpu::new(mem.clone());
    println!("✓ CPU initialized (Xtensa LX6)");

    // Step 3: Set up ROM stub dispatcher
    let dispatcher = Arc::new(Mutex::new(create_esp32_dispatcher()));
    cpu.set_rom_stub_dispatcher(dispatcher);
    println!("✓ ROM stub dispatcher attached");

    // Step 4: Load firmware binary
    match load_firmware(firmware_path, &mem) {
        Ok(firmware_info) => {
            println!("\n✓ Firmware loaded successfully!");
            println!("  Entry point: 0x{:08X}", firmware_info.entry_point);
            println!("  Segments: {}", firmware_info.segment_count);

            for (i, seg) in firmware_info.segments.iter().enumerate() {
                println!("    Segment {}: addr=0x{:08X}, size={} bytes",
                         i, seg.address, seg.size);

                // Show memory region
                let region_name = match seg.address {
                    0x3FFA0000..=0x3FFFFFFF => "SRAM",
                    0x3F400000..=0x3F7FFFFF => "Flash Data",
                    0x40080000..=0x400BFFFF => "Flash Instruction",
                    0x3FF80000..=0x3FF81FFF => "RTC DRAM",
                    _ => "Unknown",
                };
                println!("               Region: {}", region_name);
            }

            // Step 5: Set PC to entry point
            cpu.set_pc(firmware_info.entry_point);
            println!("\n✓ PC set to entry point");

            // Step 6: Run firmware
            println!("\n--- Running Firmware ---\n");

            let max_cycles = 1000;
            let mut total_cycles = 0;
            let batch_size = 100;

            loop {
                if total_cycles >= max_cycles {
                    println!("\nReached maximum cycle limit ({} cycles)", max_cycles);
                    break;
                }

                let pc = cpu.pc();

                // Execute a batch of instructions
                match run_batch(&mut cpu, batch_size) {
                    Ok(executed) => {
                        total_cycles += executed;

                        if executed > 0 {
                            println!("Executed {} cycles (total: {}, PC: 0x{:08X})",
                                     executed, total_cycles, pc);
                        }

                        if executed == 0 {
                            println!("\nCPU halted at PC: 0x{:08X}", cpu.pc());
                            println!("Total cycles: {}", total_cycles);
                            break;
                        }
                    }
                    Err(e) => {
                        println!("\nExecution error: {:?}", e);
                        println!("PC: 0x{:08X}", cpu.pc());
                        println!("Total cycles: {}", total_cycles);

                        // Show some context
                        println!("\nCurrent register state:");
                        for i in 0..16 {
                            println!("  a{:2}: 0x{:08X}", i, cpu.get_register(i));
                        }

                        break;
                    }
                }
            }

            println!("\n--- Execution Complete ---");
            println!("Final cycle count: {}", cpu.cycle_count());
        }
        Err(e) => {
            eprintln!("\n✗ Failed to load firmware: {:?}", e);
            std::process::exit(1);
        }
    }
}
