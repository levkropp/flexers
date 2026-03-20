pub mod bus;
pub mod uart;
pub mod timer;
pub mod gpio;
pub mod interrupt;
pub mod spi_flash;
pub mod adc;
pub mod dac;
pub mod ledc;
pub mod i2c;
pub mod dma;
pub mod spi;
pub mod touch;
pub mod rmt;

// ESP32 peripheral base addresses
pub const UART0_BASE: u32 = 0x3FF4_0000;
pub const UART1_BASE: u32 = 0x3FF5_0000;
pub const UART2_BASE: u32 = 0x3FF6_E000;
pub const GPIO_BASE: u32 = 0x3FF4_4000;
pub const TIMER_GROUP0_BASE: u32 = 0x3FF5_F000;
pub const TIMER_GROUP1_BASE: u32 = 0x3FF6_0000;
pub const RTC_BASE: u32 = 0x3FF4_8000;
pub const INTERRUPT_BASE: u32 = 0x3FF0_0000; // PRO CPU interrupt matrix
pub const SPI0_BASE: u32 = 0x3FF4_2000; // SPI0 flash cache controller
pub const SPI1_BASE: u32 = 0x3FF4_3000; // SPI1 general purpose flash
pub const ADC_BASE: u32 = 0x3FF4_8800; // ADC controller
pub const DAC_BASE: u32 = 0x3FF4_8820; // DAC controller
pub const LEDC_BASE: u32 = 0x3FF5_9000; // LED PWM controller
pub const I2C0_BASE: u32 = 0x3FF5_3000; // I2C controller 0
pub const I2C1_BASE: u32 = 0x3FF6_7000; // I2C controller 1
pub const DMA_BASE: u32 = 0x3FF4_E000; // DMA controller
pub const SPI2_BASE: u32 = 0x3FF6_4000; // SPI2 general purpose
pub const SPI3_BASE: u32 = 0x3FF6_5000; // SPI3 general purpose
pub const TOUCH_BASE: u32 = 0x3FF4_8800; // Touch sensor (overlaps with RTC/ADC region)
pub const RMT_BASE: u32 = 0x3FF5_6000; // RMT peripheral

// Re-export commonly used types
pub use bus::{PeripheralBus, AddrRange};
pub use interrupt::{InterruptController, InterruptSource, InterruptLevel, InterruptRaiser};
pub use uart::Uart;
pub use timer::Timer;
pub use gpio::Gpio;
pub use spi_flash::SpiFlash;
pub use adc::Adc;
pub use dac::Dac;
pub use ledc::Ledc;
pub use i2c::I2c;
pub use dma::Dma;
pub use spi::Spi;
pub use touch::Touch;
pub use rmt::Rmt;

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
impl InterruptRaiser for InterruptController {
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
