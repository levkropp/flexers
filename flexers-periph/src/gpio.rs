use flexers_core::memory::MmioHandler;
use crate::interrupt::InterruptSource;
use crate::uart::InterruptRaiser;
use std::sync::{Arc, Mutex};

/// GPIO register offsets
const GPIO_IN_LOW: u32 = 0x00;
const GPIO_IN_HIGH: u32 = 0x04;
const GPIO_OUT_LOW: u32 = 0x08;
const GPIO_OUT_HIGH: u32 = 0x0C;
const GPIO_ENABLE_LOW: u32 = 0x10;
const GPIO_ENABLE_HIGH: u32 = 0x14;

/// GPIO pin mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum PinMode {
    Input,
    Output,
    InputOutput,
}

/// GPIO interrupt type
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum IntType {
    Disabled = 0,
    RisingEdge = 1,
    FallingEdge = 2,
    AnyEdge = 3,
    LowLevel = 4,
    HighLevel = 5,
}

/// GPIO peripheral (40 pins on ESP32)
pub struct Gpio {
    /// Pin modes (input/output)
    pin_modes: [PinMode; 40],

    /// Output levels (for output pins)
    output_levels: u64, // Bitfield, 1 = high

    /// Input levels (for input pins)
    input_levels: u64,

    /// Output enable (1 = output enabled)
    output_enable: u64,

    /// Interrupt enable per pin
    int_enable: u64,

    /// Interrupt type (rising/falling/change)
    int_type: [IntType; 40],

    /// Previous input state (for edge detection)
    prev_input: u64,

    /// Interrupt raiser
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,
}

impl Gpio {
    pub fn new() -> Self {
        Self {
            pin_modes: [PinMode::Input; 40],
            output_levels: 0,
            input_levels: 0,
            output_enable: 0,
            int_enable: 0,
            int_type: [IntType::Disabled; 40],
            prev_input: 0,
            int_raiser: None,
        }
    }

    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Set input level for a pin (for simulation/testing)
    pub fn set_input(&mut self, pin: u8, level: bool) {
        if pin < 40 {
            if level {
                self.input_levels |= 1u64 << pin;
            } else {
                self.input_levels &= !(1u64 << pin);
            }

            // Check for edge detection
            self.check_interrupt(pin);
        }
    }

    /// Get output level for a pin (for simulation/testing)
    pub fn get_output(&self, pin: u8) -> bool {
        if pin < 40 {
            (self.output_levels & (1u64 << pin)) != 0
        } else {
            false
        }
    }

    fn check_interrupt(&mut self, pin: u8) {
        if (self.int_enable & (1u64 << pin)) == 0 {
            return;
        }

        let current = (self.input_levels & (1u64 << pin)) != 0;
        let prev = (self.prev_input & (1u64 << pin)) != 0;

        let triggered = match self.int_type[pin as usize] {
            IntType::RisingEdge => current && !prev,
            IntType::FallingEdge => !current && prev,
            IntType::AnyEdge => current != prev,
            IntType::LowLevel => !current,
            IntType::HighLevel => current,
            IntType::Disabled => false,
        };

        if triggered {
            if let Some(ref raiser) = self.int_raiser {
                if let Ok(mut raiser_lock) = raiser.lock() {
                    raiser_lock.raise(InterruptSource::Gpio);
                }
            }
        }

        self.prev_input = self.input_levels;
    }
}

impl Default for Gpio {
    fn default() -> Self {
        Self::new()
    }
}

impl MmioHandler for Gpio {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFF {
            0x00 => (self.input_levels & 0xFFFFFFFF) as u32,   // Input low
            0x04 => ((self.input_levels >> 32) & 0xFF) as u32, // Input high (only 8 bits)
            0x08 => (self.output_levels & 0xFFFFFFFF) as u32,  // Output low
            0x0C => ((self.output_levels >> 32) & 0xFF) as u32, // Output high
            0x10 => (self.output_enable & 0xFFFFFFFF) as u32,  // Enable low
            0x14 => ((self.output_enable >> 32) & 0xFF) as u32, // Enable high
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFF {
            0x08 => { // Output low
                self.output_levels = (self.output_levels & 0xFFFFFFFF00000000) | val as u64;
            }
            0x0C => { // Output high
                self.output_levels = (self.output_levels & 0x00000000FFFFFFFF) | ((val as u64 & 0xFF) << 32);
            }
            0x10 => { // Enable low
                self.output_enable = (self.output_enable & 0xFFFFFFFF00000000) | val as u64;
            }
            0x14 => { // Enable high
                self.output_enable = (self.output_enable & 0x00000000FFFFFFFF) | ((val as u64 & 0xFF) << 32);
            }
            // Pin mode configuration would go here (registers 0x20-0xFF)
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uart::InterruptRaiser;

    struct DummyInterruptRaiser {
        raised: Vec<InterruptSource>,
    }

    impl InterruptRaiser for DummyInterruptRaiser {
        fn raise(&mut self, source: InterruptSource) {
            self.raised.push(source);
        }
    }

    #[test]
    fn test_gpio_output() {
        let mut gpio = Gpio::new();

        // Enable pin 5 as output
        gpio.write(0x10, 4, 1 << 5);

        // Set pin 5 high
        gpio.write(0x08, 4, 1 << 5);

        // Read back output level
        let output = gpio.read(0x08, 4);
        assert_eq!(output & (1 << 5), 1 << 5);

        // Verify via get_output
        assert!(gpio.get_output(5));
    }

    #[test]
    fn test_gpio_input() {
        let mut gpio = Gpio::new();

        // Set input level for pin 3
        gpio.set_input(3, true);

        // Read input register
        let input = gpio.read(0x00, 4);
        assert_eq!(input & (1 << 3), 1 << 3);

        // Set to low
        gpio.set_input(3, false);
        let input = gpio.read(0x00, 4);
        assert_eq!(input & (1 << 3), 0);
    }

    #[test]
    fn test_gpio_interrupt_rising_edge() {
        let mut gpio = Gpio::new();
        let raiser = Arc::new(Mutex::new(DummyInterruptRaiser { raised: Vec::new() }));
        gpio.set_interrupt_raiser(raiser.clone());

        // Configure pin 2 for rising edge interrupt
        gpio.int_enable = 1 << 2;
        gpio.int_type[2] = IntType::RisingEdge;

        // Set to low initially
        gpio.set_input(2, false);

        // No interrupt yet
        let raiser_lock = raiser.lock().unwrap();
        assert!(raiser_lock.raised.is_empty());
        drop(raiser_lock);

        // Rising edge - should trigger
        gpio.set_input(2, true);

        let raiser_lock = raiser.lock().unwrap();
        assert_eq!(raiser_lock.raised.len(), 1);
        assert_eq!(raiser_lock.raised[0] as u8, InterruptSource::Gpio as u8);
        drop(raiser_lock);

        // Another high - no new interrupt
        gpio.set_input(2, true);

        let raiser_lock = raiser.lock().unwrap();
        assert_eq!(raiser_lock.raised.len(), 1);
    }

    #[test]
    fn test_gpio_interrupt_falling_edge() {
        let mut gpio = Gpio::new();
        let raiser = Arc::new(Mutex::new(DummyInterruptRaiser { raised: Vec::new() }));
        gpio.set_interrupt_raiser(raiser.clone());

        // Configure pin 4 for falling edge interrupt
        gpio.int_enable = 1 << 4;
        gpio.int_type[4] = IntType::FallingEdge;

        // Set to high initially
        gpio.set_input(4, true);

        // No interrupt yet
        let raiser_lock = raiser.lock().unwrap();
        assert!(raiser_lock.raised.is_empty());
        drop(raiser_lock);

        // Falling edge - should trigger
        gpio.set_input(4, false);

        let raiser_lock = raiser.lock().unwrap();
        assert_eq!(raiser_lock.raised.len(), 1);
    }

    #[test]
    fn test_gpio_high_pins() {
        let mut gpio = Gpio::new();

        // Set pin 35 (in high register) as output
        gpio.write(0x14, 4, 1 << (35 - 32));

        // Set pin 35 high
        gpio.write(0x0C, 4, 1 << (35 - 32));

        // Read back
        let output_high = gpio.read(0x0C, 4);
        assert_eq!(output_high & (1 << (35 - 32)), 1 << (35 - 32));
    }
}
