pub mod bus;
pub mod uart;
pub mod timer;
pub mod gpio;
pub mod interrupt;

// ESP32 peripheral base addresses
pub const UART0_BASE: u32 = 0x3FF4_0000;
pub const UART1_BASE: u32 = 0x3FF5_0000;
pub const UART2_BASE: u32 = 0x3FF6_E000;
pub const GPIO_BASE: u32 = 0x3FF4_4000;
pub const TIMER_GROUP0_BASE: u32 = 0x3FF5_F000;
pub const TIMER_GROUP1_BASE: u32 = 0x3FF6_0000;
pub const RTC_BASE: u32 = 0x3FF4_8000;
pub const INTERRUPT_BASE: u32 = 0x3FF0_0000; // PRO CPU interrupt matrix

// Re-export commonly used types
pub use bus::{PeripheralBus, AddrRange};
pub use interrupt::{InterruptController, InterruptSource, InterruptLevel};
pub use uart::Uart;
pub use timer::Timer;
pub use gpio::Gpio;

// Implement PeripheralBusDispatch for PeripheralBus
impl flexers_core::memory::PeripheralBusDispatch for PeripheralBus {
    fn dispatch_read(&self, addr: u32, size: u8) -> Option<u32> {
        self.dispatch_read(addr, size)
    }

    fn dispatch_write(&mut self, addr: u32, size: u8, val: u32) -> bool {
        self.dispatch_write(addr, size, val)
    }
}

// Implement InterruptControllerTrait for InterruptController
impl flexers_core::cpu::InterruptControllerTrait for InterruptController {
    fn get_pending_interrupt(&self) -> Option<(u8, u8)> {
        self.get_pending_interrupt().map(|(source, level)| {
            (source as u8, level)
        })
    }

    fn set_current_level(&mut self, level: u8) {
        self.set_current_level(level);
    }
}

// Implement InterruptRaiser for InterruptController
impl uart::InterruptRaiser for InterruptController {
    fn raise(&mut self, source: InterruptSource) {
        self.raise(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peripheral_addresses() {
        // Verify addresses don't overlap
        assert!(UART0_BASE < UART1_BASE);
        assert!(UART1_BASE < UART2_BASE);

        // Verify addresses are in valid ESP32 peripheral range
        assert!(UART0_BASE >= 0x3FF0_0000);
        assert!(GPIO_BASE >= 0x3FF0_0000);
        assert!(TIMER_GROUP0_BASE >= 0x3FF0_0000);
    }
}
