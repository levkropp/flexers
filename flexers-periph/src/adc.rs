use flexers_core::memory::MmioHandler;
use std::sync::{Arc, Mutex};

/// ADC register offsets (relative to base address)
const ADC_CONF_REG: u32 = 0x000;      // Configuration register
const ADC_CTRL_REG: u32 = 0x004;      // Control register (channel selection, start)
const ADC_DATA_REG: u32 = 0x008;      // Conversion result register
const ADC_STATUS_REG: u32 = 0x00C;    // Status register
const ADC_SAR1_PATT_TAB1: u32 = 0x010; // Pattern table for SAR ADC1
const ADC_SAR1_PATT_TAB2: u32 = 0x014;
const ADC_SAR1_PATT_TAB3: u32 = 0x018;
const ADC_SAR1_PATT_TAB4: u32 = 0x01C;

/// Configuration register bits
const ADC_CONF_START_CONV: u32 = 1 << 0;    // Start conversion
const ADC_CONF_RESET: u32 = 1 << 1;          // Reset ADC
const ADC_CONF_ULP_MODE: u32 = 1 << 2;       // Ultra-low-power mode

/// Control register bits
const ADC_CTRL_CHANNEL_MASK: u32 = 0xF;      // Channel select (bits 0-3)
const ADC_CTRL_ATTEN_SHIFT: u32 = 4;         // Attenuation shift
const ADC_CTRL_ATTEN_MASK: u32 = 0x3 << ADC_CTRL_ATTEN_SHIFT;
const ADC_CTRL_WIDTH_SHIFT: u32 = 6;         // Width shift
const ADC_CTRL_WIDTH_MASK: u32 = 0x3 << ADC_CTRL_WIDTH_SHIFT;

/// Status register bits
const ADC_STATUS_DONE: u32 = 1 << 0;         // Conversion done
const ADC_STATUS_BUSY: u32 = 1 << 1;         // ADC busy

/// Attenuation values
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdcAttenuation {
    Db0,    // 0dB attenuation (100mV ~ 950mV)
    Db2_5,  // 2.5dB attenuation (100mV ~ 1250mV)
    Db6,    // 6dB attenuation (150mV ~ 1750mV)
    Db11,   // 11dB attenuation (150mV ~ 2450mV)
}

impl From<u8> for AdcAttenuation {
    fn from(val: u8) -> Self {
        match val & 0x3 {
            0 => AdcAttenuation::Db0,
            1 => AdcAttenuation::Db2_5,
            2 => AdcAttenuation::Db6,
            3 => AdcAttenuation::Db11,
            _ => unreachable!(),
        }
    }
}

/// ADC width/resolution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdcWidth {
    Bit9,   // 9-bit (0-511)
    Bit10,  // 10-bit (0-1023)
    Bit11,  // 11-bit (0-2047)
    Bit12,  // 12-bit (0-4095)
}

impl AdcWidth {
    pub fn max_value(&self) -> u16 {
        match self {
            AdcWidth::Bit9 => 511,
            AdcWidth::Bit10 => 1023,
            AdcWidth::Bit11 => 2047,
            AdcWidth::Bit12 => 4095,
        }
    }
}

impl From<u8> for AdcWidth {
    fn from(val: u8) -> Self {
        match val & 0x3 {
            0 => AdcWidth::Bit9,
            1 => AdcWidth::Bit10,
            2 => AdcWidth::Bit11,
            3 => AdcWidth::Bit12,
            _ => unreachable!(),
        }
    }
}

/// SAR ADC peripheral (Successive Approximation Register)
/// ESP32 has two SAR ADCs, this implements ADC1 with 8 channels (GPIO 32-39)
pub struct Adc {
    /// Current selected channel (0-7 for ADC1)
    channel: u8,

    /// Attenuation setting
    attenuation: AdcAttenuation,

    /// Resolution/width
    width: AdcWidth,

    /// Last conversion result
    data: u16,

    /// Configuration register value
    config: u32,

    /// Control register value
    control: u32,

    /// Status register value
    status: u32,

    /// Conversion in progress
    busy: bool,

    /// Simulated channel values (for testing)
    /// In a real emulator, these could be connected to external inputs
    channel_values: [u16; 8],
}

impl Adc {
    /// Create new ADC peripheral
    pub fn new() -> Self {
        Self {
            channel: 0,
            attenuation: AdcAttenuation::Db0,
            width: AdcWidth::Bit12,
            data: 0,
            config: 0,
            control: 0,
            status: 0,
            busy: false,
            // Initialize with mid-scale values
            channel_values: [2048; 8],
        }
    }

    /// Set simulated value for a channel (for testing)
    /// Note: This sets the raw analog value. Width limiting is applied during conversion.
    pub fn set_channel_value(&mut self, channel: u8, value: u16) {
        if channel < 8 {
            self.channel_values[channel as usize] = value;
        }
    }

    /// Get channel value (for testing)
    pub fn get_channel_value(&self, channel: u8) -> Option<u16> {
        if channel < 8 {
            Some(self.channel_values[channel as usize])
        } else {
            None
        }
    }

    /// Start ADC conversion
    fn start_conversion(&mut self) {
        self.busy = true;
        self.status = ADC_STATUS_BUSY;

        // In hardware, this would take time. In emulation, it's instant.
        // Read the current channel value
        if (self.channel as usize) < self.channel_values.len() {
            self.data = self.channel_values[self.channel as usize];

            // Apply attenuation scaling (simplified simulation)
            // In reality, attenuation affects input voltage range, not output scale
            // For simulation, we just ensure the value is within width limits
            self.data = self.data.min(self.width.max_value());
        } else {
            self.data = 0;
        }

        // Mark conversion as done
        self.busy = false;
        self.status = ADC_STATUS_DONE;
    }

    /// Reset ADC to default state
    fn reset(&mut self) {
        self.channel = 0;
        self.attenuation = AdcAttenuation::Db0;
        self.width = AdcWidth::Bit12;
        self.data = 0;
        self.config = 0;
        self.control = 0;
        self.status = 0;
        self.busy = false;
        // Keep channel values (simulated analog inputs)
    }
}

impl Default for Adc {
    fn default() -> Self {
        Self::new()
    }
}

impl MmioHandler for Adc {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFFF {
            ADC_CONF_REG => self.config,
            ADC_CTRL_REG => self.control,
            ADC_DATA_REG => {
                // Reading data register clears DONE flag in some implementations
                // For simplicity, we just return the data
                self.data as u32
            }
            ADC_STATUS_REG => self.status,
            ADC_SAR1_PATT_TAB1 | ADC_SAR1_PATT_TAB2 |
            ADC_SAR1_PATT_TAB3 | ADC_SAR1_PATT_TAB4 => {
                // Pattern tables not fully implemented
                0
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFFF {
            ADC_CONF_REG => {
                self.config = val;

                // Check for start conversion
                if (val & ADC_CONF_START_CONV) != 0 {
                    self.start_conversion();
                    // Clear start bit after conversion
                    self.config &= !ADC_CONF_START_CONV;
                }

                // Check for reset
                if (val & ADC_CONF_RESET) != 0 {
                    self.reset();
                }
            }
            ADC_CTRL_REG => {
                self.control = val;

                // Extract channel
                self.channel = (val & ADC_CTRL_CHANNEL_MASK) as u8;

                // Extract attenuation
                let atten = ((val & ADC_CTRL_ATTEN_MASK) >> ADC_CTRL_ATTEN_SHIFT) as u8;
                self.attenuation = AdcAttenuation::from(atten);

                // Extract width
                let width = ((val & ADC_CTRL_WIDTH_MASK) >> ADC_CTRL_WIDTH_SHIFT) as u8;
                self.width = AdcWidth::from(width);
            }
            ADC_DATA_REG => {
                // Data register is read-only, ignore writes
            }
            ADC_STATUS_REG => {
                // Writing to status can clear flags
                if (val & ADC_STATUS_DONE) == 0 {
                    self.status &= !ADC_STATUS_DONE;
                }
            }
            ADC_SAR1_PATT_TAB1 | ADC_SAR1_PATT_TAB2 |
            ADC_SAR1_PATT_TAB3 | ADC_SAR1_PATT_TAB4 => {
                // Pattern tables not fully implemented
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc_creation() {
        let adc = Adc::new();
        assert_eq!(adc.channel, 0);
        assert_eq!(adc.attenuation, AdcAttenuation::Db0);
        assert_eq!(adc.width, AdcWidth::Bit12);
        assert_eq!(adc.data, 0);
    }

    #[test]
    fn test_channel_selection() {
        let mut adc = Adc::new();

        // Select channel 3
        adc.write(ADC_CTRL_REG, 4, 3);
        assert_eq!(adc.channel, 3);

        // Select channel 7
        adc.write(ADC_CTRL_REG, 4, 7);
        assert_eq!(adc.channel, 7);

        // Invalid channel should be masked
        adc.write(ADC_CTRL_REG, 4, 0xFF);
        assert_eq!(adc.channel, 15 & 0xF); // Masked to 4 bits
    }

    #[test]
    fn test_attenuation_config() {
        let mut adc = Adc::new();

        // Set 2.5dB attenuation
        let val = (1 << ADC_CTRL_ATTEN_SHIFT);
        adc.write(ADC_CTRL_REG, 4, val);
        assert_eq!(adc.attenuation, AdcAttenuation::Db2_5);

        // Set 11dB attenuation
        let val = (3 << ADC_CTRL_ATTEN_SHIFT);
        adc.write(ADC_CTRL_REG, 4, val);
        assert_eq!(adc.attenuation, AdcAttenuation::Db11);
    }

    #[test]
    fn test_width_config() {
        let mut adc = Adc::new();

        // Set 10-bit width
        let val = (1 << ADC_CTRL_WIDTH_SHIFT);
        adc.write(ADC_CTRL_REG, 4, val);
        assert_eq!(adc.width, AdcWidth::Bit10);
        assert_eq!(adc.width.max_value(), 1023);

        // Set 9-bit width
        let val = (0 << ADC_CTRL_WIDTH_SHIFT);
        adc.write(ADC_CTRL_REG, 4, val);
        assert_eq!(adc.width, AdcWidth::Bit9);
        assert_eq!(adc.width.max_value(), 511);
    }

    #[test]
    fn test_conversion() {
        let mut adc = Adc::new();

        // Set a known value for channel 0
        adc.set_channel_value(0, 1234);

        // Select channel 0 with 12-bit width (width bits = 3)
        let width_12bit = 3 << ADC_CTRL_WIDTH_SHIFT;
        adc.write(ADC_CTRL_REG, 4, 0 | width_12bit);

        // Start conversion
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);

        // Check status
        let status = adc.read(ADC_STATUS_REG, 4);
        assert_ne!(status & ADC_STATUS_DONE, 0);

        // Read data
        let data = adc.read(ADC_DATA_REG, 4);
        assert_eq!(data, 1234);
    }

    #[test]
    fn test_multiple_channels() {
        let mut adc = Adc::new();

        // Set different values for different channels
        adc.set_channel_value(0, 100);
        adc.set_channel_value(1, 200);
        adc.set_channel_value(2, 300);
        adc.set_channel_value(3, 400);

        // Read channel 0
        adc.write(ADC_CTRL_REG, 4, 0);
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);
        assert_eq!(adc.read(ADC_DATA_REG, 4), 100);

        // Read channel 1
        adc.write(ADC_CTRL_REG, 4, 1);
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);
        assert_eq!(adc.read(ADC_DATA_REG, 4), 200);

        // Read channel 2
        adc.write(ADC_CTRL_REG, 4, 2);
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);
        assert_eq!(adc.read(ADC_DATA_REG, 4), 300);

        // Read channel 3
        adc.write(ADC_CTRL_REG, 4, 3);
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);
        assert_eq!(adc.read(ADC_DATA_REG, 4), 400);
    }

    #[test]
    fn test_reset() {
        let mut adc = Adc::new();

        // Configure ADC
        adc.write(ADC_CTRL_REG, 4, 5 | (2 << ADC_CTRL_ATTEN_SHIFT));
        adc.set_channel_value(5, 3000);
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);

        // Verify it has state
        assert_eq!(adc.channel, 5);
        assert_ne!(adc.data, 0);

        // Reset
        adc.write(ADC_CONF_REG, 4, ADC_CONF_RESET);

        // Verify reset
        assert_eq!(adc.channel, 0);
        assert_eq!(adc.data, 0);
        assert_eq!(adc.attenuation, AdcAttenuation::Db0);

        // Channel values should be preserved (they represent external analog inputs)
        assert_eq!(adc.get_channel_value(5), Some(3000));
    }

    #[test]
    fn test_width_limiting() {
        let mut adc = Adc::new();

        // Set a value larger than 10-bit max
        adc.set_channel_value(0, 4095);

        // Configure for 10-bit width
        adc.write(ADC_CTRL_REG, 4, 0 | (1 << ADC_CTRL_WIDTH_SHIFT));

        // Start conversion
        adc.write(ADC_CONF_REG, 4, ADC_CONF_START_CONV);

        // Result should be limited to 10-bit max (1023)
        let data = adc.read(ADC_DATA_REG, 4);
        assert_eq!(data, 1023);
    }

    #[test]
    fn test_register_read_write() {
        let mut adc = Adc::new();

        // Write to control register
        adc.write(ADC_CTRL_REG, 4, 0x12345678);
        let val = adc.read(ADC_CTRL_REG, 4);
        assert_eq!(val, 0x12345678);

        // Write to config register
        adc.write(ADC_CONF_REG, 4, 0xABCD);
        let val = adc.read(ADC_CONF_REG, 4);
        // Start bit should be cleared after conversion
        assert_eq!(val & !ADC_CONF_START_CONV, 0xABCD & !ADC_CONF_START_CONV);
    }
}
