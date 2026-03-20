use flexers_core::cpu::XtensaCpu;
use flexers_core::memory::{Memory, MmioHandler};
use flexers_periph::*;
use flexers_periph::uart::InterruptRaiser;
use std::sync::{Arc, Mutex};

#[test]
fn test_peripheral_bus_integration() {
    // Create memory and CPU
    let mem = Arc::new(Memory::new());
    let _cpu = XtensaCpu::new(mem.clone());

    // Create peripheral bus
    let mut bus = PeripheralBus::new();

    // Create and register UART
    let uart = Uart::new(InterruptSource::Uart0);
    let uart_range = AddrRange::new(UART0_BASE, UART0_BASE + 0x100);
    bus.register(uart_range, Box::new(uart));

    // Create interrupt controller
    let ic = InterruptController::new();
    let ic_range = AddrRange::new(INTERRUPT_BASE, INTERRUPT_BASE + 0x100);
    bus.register(ic_range, Box::new(ic));

    // Wrap bus in Arc<Mutex<>>
    let bus = Arc::new(Mutex::new(bus));

    // Test UART read/write through bus
    {
        let mut bus_lock = bus.lock().unwrap();

        // Write to UART TX FIFO
        bus_lock.dispatch_write(UART0_BASE + 0x00, 4, b'H' as u32);

        // Read UART status
        let status = bus_lock.dispatch_read(UART0_BASE + 0x1C, 4);
        assert!(status.is_some());
    }
}

#[test]
fn test_interrupt_controller_with_uart() {

    // Create interrupt controller
    let ic = Arc::new(Mutex::new(InterruptController::new()));

    // Create UART and connect to interrupt controller
    let mut uart = Uart::new(InterruptSource::Uart0);
    uart.set_interrupt_raiser(ic.clone());

    // Enable UART RX interrupt in UART itself (bit 0 = RXFIFO_FULL_INT)
    uart.write(0x0C, 4, 0x01); // UART_INT_ENA_REG

    // Enable UART interrupt in interrupt controller
    {
        let mut ic_lock = ic.lock().unwrap();
        ic_lock.set_enabled(1 << (InterruptSource::Uart0 as u8));
    }

    // Inject enough bytes to trigger RX interrupt (threshold = 8)
    for _ in 0..8 {
        uart.inject_rx(b'X');
    }

    // Verify interrupt was raised
    let ic_lock = ic.lock().unwrap();
    let pending = ic_lock.get_pending();
    assert_eq!(pending & 0x1, 0x1); // UART0 is bit 0
}

#[test]
fn test_timer_with_interrupts() {

    // Create interrupt controller
    let ic = Arc::new(Mutex::new(InterruptController::new()));

    // Create timer and connect to interrupt controller
    let mut timer = Timer::new(InterruptSource::Timer0Group0);
    timer.set_interrupt_raiser(ic.clone());

    // Configure timer: alarm at 10, auto-reload enabled, interrupt enabled
    timer.write(0x08, 4, 10); // Alarm low
    timer.write(0x10, 4, 0x07); // Enable | Auto-reload | Int-enable

    // Enable timer interrupt in controller
    {
        let mut ic_lock = ic.lock().unwrap();
        ic_lock.set_enabled(1 << (InterruptSource::Timer0Group0 as u8));
    }

    // Tick timer 10 times
    for _ in 0..10 {
        timer.tick();
    }

    // Verify interrupt was raised
    let ic_lock = ic.lock().unwrap();
    let pending = ic_lock.get_pending();
    assert_eq!(pending & (1 << (InterruptSource::Timer0Group0 as u8)), 1 << (InterruptSource::Timer0Group0 as u8));
}

#[test]
fn test_gpio_with_interrupts() {

    // Create interrupt controller
    let ic = Arc::new(Mutex::new(InterruptController::new()));

    // Create GPIO and connect to interrupt controller
    let mut gpio = Gpio::new();
    gpio.set_interrupt_raiser(ic.clone());

    // Enable GPIO interrupt in controller
    {
        let mut ic_lock = ic.lock().unwrap();
        ic_lock.set_enabled(1 << (InterruptSource::Gpio as u8));
    }

    // Configure pin 5 for rising edge interrupt (would need public methods for this in real implementation)
    // For now, just test basic GPIO I/O

    // Set pin 5 as output
    gpio.write(0x10, 4, 1 << 5); // Enable output

    // Set pin 5 high
    gpio.write(0x08, 4, 1 << 5); // Set output

    // Read back
    let output = gpio.read(0x08, 4);
    assert_eq!(output & (1 << 5), 1 << 5);

    // Verify pin is high
    assert!(gpio.get_output(5));
}

#[test]
fn test_multiple_peripherals_on_bus() {
    // Create peripheral bus
    let mut bus = PeripheralBus::new();

    // Register multiple UARTs
    let uart0 = Uart::new(InterruptSource::Uart0);
    bus.register(AddrRange::new(UART0_BASE, UART0_BASE + 0x100), Box::new(uart0));

    let uart1 = Uart::new(InterruptSource::Uart1);
    bus.register(AddrRange::new(UART1_BASE, UART1_BASE + 0x100), Box::new(uart1));

    // Register GPIO
    let gpio = Gpio::new();
    bus.register(AddrRange::new(GPIO_BASE, GPIO_BASE + 0x100), Box::new(gpio));

    // Test access to different peripherals
    bus.dispatch_write(UART0_BASE + 0x00, 4, b'A' as u32);
    bus.dispatch_write(UART1_BASE + 0x00, 4, b'B' as u32);
    bus.dispatch_write(GPIO_BASE + 0x08, 4, 0xFF);

    // Read GPIO output
    let gpio_out = bus.dispatch_read(GPIO_BASE + 0x08, 4);
    assert_eq!(gpio_out, Some(0xFF));
}
