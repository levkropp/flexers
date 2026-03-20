use flexers_core::memory::MmioHandler;
use std::collections::VecDeque;

/// I2C register offsets
const I2C_SCL_LOW_PERIOD_REG: u32 = 0x000;    // SCL low period
const I2C_CTR_REG: u32 = 0x004;               // Control register
const I2C_SR_REG: u32 = 0x008;                // Status register
const I2C_TO_REG: u32 = 0x00C;                // Timeout register
const I2C_SLAVE_ADDR_REG: u32 = 0x010;        // Slave address
const I2C_RXFIFO_ST_REG: u32 = 0x014;         // RX FIFO status
const I2C_FIFO_CONF_REG: u32 = 0x018;         // FIFO configuration
const I2C_DATA_REG: u32 = 0x01C;              // Data FIFO register
const I2C_INT_RAW_REG: u32 = 0x020;           // Interrupt raw status
const I2C_INT_CLR_REG: u32 = 0x024;           // Interrupt clear
const I2C_INT_ENA_REG: u32 = 0x028;           // Interrupt enable
const I2C_INT_STATUS_REG: u32 = 0x02C;        // Interrupt status
const I2C_SDA_HOLD_REG: u32 = 0x030;          // SDA hold time
const I2C_SDA_SAMPLE_REG: u32 = 0x034;        // SDA sample time
const I2C_SCL_HIGH_PERIOD_REG: u32 = 0x038;   // SCL high period
const I2C_SCL_START_HOLD_REG: u32 = 0x040;    // SCL start hold time
const I2C_SCL_RSTART_SETUP_REG: u32 = 0x044;  // SCL restart setup time
const I2C_SCL_STOP_HOLD_REG: u32 = 0x048;     // SCL stop hold time
const I2C_SCL_STOP_SETUP_REG: u32 = 0x04C;    // SCL stop setup time
const I2C_COMD0_REG: u32 = 0x058;             // Command 0 register
const I2C_COMD_STEP: u32 = 0x004;             // Step between command registers

/// Control register bits
const I2C_CTR_TRANS_START: u32 = 1 << 5;      // Start transmission
const I2C_CTR_MS_MODE: u32 = 1 << 4;          // Master/slave mode (1=master)
const I2C_CTR_RX_LSB_FIRST: u32 = 1 << 1;     // RX LSB first
const I2C_CTR_TX_LSB_FIRST: u32 = 1 << 0;     // TX LSB first

/// Status register bits
const I2C_SR_BUSY: u32 = 1 << 4;              // Bus busy
const I2C_SR_ACK_REC: u32 = 1 << 0;           // ACK received
const I2C_SR_SLAVE_RW: u32 = 1 << 1;          // Slave read/write
const I2C_SR_TIME_OUT: u32 = 1 << 2;          // Timeout
const I2C_SR_ARB_LOST: u32 = 1 << 3;          // Arbitration lost

/// Command register bits
const I2C_CMD_OPCODE_MASK: u32 = 0x7;         // Operation code (bits 0-2)
const I2C_CMD_BYTE_NUM_MASK: u32 = 0xFF;      // Byte count (bits 8-15)
const I2C_CMD_ACK_VAL: u32 = 1 << 16;         // ACK value
const I2C_CMD_ACK_EXP: u32 = 1 << 17;         // ACK expected
const I2C_CMD_ACK_CHECK_EN: u32 = 1 << 18;    // ACK check enable
const I2C_CMD_DONE: u32 = 1 << 31;            // Command done

/// Command opcodes
const I2C_CMD_RSTART: u32 = 0;                // Restart
const I2C_CMD_WRITE: u32 = 1;                 // Write
const I2C_CMD_READ: u32 = 2;                  // Read
const I2C_CMD_STOP: u32 = 3;                  // Stop
const I2C_CMD_END: u32 = 4;                   // End

/// FIFO configuration bits
const I2C_FIFO_CONF_RXFIFO_RST: u32 = 1 << 0; // RX FIFO reset
const I2C_FIFO_CONF_TXFIFO_RST: u32 = 1 << 1; // TX FIFO reset

/// Interrupt bits
const I2C_INT_TRANS_COMPLETE: u32 = 1 << 7;   // Transaction complete
const I2C_INT_MASTER_TRAN_COMP: u32 = 1 << 6; // Master transaction complete
const I2C_INT_ACK_ERR: u32 = 1 << 10;         // ACK error
const I2C_INT_TIMEOUT: u32 = 1 << 8;          // Timeout

/// FIFO size
const FIFO_SIZE: usize = 32;

/// I2C clock speeds
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum I2cSpeed {
    Standard,    // 100 kHz
    Fast,        // 400 kHz
    FastPlus,    // 1 MHz
}

/// I2C peripheral state
#[derive(Debug, Clone, Copy, PartialEq)]
enum I2cState {
    Idle,
    Active,
    Error,
}

/// Inter-Integrated Circuit (I2C) Controller
/// ESP32 has 2 I2C controllers, this implements one controller
pub struct I2c {
    /// SCL pin
    scl_pin: u8,

    /// SDA pin
    sda_pin: u8,

    /// Clock speed
    speed: I2cSpeed,

    /// 7-bit slave address
    address: u8,

    /// Transmit FIFO
    tx_fifo: VecDeque<u8>,

    /// Receive FIFO
    rx_fifo: VecDeque<u8>,

    /// Control register
    control: u32,

    /// Status register
    status: u32,

    /// Configuration registers
    scl_low_period: u32,
    scl_high_period: u32,
    sda_hold: u32,
    sda_sample: u32,

    /// Command registers (16 commands)
    commands: [u32; 16],

    /// Current state
    state: I2cState,

    /// Master mode
    master_mode: bool,

    /// Interrupt registers
    int_raw: u32,
    int_ena: u32,

    /// Timeout value
    timeout: u32,

    /// FIFO configuration
    fifo_conf: u32,
}

impl I2c {
    /// Create new I2C peripheral
    pub fn new() -> Self {
        Self {
            scl_pin: 0,
            sda_pin: 0,
            speed: I2cSpeed::Standard,
            address: 0,
            tx_fifo: VecDeque::with_capacity(FIFO_SIZE),
            rx_fifo: VecDeque::with_capacity(FIFO_SIZE),
            control: 0,
            status: 0,
            scl_low_period: 0,
            scl_high_period: 0,
            sda_hold: 0,
            sda_sample: 0,
            commands: [0; 16],
            state: I2cState::Idle,
            master_mode: true,
            int_raw: 0,
            int_ena: 0,
            timeout: 0xFFFFF,  // Default timeout
            fifo_conf: 0,
        }
    }

    /// Configure I2C pins
    pub fn set_pins(&mut self, scl: u8, sda: u8) {
        self.scl_pin = scl;
        self.sda_pin = sda;
    }

    /// Set clock speed
    pub fn set_speed(&mut self, speed: I2cSpeed) {
        self.speed = speed;

        // Configure timing registers based on speed
        match speed {
            I2cSpeed::Standard => {
                self.scl_low_period = 500;  // ~100 kHz
                self.scl_high_period = 500;
            }
            I2cSpeed::Fast => {
                self.scl_low_period = 125;  // ~400 kHz
                self.scl_high_period = 125;
            }
            I2cSpeed::FastPlus => {
                self.scl_low_period = 50;   // ~1 MHz
                self.scl_high_period = 50;
            }
        }
    }

    /// Start I2C transaction
    pub fn start_transaction(&mut self, addr: u8, read: bool) {
        self.address = addr & 0x7F;  // 7-bit address
        self.state = I2cState::Active;
        self.status |= I2C_SR_BUSY;

        // Write address + R/W bit to TX FIFO
        let addr_byte = (self.address << 1) | if read { 1 } else { 0 };
        if self.tx_fifo.len() < FIFO_SIZE {
            self.tx_fifo.push_back(addr_byte);
        }
    }

    /// Write byte to TX FIFO
    pub fn write_byte(&mut self, byte: u8) -> bool {
        if self.tx_fifo.len() < FIFO_SIZE {
            self.tx_fifo.push_back(byte);
            true
        } else {
            false
        }
    }

    /// Read byte from RX FIFO
    pub fn read_byte(&mut self) -> Option<u8> {
        self.rx_fifo.pop_front()
    }

    /// Get TX FIFO count
    pub fn tx_fifo_count(&self) -> usize {
        self.tx_fifo.len()
    }

    /// Get RX FIFO count
    pub fn rx_fifo_count(&self) -> usize {
        self.rx_fifo.len()
    }

    /// Execute command queue
    fn execute_commands(&mut self) {
        for i in 0..16 {
            let cmd = self.commands[i];
            if cmd == 0 {
                continue;
            }

            let opcode = cmd & I2C_CMD_OPCODE_MASK;

            match opcode {
                I2C_CMD_RSTART => {
                    // Restart condition
                    self.state = I2cState::Active;
                    self.status |= I2C_SR_BUSY;
                }
                I2C_CMD_WRITE => {
                    // Write bytes from TX FIFO
                    let byte_num = ((cmd >> 8) & I2C_CMD_BYTE_NUM_MASK) as usize;

                    for _ in 0..byte_num {
                        if self.tx_fifo.is_empty() {
                            break;
                        }
                        let _byte = self.tx_fifo.pop_front();

                        // In real hardware, byte would be transmitted
                        // In emulation, we simulate ACK
                        self.status |= I2C_SR_ACK_REC;
                    }

                    // Check if ACK expected
                    if (cmd & I2C_CMD_ACK_CHECK_EN) != 0 {
                        let ack_exp = (cmd & I2C_CMD_ACK_EXP) != 0;
                        let ack_rec = (self.status & I2C_SR_ACK_REC) != 0;

                        if ack_exp != ack_rec {
                            // ACK error
                            self.int_raw |= I2C_INT_ACK_ERR;
                            self.state = I2cState::Error;
                        }
                    }
                }
                I2C_CMD_READ => {
                    // Read bytes into RX FIFO
                    let byte_num = ((cmd >> 8) & I2C_CMD_BYTE_NUM_MASK) as usize;

                    for _ in 0..byte_num {
                        if self.rx_fifo.len() >= FIFO_SIZE {
                            break;
                        }

                        // In real hardware, bytes would be read from slave
                        // In emulation, we return dummy data
                        self.rx_fifo.push_back(0x00);
                    }
                }
                I2C_CMD_STOP => {
                    // Stop condition
                    self.state = I2cState::Idle;
                    self.status &= !I2C_SR_BUSY;
                    self.int_raw |= I2C_INT_TRANS_COMPLETE | I2C_INT_MASTER_TRAN_COMP;
                }
                I2C_CMD_END => {
                    // End of command queue
                    break;
                }
                _ => {}
            }

            // Mark command as done
            self.commands[i] |= I2C_CMD_DONE;
        }
    }

    /// Reset FIFOs
    fn reset_fifos(&mut self, rx: bool, tx: bool) {
        if rx {
            self.rx_fifo.clear();
        }
        if tx {
            self.tx_fifo.clear();
        }
    }

    /// Simulate putting data in RX FIFO (for testing)
    pub fn simulate_rx_data(&mut self, data: &[u8]) {
        for &byte in data {
            if self.rx_fifo.len() < FIFO_SIZE {
                self.rx_fifo.push_back(byte);
            }
        }
    }
}

impl Default for I2c {
    fn default() -> Self {
        Self::new()
    }
}

impl MmioHandler for I2c {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFFF {
            I2C_SCL_LOW_PERIOD_REG => self.scl_low_period,
            I2C_CTR_REG => self.control,
            I2C_SR_REG => self.status,
            I2C_TO_REG => self.timeout,
            I2C_SLAVE_ADDR_REG => self.address as u32,
            I2C_RXFIFO_ST_REG => {
                // RX FIFO count in lower bits
                self.rx_fifo.len() as u32
            }
            I2C_FIFO_CONF_REG => self.fifo_conf,
            I2C_DATA_REG => {
                // Can't modify self in read, so return 0
                // In real implementation, this would need RefCell or similar
                0
            }
            I2C_INT_RAW_REG => self.int_raw,
            I2C_INT_ENA_REG => self.int_ena,
            I2C_INT_STATUS_REG => self.int_raw & self.int_ena,
            I2C_SDA_HOLD_REG => self.sda_hold,
            I2C_SDA_SAMPLE_REG => self.sda_sample,
            I2C_SCL_HIGH_PERIOD_REG => self.scl_high_period,
            I2C_SCL_START_HOLD_REG => 10,  // Default values
            I2C_SCL_RSTART_SETUP_REG => 10,
            I2C_SCL_STOP_HOLD_REG => 10,
            I2C_SCL_STOP_SETUP_REG => 10,
            offset if offset >= I2C_COMD0_REG && offset < I2C_COMD0_REG + (16 * I2C_COMD_STEP) => {
                let cmd_idx = ((offset - I2C_COMD0_REG) / I2C_COMD_STEP) as usize;
                self.commands[cmd_idx]
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFFF {
            I2C_SCL_LOW_PERIOD_REG => self.scl_low_period = val,
            I2C_CTR_REG => {
                self.control = val;
                self.master_mode = (val & I2C_CTR_MS_MODE) != 0;

                // Check for transaction start
                if (val & I2C_CTR_TRANS_START) != 0 {
                    self.execute_commands();
                    // Clear trans_start bit
                    self.control &= !I2C_CTR_TRANS_START;
                }
            }
            I2C_SR_REG => {
                // Writing to status can clear certain flags
                self.status &= !val;
            }
            I2C_TO_REG => self.timeout = val,
            I2C_SLAVE_ADDR_REG => self.address = (val & 0x7F) as u8,
            I2C_FIFO_CONF_REG => {
                self.fifo_conf = val;

                // Check FIFO reset bits
                let rx_reset = (val & I2C_FIFO_CONF_RXFIFO_RST) != 0;
                let tx_reset = (val & I2C_FIFO_CONF_TXFIFO_RST) != 0;

                if rx_reset || tx_reset {
                    self.reset_fifos(rx_reset, tx_reset);
                    // Clear reset bits
                    self.fifo_conf &= !(I2C_FIFO_CONF_RXFIFO_RST | I2C_FIFO_CONF_TXFIFO_RST);
                }
            }
            I2C_DATA_REG => {
                // Write to TX FIFO
                self.write_byte(val as u8);
            }
            I2C_INT_RAW_REG => {
                // Writing doesn't change raw interrupt
            }
            I2C_INT_CLR_REG => {
                // Clear interrupts
                self.int_raw &= !val;
            }
            I2C_INT_ENA_REG => self.int_ena = val,
            I2C_SDA_HOLD_REG => self.sda_hold = val,
            I2C_SDA_SAMPLE_REG => self.sda_sample = val,
            I2C_SCL_HIGH_PERIOD_REG => self.scl_high_period = val,
            I2C_SCL_START_HOLD_REG | I2C_SCL_RSTART_SETUP_REG |
            I2C_SCL_STOP_HOLD_REG | I2C_SCL_STOP_SETUP_REG => {
                // Timing registers - accept but don't need to store for emulation
            }
            offset if offset >= I2C_COMD0_REG && offset < I2C_COMD0_REG + (16 * I2C_COMD_STEP) => {
                let cmd_idx = ((offset - I2C_COMD0_REG) / I2C_COMD_STEP) as usize;
                self.commands[cmd_idx] = val;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i2c_creation() {
        let i2c = I2c::new();
        assert_eq!(i2c.state, I2cState::Idle);
        assert_eq!(i2c.tx_fifo.len(), 0);
        assert_eq!(i2c.rx_fifo.len(), 0);
    }

    #[test]
    fn test_pin_configuration() {
        let mut i2c = I2c::new();
        i2c.set_pins(22, 21);

        assert_eq!(i2c.scl_pin, 22);
        assert_eq!(i2c.sda_pin, 21);
    }

    #[test]
    fn test_speed_configuration() {
        let mut i2c = I2c::new();

        i2c.set_speed(I2cSpeed::Standard);
        assert_eq!(i2c.speed, I2cSpeed::Standard);

        i2c.set_speed(I2cSpeed::Fast);
        assert_eq!(i2c.speed, I2cSpeed::Fast);

        i2c.set_speed(I2cSpeed::FastPlus);
        assert_eq!(i2c.speed, I2cSpeed::FastPlus);
    }

    #[test]
    fn test_address_configuration() {
        let mut i2c = I2c::new();

        // Write address register
        i2c.write(I2C_SLAVE_ADDR_REG, 4, 0x50);

        assert_eq!(i2c.address, 0x50);
        assert_eq!(i2c.read(I2C_SLAVE_ADDR_REG, 4), 0x50);
    }

    #[test]
    fn test_tx_fifo_write() {
        let mut i2c = I2c::new();

        // Write to data register (TX FIFO)
        i2c.write(I2C_DATA_REG, 4, 0x42);
        i2c.write(I2C_DATA_REG, 4, 0x43);

        assert_eq!(i2c.tx_fifo_count(), 2);
    }

    #[test]
    fn test_rx_fifo_read() {
        let mut i2c = I2c::new();

        // Simulate RX data
        i2c.simulate_rx_data(&[0x10, 0x20, 0x30]);

        assert_eq!(i2c.rx_fifo_count(), 3);

        // Read FIFO count register
        let count = i2c.read(I2C_RXFIFO_ST_REG, 4);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_fifo_reset() {
        let mut i2c = I2c::new();

        // Fill FIFOs
        i2c.write_byte(0x11);
        i2c.simulate_rx_data(&[0x22]);

        assert_eq!(i2c.tx_fifo_count(), 1);
        assert_eq!(i2c.rx_fifo_count(), 1);

        // Reset RX FIFO
        i2c.write(I2C_FIFO_CONF_REG, 4, I2C_FIFO_CONF_RXFIFO_RST);

        assert_eq!(i2c.rx_fifo_count(), 0);
        assert_eq!(i2c.tx_fifo_count(), 1); // TX FIFO unchanged

        // Reset TX FIFO
        i2c.write(I2C_FIFO_CONF_REG, 4, I2C_FIFO_CONF_TXFIFO_RST);

        assert_eq!(i2c.tx_fifo_count(), 0);
    }

    #[test]
    fn test_master_mode() {
        let mut i2c = I2c::new();

        // Enable master mode
        i2c.write(I2C_CTR_REG, 4, I2C_CTR_MS_MODE);

        assert!(i2c.master_mode);

        // Disable master mode
        i2c.write(I2C_CTR_REG, 4, 0);

        assert!(!i2c.master_mode);
    }

    #[test]
    fn test_command_write() {
        let mut i2c = I2c::new();

        // Write command 0
        let cmd = I2C_CMD_WRITE | (5 << 8); // Write 5 bytes
        i2c.write(I2C_COMD0_REG, 4, cmd);

        assert_eq!(i2c.commands[0], cmd);
        assert_eq!(i2c.read(I2C_COMD0_REG, 4), cmd);
    }

    #[test]
    fn test_transaction_start() {
        let mut i2c = I2c::new();

        // Configure address and command
        i2c.write(I2C_SLAVE_ADDR_REG, 4, 0x50);

        // Write STOP command
        i2c.write(I2C_COMD0_REG, 4, I2C_CMD_STOP);

        // Start transaction
        i2c.write(I2C_CTR_REG, 4, I2C_CTR_MS_MODE | I2C_CTR_TRANS_START);

        // Should complete and set interrupt
        assert!(i2c.int_raw & I2C_INT_TRANS_COMPLETE != 0);
    }

    #[test]
    fn test_busy_flag() {
        let mut i2c = I2c::new();

        // Initially not busy
        assert!((i2c.status & I2C_SR_BUSY) == 0);

        // Start transaction
        i2c.start_transaction(0x50, false);

        // Should be busy
        assert!((i2c.status & I2C_SR_BUSY) != 0);
    }

    #[test]
    fn test_interrupt_enable() {
        let mut i2c = I2c::new();

        // Enable transaction complete interrupt
        i2c.write(I2C_INT_ENA_REG, 4, I2C_INT_TRANS_COMPLETE);

        assert_eq!(i2c.int_ena, I2C_INT_TRANS_COMPLETE);
    }

    #[test]
    fn test_interrupt_clear() {
        let mut i2c = I2c::new();

        // Set interrupt
        i2c.int_raw = I2C_INT_TRANS_COMPLETE;

        // Clear interrupt
        i2c.write(I2C_INT_CLR_REG, 4, I2C_INT_TRANS_COMPLETE);

        assert_eq!(i2c.int_raw, 0);
    }

    #[test]
    fn test_timing_registers() {
        let mut i2c = I2c::new();

        i2c.write(I2C_SCL_LOW_PERIOD_REG, 4, 100);
        i2c.write(I2C_SCL_HIGH_PERIOD_REG, 4, 100);
        i2c.write(I2C_SDA_HOLD_REG, 4, 10);
        i2c.write(I2C_SDA_SAMPLE_REG, 4, 10);

        assert_eq!(i2c.read(I2C_SCL_LOW_PERIOD_REG, 4), 100);
        assert_eq!(i2c.read(I2C_SCL_HIGH_PERIOD_REG, 4), 100);
        assert_eq!(i2c.read(I2C_SDA_HOLD_REG, 4), 10);
        assert_eq!(i2c.read(I2C_SDA_SAMPLE_REG, 4), 10);
    }
}
