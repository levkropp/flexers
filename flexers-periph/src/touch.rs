use flexers_core::memory::MmioHandler;
use crate::interrupt::{InterruptSource, InterruptRaiser};
use std::sync::{Arc, Mutex};

/// Touch sensor register offsets (simplified)
const SENS_SAR_TOUCH_CONF_REG: u32 = 0x100;    // Touch configuration
const SENS_SAR_TOUCH_ENABLE_REG: u32 = 0x104;  // Channel enable mask
const SENS_SAR_TOUCH_CTRL_REG: u32 = 0x108;    // Control register
const SENS_SAR_TOUCH_OUT_BASE: u32 = 0x200;    // Touch measurement output base (10 channels)
const SENS_SAR_TOUCH_THRES_BASE: u32 = 0x240;  // Touch threshold base (10 channels)

/// Touch interrupt register
const RTC_CNTL_INT_RAW_REG: u32 = 0x010;       // Interrupt raw status
const RTC_CNTL_INT_ENA_REG: u32 = 0x014;       // Interrupt enable
const RTC_CNTL_INT_CLR_REG: u32 = 0x018;       // Interrupt clear

/// Configuration register bits
const TOUCH_START: u32 = 1 << 0;               // Start measurement
const TOUCH_FILTER_EN: u32 = 1 << 1;           // Enable filter
const TOUCH_XPD_WAIT_MASK: u32 = 0xFF << 8;    // Wait cycles

/// Interrupt bits
const TOUCH_INT_DONE: u32 = 1 << 0;            // Measurement done
const TOUCH_INT_ACTIVE: u32 = 1 << 1;          // Touch detected

/// Touch pad channel
#[derive(Debug, Clone)]
struct TouchChannel {
    /// Channel enabled
    enabled: bool,

    /// Current touch value (raw)
    value: u16,

    /// Threshold value
    threshold: u16,

    /// Touch state (pressed or not)
    touched: bool,
}

impl TouchChannel {
    fn new() -> Self {
        Self {
            enabled: false,
            value: 1000, // Default idle value
            threshold: 800, // Default threshold
            touched: false,
        }
    }

    /// Update touch state based on threshold
    fn update_state(&mut self) {
        self.touched = self.value < self.threshold;
    }

    /// Simulate a touch value read
    fn read_value(&self) -> u16 {
        self.value
    }

    /// Set the simulated touch value
    fn set_value(&mut self, value: u16) {
        self.value = value;
        self.update_state();
    }
}

/// Touch sensor controller
/// Supports 10 touch channels on GPIO 0, 2, 4, 12-15, 27, 32, 33
pub struct Touch {
    /// 10 touch channels
    channels: [TouchChannel; 10],

    /// Channel enable mask (bits 0-9)
    enable_mask: u16,

    /// Configuration register
    config: u32,

    /// Control register
    ctrl: u32,

    /// Filter period
    filter_period: u32,

    /// Measurement in progress
    measuring: bool,

    /// Interrupt raw register
    int_raw: u32,

    /// Interrupt enable register
    int_ena: u32,

    /// Interrupt raiser
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,
}

impl Touch {
    /// Create a new touch sensor controller
    pub fn new() -> Self {
        Self {
            channels: [
                TouchChannel::new(), TouchChannel::new(), TouchChannel::new(),
                TouchChannel::new(), TouchChannel::new(), TouchChannel::new(),
                TouchChannel::new(), TouchChannel::new(), TouchChannel::new(),
                TouchChannel::new(),
            ],
            enable_mask: 0,
            config: 0,
            ctrl: 0,
            filter_period: 0,
            measuring: false,
            int_raw: 0,
            int_ena: 0,
            int_raiser: None,
        }
    }

    /// Set interrupt raiser
    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    /// Enable a channel
    pub fn enable_channel(&mut self, channel: u8) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].enabled = true;
            self.enable_mask |= 1 << channel;
        }
    }

    /// Disable a channel
    pub fn disable_channel(&mut self, channel: u8) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].enabled = false;
            self.enable_mask &= !(1 << channel);
        }
    }

    /// Set channel threshold
    pub fn set_threshold(&mut self, channel: u8, threshold: u16) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].threshold = threshold;
            self.channels[channel as usize].update_state();
        }
    }

    /// Get channel value
    pub fn get_value(&self, channel: u8) -> u16 {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].read_value()
        } else {
            0
        }
    }

    /// Simulate setting a touch value (for testing)
    pub fn simulate_touch(&mut self, channel: u8, value: u16) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].set_value(value);
            self.check_and_raise_interrupt();
        }
    }

    /// Check if any channel is touched
    pub fn is_touched(&self, channel: u8) -> bool {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].touched
        } else {
            false
        }
    }

    /// Start measurement
    fn start_measurement(&mut self) {
        self.measuring = true;

        // Simulate measurement completion immediately
        // In real hardware, this would take some time
        for ch in &mut self.channels {
            if ch.enabled {
                ch.update_state();
            }
        }

        self.measuring = false;
        self.int_raw |= TOUCH_INT_DONE;

        // Check if any channel is touched
        for ch in &self.channels {
            if ch.enabled && ch.touched {
                self.int_raw |= TOUCH_INT_ACTIVE;
                break;
            }
        }

        self.check_and_raise_interrupt();
    }

    /// Check and raise interrupt if needed
    fn check_and_raise_interrupt(&mut self) {
        if (self.int_raw & self.int_ena) != 0 {
            if let Some(ref raiser) = self.int_raiser {
                if let Ok(mut r) = raiser.lock() {
                    r.raise(InterruptSource::Touch);
                }
            }
        }
    }
}

impl MmioHandler for Touch {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        let offset = addr & 0xFFF;

        match offset {
            SENS_SAR_TOUCH_CONF_REG => self.config,
            SENS_SAR_TOUCH_ENABLE_REG => self.enable_mask as u32,
            SENS_SAR_TOUCH_CTRL_REG => self.ctrl,
            RTC_CNTL_INT_RAW_REG => self.int_raw,
            RTC_CNTL_INT_ENA_REG => self.int_ena,
            _ if offset >= SENS_SAR_TOUCH_OUT_BASE && offset < SENS_SAR_TOUCH_OUT_BASE + 40 => {
                // Touch output registers (10 channels, 4 bytes each)
                let channel = ((offset - SENS_SAR_TOUCH_OUT_BASE) / 4) as u8;
                self.get_value(channel) as u32
            },
            _ if offset >= SENS_SAR_TOUCH_THRES_BASE && offset < SENS_SAR_TOUCH_THRES_BASE + 40 => {
                // Touch threshold registers (10 channels, 4 bytes each)
                let channel = ((offset - SENS_SAR_TOUCH_THRES_BASE) / 4) as usize;
                if channel < self.channels.len() {
                    self.channels[channel].threshold as u32
                } else {
                    0
                }
            },
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        let offset = addr & 0xFFF;

        match offset {
            SENS_SAR_TOUCH_CONF_REG => {
                self.config = val;
                if (val & TOUCH_START) != 0 {
                    self.start_measurement();
                }
            },
            SENS_SAR_TOUCH_ENABLE_REG => {
                self.enable_mask = val as u16;
                for i in 0..10 {
                    if (val & (1 << i)) != 0 {
                        self.enable_channel(i);
                    } else {
                        self.disable_channel(i);
                    }
                }
            },
            SENS_SAR_TOUCH_CTRL_REG => self.ctrl = val,
            RTC_CNTL_INT_RAW_REG => {
                // Writing to int_raw clears the bits
                self.int_raw &= !val;
            },
            RTC_CNTL_INT_ENA_REG => self.int_ena = val,
            RTC_CNTL_INT_CLR_REG => {
                // Clear interrupts
                self.int_raw &= !val;
            },
            _ if offset >= SENS_SAR_TOUCH_THRES_BASE && offset < SENS_SAR_TOUCH_THRES_BASE + 40 => {
                // Touch threshold registers
                let channel = ((offset - SENS_SAR_TOUCH_THRES_BASE) / 4) as u8;
                self.set_threshold(channel, val as u16);
            },
            _ => {},
        }
    }
}

impl Default for Touch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_creation() {
        let touch = Touch::new();
        assert_eq!(touch.channels.len(), 10);
        assert_eq!(touch.enable_mask, 0);
    }

    #[test]
    fn test_touch_channel_enable() {
        let mut touch = Touch::new();
        touch.enable_channel(0);
        assert!(touch.channels[0].enabled);
        assert_eq!(touch.enable_mask, 1);
    }

    #[test]
    fn test_touch_channel_disable() {
        let mut touch = Touch::new();
        touch.enable_channel(0);
        touch.disable_channel(0);
        assert!(!touch.channels[0].enabled);
        assert_eq!(touch.enable_mask, 0);
    }

    #[test]
    fn test_touch_threshold_setting() {
        let mut touch = Touch::new();
        touch.set_threshold(0, 500);
        assert_eq!(touch.channels[0].threshold, 500);
    }

    #[test]
    fn test_touch_detection() {
        let mut touch = Touch::new();
        touch.enable_channel(0);
        touch.set_threshold(0, 800);

        // Simulate touch (value below threshold)
        touch.simulate_touch(0, 600);
        assert!(touch.is_touched(0));
    }

    #[test]
    fn test_touch_no_detection() {
        let mut touch = Touch::new();
        touch.enable_channel(0);
        touch.set_threshold(0, 800);

        // Simulate no touch (value above threshold)
        touch.simulate_touch(0, 1000);
        assert!(!touch.is_touched(0));
    }

    #[test]
    fn test_touch_multiple_channels() {
        let mut touch = Touch::new();

        touch.enable_channel(0);
        touch.enable_channel(1);
        touch.enable_channel(2);

        assert_eq!(touch.enable_mask, 0b111);
        assert!(touch.channels[0].enabled);
        assert!(touch.channels[1].enabled);
        assert!(touch.channels[2].enabled);
    }

    #[test]
    fn test_touch_measurement() {
        let mut touch = Touch::new();
        touch.enable_channel(0);

        // Start measurement via MMIO
        touch.write(SENS_SAR_TOUCH_CONF_REG, 4, TOUCH_START);

        // Should set done interrupt
        assert_ne!(touch.int_raw & TOUCH_INT_DONE, 0);
    }

    #[test]
    fn test_touch_interrupt_enable() {
        let mut touch = Touch::new();
        touch.write(RTC_CNTL_INT_ENA_REG, 4, TOUCH_INT_DONE | TOUCH_INT_ACTIVE);
        assert_eq!(touch.int_ena, TOUCH_INT_DONE | TOUCH_INT_ACTIVE);
    }

    #[test]
    fn test_touch_interrupt_clear() {
        let mut touch = Touch::new();
        touch.int_raw = TOUCH_INT_DONE | TOUCH_INT_ACTIVE;

        // Clear DONE interrupt
        touch.write(RTC_CNTL_INT_RAW_REG, 4, TOUCH_INT_DONE);

        assert_eq!(touch.int_raw, TOUCH_INT_ACTIVE);
    }

    #[test]
    fn test_touch_value_read() {
        let mut touch = Touch::new();
        touch.simulate_touch(0, 750);

        let val = touch.read(SENS_SAR_TOUCH_OUT_BASE, 4);
        assert_eq!(val, 750);
    }

    #[test]
    fn test_touch_threshold_read_write() {
        let mut touch = Touch::new();

        // Write threshold via MMIO
        touch.write(SENS_SAR_TOUCH_THRES_BASE, 4, 900);

        // Read it back
        let val = touch.read(SENS_SAR_TOUCH_THRES_BASE, 4);
        assert_eq!(val, 900);
    }

    #[test]
    fn test_touch_filter_enable() {
        let mut touch = Touch::new();
        touch.write(SENS_SAR_TOUCH_CONF_REG, 4, TOUCH_FILTER_EN);
        assert_ne!(touch.config & TOUCH_FILTER_EN, 0);
    }

    #[test]
    fn test_touch_all_channels() {
        let mut touch = Touch::new();

        // Enable all 10 channels
        for i in 0..10 {
            touch.enable_channel(i);
        }

        assert_eq!(touch.enable_mask, 0x3FF); // 10 bits set
    }
}
