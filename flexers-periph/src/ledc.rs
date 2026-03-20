use flexers_core::memory::MmioHandler;

/// LEDC (LED PWM Controller) register offsets
/// High-speed channels (0-7)
const LEDC_HSCH0_CONF0: u32 = 0x000;
const LEDC_CH_CONF0_STEP: u32 = 0x14;  // 20 bytes between channel configs

/// Low-speed channels (8-15) start at different offset
const LEDC_LSCH0_CONF0: u32 = 0x100;

/// Channel configuration register offsets (per channel)
const LEDC_CHn_CONF0_OFFSET: u32 = 0x00;
const LEDC_CHn_HPOINT_OFFSET: u32 = 0x04;
const LEDC_CHn_DUTY_OFFSET: u32 = 0x08;
const LEDC_CHn_CONF1_OFFSET: u32 = 0x0C;
const LEDC_CHn_DUTY_R_OFFSET: u32 = 0x10;

/// Timer configuration registers (4 high-speed + 4 low-speed timers)
const LEDC_HSTIMER0_CONF: u32 = 0x0A0;
const LEDC_LSTIMER0_CONF: u32 = 0x1A0;
const LEDC_TIMER_CONF_STEP: u32 = 0x08;

/// Configuration bits for channel CONF0
const LEDC_TIMER_SEL_MASK: u32 = 0x3;         // Timer select (bits 0-1)
const LEDC_SIG_OUT_EN: u32 = 1 << 2;          // Signal output enable
const LEDC_IDLE_LV: u32 = 1 << 3;             // Idle level
const LEDC_PARA_UP: u32 = 1 << 4;             // Parameter update

/// Configuration bits for channel CONF1
const LEDC_DUTY_START: u32 = 1 << 31;         // Start duty cycle update

/// Timer configuration bits
const LEDC_TIMER_DUTY_RES_MASK: u32 = 0x1F;   // Duty resolution (bits 0-4)
const LEDC_TIMER_CLK_DIV_MASK: u32 = 0x3FFFF; // Clock divider (bits 5-22)
const LEDC_TIMER_PAUSE: u32 = 1 << 23;        // Pause timer
const LEDC_TIMER_RST: u32 = 1 << 24;          // Reset timer
const LEDC_TICK_SEL: u32 = 1 << 25;           // Tick select

/// Maximum duty resolution (13 bits)
const MAX_DUTY_BITS: u8 = 13;
const MAX_DUTY: u32 = (1 << MAX_DUTY_BITS) - 1; // 8191

/// LED Controller Channel
#[derive(Debug, Clone)]
struct LedcChannel {
    /// Duty cycle value (0-8191 for 13-bit)
    duty: u32,

    /// High point (when to start high level in PWM cycle)
    hpoint: u32,

    /// Timer selection (0-3)
    timer_sel: u8,

    /// Signal output enable
    output_enable: bool,

    /// Idle level (0 or 1)
    idle_level: bool,

    /// GPIO pin number (for tracking)
    gpio: u8,

    /// Configuration register 0
    conf0: u32,

    /// Configuration register 1
    conf1: u32,
}

impl LedcChannel {
    fn new() -> Self {
        Self {
            duty: 0,
            hpoint: 0,
            timer_sel: 0,
            output_enable: false,
            idle_level: false,
            gpio: 0,
            conf0: 0,
            conf1: 0,
        }
    }

    fn reset(&mut self) {
        self.duty = 0;
        self.hpoint = 0;
        self.timer_sel = 0;
        self.output_enable = false;
        self.idle_level = false;
        self.conf0 = 0;
        self.conf1 = 0;
    }
}

/// LED Controller Timer
#[derive(Debug, Clone)]
struct LedcTimer {
    /// Frequency in Hz (derived from clock divider)
    freq: u32,

    /// Duty resolution in bits (1-13)
    duty_res: u8,

    /// Clock divider
    clk_div: u32,

    /// Timer paused
    paused: bool,

    /// Configuration register
    conf: u32,

    /// Current counter value
    counter: u32,
}

impl LedcTimer {
    fn new() -> Self {
        Self {
            freq: 5000,  // Default 5kHz
            duty_res: 13,  // Default 13-bit
            clk_div: 256,
            paused: false,
            conf: 0,
            counter: 0,
        }
    }

    fn reset(&mut self) {
        self.freq = 5000;
        self.duty_res = 13;
        self.clk_div = 256;
        self.paused = false;
        self.conf = 0;
        self.counter = 0;
    }

    /// Get maximum counter value based on duty resolution
    fn max_count(&self) -> u32 {
        (1 << self.duty_res) - 1
    }

    /// Tick timer (increment counter)
    fn tick(&mut self) {
        if !self.paused {
            self.counter += 1;
            if self.counter >= self.max_count() {
                self.counter = 0;
            }
        }
    }
}

/// LED PWM Controller (LEDC)
/// Provides 16 PWM channels and 8 timers
pub struct Ledc {
    /// 16 PWM channels (8 high-speed + 8 low-speed)
    channels: [LedcChannel; 16],

    /// 8 timers (4 high-speed + 4 low-speed)
    timers: [LedcTimer; 8],
}

impl Ledc {
    /// Create new LEDC peripheral
    pub fn new() -> Self {
        Self {
            channels: [
                LedcChannel::new(), LedcChannel::new(), LedcChannel::new(), LedcChannel::new(),
                LedcChannel::new(), LedcChannel::new(), LedcChannel::new(), LedcChannel::new(),
                LedcChannel::new(), LedcChannel::new(), LedcChannel::new(), LedcChannel::new(),
                LedcChannel::new(), LedcChannel::new(), LedcChannel::new(), LedcChannel::new(),
            ],
            timers: [
                LedcTimer::new(), LedcTimer::new(), LedcTimer::new(), LedcTimer::new(),
                LedcTimer::new(), LedcTimer::new(), LedcTimer::new(), LedcTimer::new(),
            ],
        }
    }

    /// Get duty cycle for a channel (for testing/monitoring)
    pub fn get_duty(&self, channel: u8) -> Option<u32> {
        if (channel as usize) < self.channels.len() {
            Some(self.channels[channel as usize].duty)
        } else {
            None
        }
    }

    /// Get timer frequency (for testing/monitoring)
    pub fn get_timer_freq(&self, timer: u8) -> Option<u32> {
        if (timer as usize) < self.timers.len() {
            Some(self.timers[timer as usize].freq)
        } else {
            None
        }
    }

    /// Get channel output state (high or low) based on current timer counter
    pub fn get_channel_output(&self, channel: u8) -> Option<bool> {
        if (channel as usize) >= self.channels.len() {
            return None;
        }

        let ch = &self.channels[channel as usize];
        if !ch.output_enable {
            return Some(ch.idle_level);
        }

        let timer = &self.timers[ch.timer_sel as usize];
        let counter = timer.counter;
        let max_count = timer.max_count();

        // Simple PWM logic: output high when counter < duty
        // Scale duty to timer's resolution
        let scaled_duty = if max_count > 0 {
            (ch.duty * max_count) / MAX_DUTY
        } else {
            0
        };

        Some(counter < scaled_duty)
    }

    /// Tick all timers (called periodically)
    pub fn tick(&mut self) {
        for timer in &mut self.timers {
            timer.tick();
        }
    }

    /// Map channel to GPIO pin
    pub fn set_channel_gpio(&mut self, channel: u8, gpio: u8) {
        if (channel as usize) < self.channels.len() {
            self.channels[channel as usize].gpio = gpio;
        }
    }

    /// Get channel register base offset
    fn channel_reg_base(&self, channel: u8) -> u32 {
        if channel < 8 {
            // High-speed channel
            LEDC_HSCH0_CONF0 + (channel as u32) * LEDC_CH_CONF0_STEP
        } else {
            // Low-speed channel
            LEDC_LSCH0_CONF0 + ((channel - 8) as u32) * LEDC_CH_CONF0_STEP
        }
    }

    /// Get timer register base offset
    fn timer_reg_base(&self, timer: u8) -> u32 {
        if timer < 4 {
            // High-speed timer
            LEDC_HSTIMER0_CONF + (timer as u32) * LEDC_TIMER_CONF_STEP
        } else {
            // Low-speed timer
            LEDC_LSTIMER0_CONF + ((timer - 4) as u32) * LEDC_TIMER_CONF_STEP
        }
    }
}

impl Default for Ledc {
    fn default() -> Self {
        Self::new()
    }
}

impl MmioHandler for Ledc {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        let offset = addr & 0xFFF;

        // Check if it's a high-speed channel register (0x000-0x09C)
        if offset >= LEDC_HSCH0_CONF0 && offset < LEDC_HSTIMER0_CONF {
            let channel = ((offset - LEDC_HSCH0_CONF0) / LEDC_CH_CONF0_STEP) as usize;
            if channel >= 8 {
                return 0;
            }

            let ch_offset = (offset - LEDC_HSCH0_CONF0) % LEDC_CH_CONF0_STEP;
            let ch = &self.channels[channel];

            match ch_offset {
                LEDC_CHn_CONF0_OFFSET => ch.conf0,
                LEDC_CHn_HPOINT_OFFSET => ch.hpoint,
                LEDC_CHn_DUTY_OFFSET => ch.duty,
                LEDC_CHn_CONF1_OFFSET => ch.conf1,
                LEDC_CHn_DUTY_R_OFFSET => ch.duty, // Read-only copy of duty
                _ => 0,
            }
        }
        // Check if it's a low-speed channel register (0x100-0x19C)
        else if offset >= LEDC_LSCH0_CONF0 && offset < LEDC_LSTIMER0_CONF {
            let channel = 8 + ((offset - LEDC_LSCH0_CONF0) / LEDC_CH_CONF0_STEP) as usize;
            if channel >= self.channels.len() {
                return 0;
            }

            let ch_offset = (offset - LEDC_LSCH0_CONF0) % LEDC_CH_CONF0_STEP;
            let ch = &self.channels[channel];

            match ch_offset {
                LEDC_CHn_CONF0_OFFSET => ch.conf0,
                LEDC_CHn_HPOINT_OFFSET => ch.hpoint,
                LEDC_CHn_DUTY_OFFSET => ch.duty,
                LEDC_CHn_CONF1_OFFSET => ch.conf1,
                LEDC_CHn_DUTY_R_OFFSET => ch.duty, // Read-only copy of duty
                _ => 0,
            }
        }
        // Check if it's a high-speed timer register (0x0A0-0x0B8)
        else if offset >= LEDC_HSTIMER0_CONF && offset < LEDC_HSTIMER0_CONF + (4 * LEDC_TIMER_CONF_STEP) {
            let timer = ((offset - LEDC_HSTIMER0_CONF) / LEDC_TIMER_CONF_STEP) as usize;
            if timer >= 4 {
                return 0;
            }
            self.timers[timer].conf
        }
        // Check if it's a low-speed timer register (0x1A0-0x1B8)
        else if offset >= LEDC_LSTIMER0_CONF && offset < LEDC_LSTIMER0_CONF + (4 * LEDC_TIMER_CONF_STEP) {
            let timer = 4 + ((offset - LEDC_LSTIMER0_CONF) / LEDC_TIMER_CONF_STEP) as usize;
            if timer >= self.timers.len() {
                return 0;
            }
            self.timers[timer].conf
        } else {
            0
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        let offset = addr & 0xFFF;

        // Check if it's a high-speed channel register
        if offset >= LEDC_HSCH0_CONF0 && offset < LEDC_HSTIMER0_CONF {
            let channel = ((offset - LEDC_HSCH0_CONF0) / LEDC_CH_CONF0_STEP) as usize;
            if channel >= 8 {
                return;
            }

            let ch_offset = (offset - LEDC_HSCH0_CONF0) % LEDC_CH_CONF0_STEP;
            let ch = &mut self.channels[channel];

            match ch_offset {
                LEDC_CHn_CONF0_OFFSET => {
                    ch.conf0 = val;
                    ch.timer_sel = (val & LEDC_TIMER_SEL_MASK) as u8;
                    ch.output_enable = (val & LEDC_SIG_OUT_EN) != 0;
                    ch.idle_level = (val & LEDC_IDLE_LV) != 0;
                }
                LEDC_CHn_HPOINT_OFFSET => {
                    ch.hpoint = val & MAX_DUTY;
                }
                LEDC_CHn_DUTY_OFFSET => {
                    ch.duty = val & MAX_DUTY;
                }
                LEDC_CHn_CONF1_OFFSET => {
                    ch.conf1 = val;
                    if (val & LEDC_DUTY_START) != 0 {
                        ch.conf1 &= !LEDC_DUTY_START;
                    }
                }
                LEDC_CHn_DUTY_R_OFFSET => {}
                _ => {}
            }
        }
        // Check if it's a low-speed channel register
        else if offset >= LEDC_LSCH0_CONF0 && offset < LEDC_LSTIMER0_CONF {
            let channel = 8 + ((offset - LEDC_LSCH0_CONF0) / LEDC_CH_CONF0_STEP) as usize;
            if channel >= self.channels.len() {
                return;
            }

            let ch_offset = (offset - LEDC_LSCH0_CONF0) % LEDC_CH_CONF0_STEP;
            let ch = &mut self.channels[channel];

            match ch_offset {
                LEDC_CHn_CONF0_OFFSET => {
                    ch.conf0 = val;
                    ch.timer_sel = (val & LEDC_TIMER_SEL_MASK) as u8;
                    ch.output_enable = (val & LEDC_SIG_OUT_EN) != 0;
                    ch.idle_level = (val & LEDC_IDLE_LV) != 0;
                }
                LEDC_CHn_HPOINT_OFFSET => {
                    ch.hpoint = val & MAX_DUTY;
                }
                LEDC_CHn_DUTY_OFFSET => {
                    ch.duty = val & MAX_DUTY;
                }
                LEDC_CHn_CONF1_OFFSET => {
                    ch.conf1 = val;
                    if (val & LEDC_DUTY_START) != 0 {
                        ch.conf1 &= !LEDC_DUTY_START;
                    }
                }
                LEDC_CHn_DUTY_R_OFFSET => {}
                _ => {}
            }
        }
        // Check if it's a high-speed timer register
        else if offset >= LEDC_HSTIMER0_CONF && offset < LEDC_HSTIMER0_CONF + (4 * LEDC_TIMER_CONF_STEP) {
            let timer = ((offset - LEDC_HSTIMER0_CONF) / LEDC_TIMER_CONF_STEP) as usize;
            if timer >= 4 {
                return;
            }

            let tm = &mut self.timers[timer];
            tm.conf = val;
            tm.duty_res = ((val & LEDC_TIMER_DUTY_RES_MASK) as u8).min(MAX_DUTY_BITS);
            tm.clk_div = ((val >> 5) & LEDC_TIMER_CLK_DIV_MASK).max(1);

            const APB_CLK_FREQ: u32 = 80_000_000;
            let divisor = tm.clk_div * (1 << tm.duty_res);
            tm.freq = if divisor > 0 { APB_CLK_FREQ / divisor } else { 5000 };

            tm.paused = (val & LEDC_TIMER_PAUSE) != 0;

            if (val & LEDC_TIMER_RST) != 0 {
                tm.counter = 0;
                tm.conf &= !LEDC_TIMER_RST;
            }
        }
        // Check if it's a low-speed timer register
        else if offset >= LEDC_LSTIMER0_CONF && offset < LEDC_LSTIMER0_CONF + (4 * LEDC_TIMER_CONF_STEP) {
            let timer = 4 + ((offset - LEDC_LSTIMER0_CONF) / LEDC_TIMER_CONF_STEP) as usize;
            if timer >= self.timers.len() {
                return;
            }

            let tm = &mut self.timers[timer];
            tm.conf = val;
            tm.duty_res = ((val & LEDC_TIMER_DUTY_RES_MASK) as u8).min(MAX_DUTY_BITS);
            tm.clk_div = ((val >> 5) & LEDC_TIMER_CLK_DIV_MASK).max(1);

            const APB_CLK_FREQ: u32 = 80_000_000;
            let divisor = tm.clk_div * (1 << tm.duty_res);
            tm.freq = if divisor > 0 { APB_CLK_FREQ / divisor } else { 5000 };

            tm.paused = (val & LEDC_TIMER_PAUSE) != 0;

            if (val & LEDC_TIMER_RST) != 0 {
                tm.counter = 0;
                tm.conf &= !LEDC_TIMER_RST;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ledc_creation() {
        let ledc = Ledc::new();
        assert_eq!(ledc.channels.len(), 16);
        assert_eq!(ledc.timers.len(), 8);
    }

    #[test]
    fn test_channel_duty_config() {
        let mut ledc = Ledc::new();

        // Write to channel 0 duty register (high-speed)
        let reg = LEDC_HSCH0_CONF0 + LEDC_CHn_DUTY_OFFSET;
        ledc.write(reg, 4, 4095);

        assert_eq!(ledc.get_duty(0), Some(4095));
        assert_eq!(ledc.read(reg, 4), 4095);
    }

    #[test]
    fn test_multiple_channels() {
        let mut ledc = Ledc::new();

        // Configure different duty cycles for different channels
        for ch in 0..16 {
            let base = if ch < 8 { LEDC_HSCH0_CONF0 } else { LEDC_LSCH0_CONF0 };
            let ch_offset = if ch < 8 { ch } else { ch - 8 };
            let reg = base + (ch_offset * LEDC_CH_CONF0_STEP) + LEDC_CHn_DUTY_OFFSET;
            ledc.write(reg, 4, (ch + 1) * 100);
        }

        // Verify each channel
        for ch in 0..16 {
            assert_eq!(ledc.get_duty(ch as u8), Some((ch + 1) * 100));
        }
    }

    #[test]
    fn test_timer_config() {
        let mut ledc = Ledc::new();

        // Configure timer 0: 13-bit resolution, divider = 100
        let val = 13 | (100 << 5);
        ledc.write(LEDC_HSTIMER0_CONF, 4, val);

        assert_eq!(ledc.timers[0].duty_res, 13);
        assert_eq!(ledc.timers[0].clk_div, 100);

        // Frequency should be calculated
        assert!(ledc.get_timer_freq(0).unwrap() > 0);
    }

    #[test]
    fn test_channel_timer_selection() {
        let mut ledc = Ledc::new();

        // Select timer 2 for channel 0
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_CONF0_OFFSET, 4, 2);

        assert_eq!(ledc.channels[0].timer_sel, 2);

        // Select timer 3 for channel 1
        let reg = LEDC_HSCH0_CONF0 + LEDC_CH_CONF0_STEP + LEDC_CHn_CONF0_OFFSET;
        ledc.write(reg, 4, 3);

        assert_eq!(ledc.channels[1].timer_sel, 3);
    }

    #[test]
    fn test_output_enable() {
        let mut ledc = Ledc::new();

        // Enable output for channel 0
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_CONF0_OFFSET, 4, LEDC_SIG_OUT_EN);

        assert!(ledc.channels[0].output_enable);

        // Disable output
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_CONF0_OFFSET, 4, 0);

        assert!(!ledc.channels[0].output_enable);
    }

    #[test]
    fn test_timer_pause() {
        let mut ledc = Ledc::new();

        // Configure and pause timer 0
        let val = 13 | (100 << 5) | LEDC_TIMER_PAUSE;
        ledc.write(LEDC_HSTIMER0_CONF, 4, val);

        assert!(ledc.timers[0].paused);

        // Tick should not increment counter
        let counter_before = ledc.timers[0].counter;
        ledc.tick();
        assert_eq!(ledc.timers[0].counter, counter_before);
    }

    #[test]
    fn test_timer_reset() {
        let mut ledc = Ledc::new();

        // Set counter to non-zero
        ledc.timers[0].counter = 1000;

        // Reset timer
        let val = LEDC_TIMER_RST;
        ledc.write(LEDC_HSTIMER0_CONF, 4, val);

        assert_eq!(ledc.timers[0].counter, 0);
    }

    #[test]
    fn test_pwm_output_simulation() {
        let mut ledc = Ledc::new();

        // Configure timer 0: 10-bit resolution
        ledc.write(LEDC_HSTIMER0_CONF, 4, 10 | (100 << 5));

        // Configure channel 0: 50% duty (512 out of 1024)
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_CONF0_OFFSET, 4, LEDC_SIG_OUT_EN);
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_DUTY_OFFSET, 4, 4096); // 50% of 8191

        // At counter=0, output should be high
        ledc.timers[0].counter = 0;
        assert_eq!(ledc.get_channel_output(0), Some(true));

        // At counter=max, output should be low
        ledc.timers[0].counter = 1023;
        assert_eq!(ledc.get_channel_output(0), Some(false));
    }

    #[test]
    fn test_idle_level() {
        let mut ledc = Ledc::new();

        // Disable output, set idle level high
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_CONF0_OFFSET, 4, LEDC_IDLE_LV);

        assert_eq!(ledc.get_channel_output(0), Some(true));

        // Disable output, set idle level low
        ledc.write(LEDC_HSCH0_CONF0 + LEDC_CHn_CONF0_OFFSET, 4, 0);

        assert_eq!(ledc.get_channel_output(0), Some(false));
    }

    #[test]
    fn test_duty_limit() {
        let mut ledc = Ledc::new();

        // Write value larger than max duty
        let reg = LEDC_HSCH0_CONF0 + LEDC_CHn_DUTY_OFFSET;
        ledc.write(reg, 4, 0xFFFF);

        // Should be limited to MAX_DUTY (8191)
        assert_eq!(ledc.get_duty(0), Some(MAX_DUTY));
    }

    #[test]
    fn test_gpio_mapping() {
        let mut ledc = Ledc::new();

        ledc.set_channel_gpio(0, 25);
        ledc.set_channel_gpio(1, 26);

        assert_eq!(ledc.channels[0].gpio, 25);
        assert_eq!(ledc.channels[1].gpio, 26);
    }

    #[test]
    fn test_timer_tick() {
        let mut ledc = Ledc::new();

        // Configure timer 0: 8-bit resolution (max count = 255)
        ledc.write(LEDC_HSTIMER0_CONF, 4, 8 | (100 << 5));

        let initial_counter = ledc.timers[0].counter;
        ledc.tick();

        assert_eq!(ledc.timers[0].counter, initial_counter + 1);

        // Tick until overflow
        for _ in 0..300 {
            ledc.tick();
        }

        // Counter should have wrapped around
        assert!(ledc.timers[0].counter <= 255);
    }
}
