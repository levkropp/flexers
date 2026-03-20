use flexers_core::memory::MmioHandler;

/// DAC register offsets (relative to base address)
const DAC_CONF_REG: u32 = 0x000;      // Configuration register
const DAC1_REG: u32 = 0x004;          // DAC channel 1 data (GPIO 25)
const DAC2_REG: u32 = 0x008;          // DAC channel 2 data (GPIO 26)
const DAC_CTRL_REG: u32 = 0x00C;      // Control register

/// Configuration register bits
const DAC_CONF_ENABLE1: u32 = 1 << 0;     // Enable DAC channel 1
const DAC_CONF_ENABLE2: u32 = 1 << 1;     // Enable DAC channel 2
const DAC_CONF_CW_EN1: u32 = 1 << 2;      // Enable cosine wave generator for channel 1
const DAC_CONF_CW_EN2: u32 = 1 << 3;      // Enable cosine wave generator for channel 2

/// Control register bits
const DAC_CTRL_RESET: u32 = 1 << 0;       // Reset DAC

/// DAC data mask (8-bit)
const DAC_DATA_MASK: u32 = 0xFF;

/// Digital-to-Analog Converter
/// ESP32 has 2 DAC channels:
/// - Channel 1: GPIO 25
/// - Channel 2: GPIO 26
/// Each channel has 8-bit resolution (0-255), corresponding to 0-3.3V output
pub struct Dac {
    /// Channel 1 value (0-255)
    channel1: u8,

    /// Channel 2 value (0-255)
    channel2: u8,

    /// Channel 1 enabled
    enabled1: bool,

    /// Channel 2 enabled
    enabled2: bool,

    /// Cosine wave generator enabled for channel 1
    cw_enable1: bool,

    /// Cosine wave generator enabled for channel 2
    cw_enable2: bool,

    /// Configuration register value
    config: u32,

    /// Control register value
    control: u32,

    /// Cosine wave phase (for simulation)
    cw_phase: f32,
}

impl Dac {
    /// Create new DAC peripheral
    pub fn new() -> Self {
        Self {
            channel1: 0,
            channel2: 0,
            enabled1: false,
            enabled2: false,
            cw_enable1: false,
            cw_enable2: false,
            config: 0,
            control: 0,
            cw_phase: 0.0,
        }
    }

    /// Get channel 1 value (for testing/monitoring)
    pub fn get_channel1(&self) -> u8 {
        if self.enabled1 {
            self.channel1
        } else {
            0
        }
    }

    /// Get channel 2 value (for testing/monitoring)
    pub fn get_channel2(&self) -> u8 {
        if self.enabled2 {
            self.channel2
        } else {
            0
        }
    }

    /// Set channel 1 value
    fn set_channel1(&mut self, value: u8) {
        self.channel1 = value;
    }

    /// Set channel 2 value
    fn set_channel2(&mut self, value: u8) {
        self.channel2 = value;
    }

    /// Update cosine wave generator (called periodically in real hardware)
    /// For emulation, this is simplified
    pub fn tick_cosine_wave(&mut self) {
        if self.cw_enable1 || self.cw_enable2 {
            // Simple cosine wave simulation
            self.cw_phase += 0.1;
            if self.cw_phase > 2.0 * std::f32::consts::PI {
                self.cw_phase -= 2.0 * std::f32::consts::PI;
            }

            let cos_value = ((self.cw_phase.cos() + 1.0) * 127.5) as u8;

            if self.cw_enable1 {
                self.channel1 = cos_value;
            }
            if self.cw_enable2 {
                self.channel2 = cos_value;
            }
        }
    }

    /// Reset DAC to default state
    fn reset(&mut self) {
        self.channel1 = 0;
        self.channel2 = 0;
        self.enabled1 = false;
        self.enabled2 = false;
        self.cw_enable1 = false;
        self.cw_enable2 = false;
        self.config = 0;
        self.control = 0;
        self.cw_phase = 0.0;
    }
}

impl Default for Dac {
    fn default() -> Self {
        Self::new()
    }
}

impl MmioHandler for Dac {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFFF {
            DAC_CONF_REG => self.config,
            DAC1_REG => self.channel1 as u32,
            DAC2_REG => self.channel2 as u32,
            DAC_CTRL_REG => self.control,
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFFF {
            DAC_CONF_REG => {
                self.config = val;

                // Update enable flags
                self.enabled1 = (val & DAC_CONF_ENABLE1) != 0;
                self.enabled2 = (val & DAC_CONF_ENABLE2) != 0;

                // Update cosine wave enable flags
                self.cw_enable1 = (val & DAC_CONF_CW_EN1) != 0;
                self.cw_enable2 = (val & DAC_CONF_CW_EN2) != 0;
            }
            DAC1_REG => {
                // Only update if not using cosine wave generator
                if !self.cw_enable1 {
                    self.set_channel1((val & DAC_DATA_MASK) as u8);
                }
            }
            DAC2_REG => {
                // Only update if not using cosine wave generator
                if !self.cw_enable2 {
                    self.set_channel2((val & DAC_DATA_MASK) as u8);
                }
            }
            DAC_CTRL_REG => {
                self.control = val;

                // Check for reset
                if (val & DAC_CTRL_RESET) != 0 {
                    self.reset();
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dac_creation() {
        let dac = Dac::new();
        assert_eq!(dac.channel1, 0);
        assert_eq!(dac.channel2, 0);
        assert!(!dac.enabled1);
        assert!(!dac.enabled2);
    }

    #[test]
    fn test_channel1_write() {
        let mut dac = Dac::new();

        // Enable channel 1
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1);

        // Write value to channel 1
        dac.write(DAC1_REG, 4, 128);

        assert_eq!(dac.get_channel1(), 128);
        assert_eq!(dac.read(DAC1_REG, 4), 128);
    }

    #[test]
    fn test_channel2_write() {
        let mut dac = Dac::new();

        // Enable channel 2
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE2);

        // Write value to channel 2
        dac.write(DAC2_REG, 4, 200);

        assert_eq!(dac.get_channel2(), 200);
        assert_eq!(dac.read(DAC2_REG, 4), 200);
    }

    #[test]
    fn test_both_channels() {
        let mut dac = Dac::new();

        // Enable both channels
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1 | DAC_CONF_ENABLE2);

        // Write different values
        dac.write(DAC1_REG, 4, 100);
        dac.write(DAC2_REG, 4, 150);

        assert_eq!(dac.get_channel1(), 100);
        assert_eq!(dac.get_channel2(), 150);
    }

    #[test]
    fn test_enable_disable() {
        let mut dac = Dac::new();

        // Write value while disabled
        dac.write(DAC1_REG, 4, 123);

        // Should not output (disabled)
        assert_eq!(dac.get_channel1(), 0);

        // Enable
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1);

        // Now should output
        assert_eq!(dac.get_channel1(), 123);

        // Disable
        dac.write(DAC_CONF_REG, 4, 0);

        // Should not output
        assert_eq!(dac.get_channel1(), 0);
    }

    #[test]
    fn test_8bit_mask() {
        let mut dac = Dac::new();

        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1);

        // Write value larger than 8-bit
        dac.write(DAC1_REG, 4, 0x1234);

        // Should be masked to 8-bit
        assert_eq!(dac.get_channel1(), 0x34);
    }

    #[test]
    fn test_reset() {
        let mut dac = Dac::new();

        // Configure DAC
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1 | DAC_CONF_ENABLE2);
        dac.write(DAC1_REG, 4, 100);
        dac.write(DAC2_REG, 4, 200);

        // Verify state
        assert!(dac.enabled1);
        assert!(dac.enabled2);
        assert_eq!(dac.channel1, 100);
        assert_eq!(dac.channel2, 200);

        // Reset
        dac.write(DAC_CTRL_REG, 4, DAC_CTRL_RESET);

        // Verify reset
        assert!(!dac.enabled1);
        assert!(!dac.enabled2);
        assert_eq!(dac.channel1, 0);
        assert_eq!(dac.channel2, 0);
    }

    #[test]
    fn test_cosine_wave_mode() {
        let mut dac = Dac::new();

        // Enable channel 1 with cosine wave
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1 | DAC_CONF_CW_EN1);

        // Direct writes should be ignored in cosine wave mode
        dac.write(DAC1_REG, 4, 123);

        // Channel value should be 0 initially (not 123)
        assert_ne!(dac.channel1, 123);

        // Tick cosine wave generator
        dac.tick_cosine_wave();

        // Value should be from cosine wave (not direct write)
        assert_ne!(dac.channel1, 0);
        assert_ne!(dac.channel1, 123);
    }

    #[test]
    fn test_value_range() {
        let mut dac = Dac::new();

        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1 | DAC_CONF_ENABLE2);

        // Test minimum value
        dac.write(DAC1_REG, 4, 0);
        assert_eq!(dac.get_channel1(), 0);

        // Test maximum value
        dac.write(DAC1_REG, 4, 255);
        assert_eq!(dac.get_channel1(), 255);

        // Test mid value
        dac.write(DAC2_REG, 4, 127);
        assert_eq!(dac.get_channel2(), 127);
    }

    #[test]
    fn test_register_read_write() {
        let mut dac = Dac::new();

        // Write to config register
        dac.write(DAC_CONF_REG, 4, DAC_CONF_ENABLE1 | DAC_CONF_ENABLE2);
        assert_eq!(dac.read(DAC_CONF_REG, 4), DAC_CONF_ENABLE1 | DAC_CONF_ENABLE2);

        // Write to data registers
        dac.write(DAC1_REG, 4, 42);
        dac.write(DAC2_REG, 4, 84);

        assert_eq!(dac.read(DAC1_REG, 4), 42);
        assert_eq!(dac.read(DAC2_REG, 4), 84);
    }
}
