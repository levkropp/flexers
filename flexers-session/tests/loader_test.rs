use flexers_session::load_firmware;
use flexers_core::{cpu::XtensaCpu, memory::Memory, run_batch};
use std::sync::Arc;
use std::path::Path;

#[test]
#[ignore] // Ignore by default (requires firmware file)
fn test_load_real_firmware() {
    let mem = Arc::new(Memory::new());

    // Load firmware (user must provide test firmware)
    let firmware_path = Path::new("tests/fixtures/test_firmware.bin");
    if !firmware_path.exists() {
        println!("Skipping test - no firmware file at {:?}", firmware_path);
        return;
    }

    let info = load_firmware(firmware_path, &mem).unwrap();

    // Verify entry point is reasonable (ESP32 addresses start at 0x40000000 or 0x3FF00000)
    assert!(info.entry_point >= 0x3FF00000);
    assert!(info.segment_count > 0);

    // Create CPU and set entry point
    let mut cpu = XtensaCpu::new(mem.clone());
    cpu.set_pc(info.entry_point);

    // Run a few cycles
    let result = run_batch(&mut cpu, 100);
    assert!(result.is_ok());
}

#[test]
fn test_loader_error_handling() {
    use flexers_session::LoadError;

    let mem = Arc::new(Memory::new());

    // Test with non-existent file
    let result = load_firmware(Path::new("nonexistent.bin"), &mem);
    assert!(matches!(result, Err(LoadError::IoError(_))));

    // Test with invalid magic byte (would need to create a temp file)
    // For now, just verify the error types exist
    let _magic_err = LoadError::InvalidMagic(0xFF);
    let _format_err = LoadError::InvalidFormat("test".to_string());
}
