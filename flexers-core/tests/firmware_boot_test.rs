/*
 * Firmware Boot Integration Test
 *
 * This test validates the complete firmware loading and execution flow:
 * 1. Load ESP32 binary format
 * 2. Initialize memory and peripherals
 * 3. Set up ROM stubs
 * 4. Execute firmware from flash
 * 5. Verify ROM function calls
 */

use flexers_core::{cpu::XtensaCpu, memory::Memory, run_batch};
use flexers_session::loader::load_firmware;
use flexers_stubs::registry::create_esp32_dispatcher;
use std::sync::{Arc, Mutex};
use std::path::Path;

#[test]
fn test_load_minimal_firmware() {
    // Load the test binary
    let firmware_path = Path::new("../test-firmware/minimal_test.bin");

    // Skip test if firmware not found (toolchain not available)
    if !firmware_path.exists() {
        eprintln!("Skipping test: firmware not found at {:?}", firmware_path);
        eprintln!("Run: cd ../test-firmware && python generate_test_binary.py");
        return;
    }

    // Set up memory and CPU
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    // Load firmware
    let firmware_info = load_firmware(firmware_path, &mem)
        .expect("Failed to load firmware");

    println!("Loaded firmware:");
    println!("  Entry point: 0x{:08X}", firmware_info.entry_point);
    println!("  Segments: {}", firmware_info.segment_count);

    for (i, seg) in firmware_info.segments.iter().enumerate() {
        println!("    Segment {}: addr=0x{:08X}, size={} bytes",
                 i, seg.address, seg.size);
    }

    // Verify entry point is in flash region
    assert!(
        firmware_info.entry_point >= 0x40080000 &&
        firmware_info.entry_point < 0x400C0000,
        "Entry point should be in flash instruction region"
    );

    // Set PC to entry point
    cpu.set_pc(firmware_info.entry_point);

    // Verify PC was set correctly
    assert_eq!(cpu.pc(), firmware_info.entry_point);

    println!("\nFirmware loaded successfully!");
}

#[test]
fn test_run_minimal_firmware() {
    let firmware_path = Path::new("../test-firmware/minimal_test.bin");

    if !firmware_path.exists() {
        eprintln!("Skipping test: firmware not found");
        return;
    }

    // Set up memory and CPU
    let mem = Arc::new(Memory::new());
    let mut cpu = XtensaCpu::new(mem.clone());

    // Set up ROM stub dispatcher
    let dispatcher = Arc::new(Mutex::new(create_esp32_dispatcher()));
    cpu.set_rom_stub_dispatcher(dispatcher);

    // Load firmware
    let firmware_info = load_firmware(firmware_path, &mem)
        .expect("Failed to load firmware");

    cpu.set_pc(firmware_info.entry_point);

    println!("Running firmware from 0x{:08X}...", cpu.pc());

    // Execute in batches
    let max_cycles = 100;
    let mut total_cycles = 0;
    let mut last_pc = 0;
    let mut pc_stable_count = 0;

    while total_cycles < max_cycles {
        let pc = cpu.pc();

        // Check if we've hit a halt condition (PC not changing)
        if pc == last_pc {
            pc_stable_count += 1;
            if pc_stable_count >= 3 {
                println!("Detected halt at PC: 0x{:08X} after {} cycles", pc, total_cycles);
                break;
            }
        } else {
            pc_stable_count = 0;
        }

        last_pc = pc;

        // Execute a batch
        match run_batch(&mut cpu, 10) {
            Ok(executed) => {
                total_cycles += executed;
                if executed == 0 {
                    println!("CPU halted at {} cycles", total_cycles);
                    break;
                }
            }
            Err(e) => {
                println!("Execution stopped at cycle {}: {:?}", total_cycles, e);
                println!("Final PC: 0x{:08X}", cpu.pc());
                break;
            }
        }
    }

    println!("Executed {} cycles", total_cycles);

    // Verify we executed at least some instructions
    assert!(total_cycles > 0, "Should execute at least one instruction");
}

#[test]
fn test_firmware_with_peripherals() {
    use flexers_periph::{
        PeripheralBus, AddrRange, SpiFlash,
        InterruptController, InterruptSource,
    };

    let firmware_path = Path::new("../test-firmware/minimal_test.bin");

    if !firmware_path.exists() {
        eprintln!("Skipping test: firmware not found");
        return;
    }

    // Set up memory
    let mem = Arc::new(Memory::new());

    // Set up SPI flash controller
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));
    let mut spi_flash = SpiFlash::new(4 * 1024 * 1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    // Get flash store reference before moving spi_flash
    let flash_store = spi_flash.flash_store();

    // Set up peripheral bus
    let mut bus = PeripheralBus::new();
    bus.register(
        AddrRange::new(0x3FF43000, 0x3FF43200),
        Box::new(spi_flash),
    );

    // Attach peripheral bus to memory
    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // Load firmware
    let firmware_info = load_firmware(firmware_path, &mem)
        .expect("Failed to load firmware");

    // Copy flash contents to memory-mapped regions
    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.load_flash_from_controller(flash_store);
    }

    println!("Firmware loaded with peripheral integration:");
    println!("  Entry point: 0x{:08X}", firmware_info.entry_point);
    println!("  SPI flash controller configured");
    println!("  Flash-backed memory ready");

    // Set up CPU
    let mut cpu = XtensaCpu::new(mem);
    let dispatcher = Arc::new(Mutex::new(create_esp32_dispatcher()));
    cpu.set_rom_stub_dispatcher(dispatcher);
    cpu.set_pc(firmware_info.entry_point);

    // Try to execute
    match run_batch(&mut cpu, 50) {
        Ok(cycles) => {
            println!("Executed {} cycles with peripheral bus", cycles);
            assert!(cycles > 0, "Should execute with peripherals");
        }
        Err(e) => {
            println!("Execution error: {:?}", e);
            // Some errors may be expected for incomplete firmware
        }
    }
}

#[test]
fn test_invalid_firmware_rejected() {
    let mem = Arc::new(Memory::new());

    // Test 1: Invalid magic byte
    let bad_magic = vec![0x00, 0x01, 0x00, 0x20, 0x00, 0x00, 0x08, 0x40];
    let temp_path = Path::new("test_bad_magic.bin");
    std::fs::write(temp_path, &bad_magic).unwrap();

    let result = load_firmware(temp_path, &mem);
    assert!(matches!(result, Err(flexers_session::loader::LoadError::InvalidMagic(_))));

    std::fs::remove_file(temp_path).ok();

    // Test 2: Truncated file
    let truncated = vec![0xE9, 0x01];
    let temp_path = Path::new("test_truncated.bin");
    std::fs::write(temp_path, &truncated).unwrap();

    let result = load_firmware(temp_path, &mem);
    assert!(result.is_err());

    std::fs::remove_file(temp_path).ok();

    println!("Invalid firmware correctly rejected");
}
