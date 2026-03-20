use flexers_core::{cpu::XtensaCpu, memory::Memory};
use flexers_periph::{
    spi_flash::SpiFlash,
    bus::PeripheralBus,
    bus::AddrRange,
    interrupt::{InterruptController, InterruptSource},
    SPI1_BASE,
};
use std::sync::{Arc, Mutex};

/// SPI Flash register offsets
const SPI_CMD_REG: u32 = 0x00;
const SPI_ADDR_REG: u32 = 0x04;
const SPI_CTRL_REG: u32 = 0x08;
const SPI_W0_REG: u32 = 0x80;

/// Flash commands
const CMD_READ: u8 = 0x03;
const CMD_WRITE: u8 = 0x02;
const CMD_ERASE_SECTOR: u8 = 0x20;
const CMD_WRITE_ENABLE: u8 = 0x06;

#[test]
fn test_flash_read_write_sequence() {
    // Setup
    let mem = Arc::new(Memory::new());
    let _cpu = XtensaCpu::new(mem.clone());

    // Create interrupt controller
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));

    // Create SPI flash controller
    let mut spi_flash = SpiFlash::new(1024 * 1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    // Register in peripheral bus
    let mut bus = PeripheralBus::new();
    let spi_range = AddrRange::new(SPI1_BASE, SPI1_BASE + 0x200);
    bus.register(spi_range, Box::new(spi_flash));

    // Set peripheral bus in memory
    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // Execute firmware sequence:
    // 1. Enable writes
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));

    // 2. Write data
    mem.write_u32(SPI1_BASE + SPI_W0_REG, 0xDEADBEEF);
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x1000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE as u32));

    // 3. Read back
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x1000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_READ as u32));

    // 4. Verify data
    let read_data = mem.read_u32(SPI1_BASE + SPI_W0_REG);
    assert_eq!(read_data, 0xDEADBEEF);
}

#[test]
fn test_flash_erase_write_read() {
    // Setup
    let mem = Arc::new(Memory::new());
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));

    let mut spi_flash = SpiFlash::new(16 * 1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    let mut bus = PeripheralBus::new();
    let spi_range = AddrRange::new(SPI1_BASE, SPI1_BASE + 0x200);
    bus.register(spi_range, Box::new(spi_flash));

    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // 1. Erase sector 0
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x0000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_ERASE_SECTOR as u32));

    // 2. Write pattern to start of sector
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));
    mem.write_u32(SPI1_BASE + SPI_W0_REG, 0x12345678);
    mem.write_u32(SPI1_BASE + SPI_W0_REG + 4, 0xABCDEF00);
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x0000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE as u32));

    // 3. Read back and verify
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x0000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_READ as u32));

    let data0 = mem.read_u32(SPI1_BASE + SPI_W0_REG);
    let data1 = mem.read_u32(SPI1_BASE + SPI_W0_REG + 4);

    assert_eq!(data0, 0x12345678);
    assert_eq!(data1, 0xABCDEF00);
}

#[test]
fn test_flash_multiple_writes() {
    // Setup
    let mem = Arc::new(Memory::new());
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));

    let mut spi_flash = SpiFlash::new(64 * 1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    let mut bus = PeripheralBus::new();
    let spi_range = AddrRange::new(SPI1_BASE, SPI1_BASE + 0x200);
    bus.register(spi_range, Box::new(spi_flash));

    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // Write to multiple locations
    let test_data = [
        (0x0000, 0x11111111),
        (0x1000, 0x22222222),
        (0x2000, 0x33333333),
        (0x3000, 0x44444444),
    ];

    for (addr, data) in &test_data {
        mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));
        mem.write_u32(SPI1_BASE + SPI_W0_REG, *data);
        mem.write_u32(SPI1_BASE + SPI_ADDR_REG, *addr);
        mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE as u32));
    }

    // Read back and verify all
    for (addr, expected_data) in &test_data {
        mem.write_u32(SPI1_BASE + SPI_ADDR_REG, *addr);
        mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_READ as u32));

        let read_data = mem.read_u32(SPI1_BASE + SPI_W0_REG);
        assert_eq!(read_data, *expected_data, "Mismatch at address 0x{:04X}", addr);
    }
}

#[test]
fn test_flash_64_byte_transfer() {
    // Setup
    let mem = Arc::new(Memory::new());
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));

    let mut spi_flash = SpiFlash::new(1024 * 1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    let mut bus = PeripheralBus::new();
    let spi_range = AddrRange::new(SPI1_BASE, SPI1_BASE + 0x200);
    bus.register(spi_range, Box::new(spi_flash));

    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // Write 64 bytes (16 words)
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE_ENABLE as u32));

    for i in 0..16u32 {
        let data = 0x10000000 + i;
        mem.write_u32(SPI1_BASE + SPI_W0_REG + (i * 4), data);
    }

    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x5000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE as u32));

    // Read back 64 bytes
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x5000);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_READ as u32));

    // Verify all 16 words
    for i in 0..16u32 {
        let expected = 0x10000000 + i;
        let actual = mem.read_u32(SPI1_BASE + SPI_W0_REG + (i * 4));
        assert_eq!(actual, expected, "Mismatch at word {}", i);
    }
}

#[test]
fn test_flash_write_without_enable() {
    // Setup
    let mem = Arc::new(Memory::new());
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));

    let mut spi_flash = SpiFlash::new(1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    let mut bus = PeripheralBus::new();
    let spi_range = AddrRange::new(SPI1_BASE, SPI1_BASE + 0x200);
    bus.register(spi_range, Box::new(spi_flash));

    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // Try to write WITHOUT enabling first
    mem.write_u32(SPI1_BASE + SPI_W0_REG, 0xBADBAD00);
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x0100);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_WRITE as u32));

    // Read back - should still be erased (0xFF)
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0x0100);
    mem.write_u32(SPI1_BASE + SPI_CMD_REG, 0x8000_0000 | (CMD_READ as u32));

    let read_data = mem.read_u32(SPI1_BASE + SPI_W0_REG);
    assert_eq!(read_data, 0xFFFFFFFF, "Write protection failed - data was written!");
}

#[test]
fn test_flash_register_readback() {
    // Setup
    let mem = Arc::new(Memory::new());
    let int_controller = Arc::new(Mutex::new(InterruptController::new()));

    let mut spi_flash = SpiFlash::new(1024, InterruptSource::Spi1);
    spi_flash.set_interrupt_raiser(int_controller.clone());

    let mut bus = PeripheralBus::new();
    let spi_range = AddrRange::new(SPI1_BASE, SPI1_BASE + 0x200);
    bus.register(spi_range, Box::new(spi_flash));

    unsafe {
        let mem_mut = &mut *(Arc::as_ptr(&mem) as *mut Memory);
        mem_mut.set_peripheral_bus(Arc::new(Mutex::new(bus)));
    }

    // Test address register
    mem.write_u32(SPI1_BASE + SPI_ADDR_REG, 0xABCDEF);
    let addr_read = mem.read_u32(SPI1_BASE + SPI_ADDR_REG);
    assert_eq!(addr_read, 0xABCDEF);

    // Test control register
    mem.write_u32(SPI1_BASE + SPI_CTRL_REG, 0x12345678);
    let ctrl_read = mem.read_u32(SPI1_BASE + SPI_CTRL_REG);
    assert_eq!(ctrl_read, 0x12345678);

    // Test data buffer registers
    for i in 0..16 {
        let val = 0x10000000 + i;
        mem.write_u32(SPI1_BASE + SPI_W0_REG + (i * 4), val);
        let read_val = mem.read_u32(SPI1_BASE + SPI_W0_REG + (i * 4));
        assert_eq!(read_val, val, "W{} register mismatch", i);
    }
}
