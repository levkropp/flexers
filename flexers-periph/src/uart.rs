use flexers_core::memory::MmioHandler;
use crate::interrupt::InterruptSource;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// UART register offsets (simplified ESP32 UART)
const UART_FIFO_REG: u32 = 0x00;
const UART_STATUS_REG: u32 = 0x1C;
const UART_CONF0_REG: u32 = 0x20;
const UART_INT_RAW_REG: u32 = 0x04;
const UART_INT_ENA_REG: u32 = 0x0C;
const UART_INT_CLR_REG: u32 = 0x10;

/// UART interrupt bits
const UART_RXFIFO_FULL_INT: u32 = 1 << 0;
const UART_TXFIFO_EMPTY_INT: u32 = 1 << 1;
const UART_FRM_ERR_INT: u32 = 1 << 3;

/// Trait for interrupt raising (for dependency injection)
pub trait InterruptRaiser: Send + Sync {
    fn raise(&mut self, source: InterruptSource);
}

/// UART peripheral (models one UART instance)
pub struct Uart {
    /// RX FIFO (bytes received)
    rx_fifo: VecDeque<u8>,

    /// TX FIFO (bytes to transmit)
    tx_fifo: VecDeque<u8>,

    /// Configuration register
    conf0: u32,

    /// Interrupt raw status
    int_raw: u32,

    /// Interrupt enable mask
    int_ena: u32,

    /// FIFO size limit
    fifo_size: usize,

    /// Interrupt raiser reference
    int_raiser: Option<Arc<Mutex<dyn InterruptRaiser>>>,

    /// Interrupt source ID
    int_source: InterruptSource,

    /// Output callback for TX data (for testing/logging)
    tx_callback: Option<Arc<dyn Fn(u8) + Send + Sync>>,
}

impl Uart {
    pub fn new(int_source: InterruptSource) -> Self {
        Self {
            rx_fifo: VecDeque::with_capacity(128),
            tx_fifo: VecDeque::with_capacity(128),
            conf0: 0,
            int_raw: 0,
            int_ena: 0,
            fifo_size: 128,
            int_raiser: None,
            int_source,
            tx_callback: None,
        }
    }

    pub fn set_interrupt_raiser(&mut self, raiser: Arc<Mutex<dyn InterruptRaiser>>) {
        self.int_raiser = Some(raiser);
    }

    pub fn set_tx_callback<F>(&mut self, callback: F)
    where
        F: Fn(u8) + Send + Sync + 'static,
    {
        self.tx_callback = Some(Arc::new(callback));
    }

    /// Inject byte into RX FIFO (for testing/simulation)
    pub fn inject_rx(&mut self, byte: u8) {
        if self.rx_fifo.len() < self.fifo_size {
            self.rx_fifo.push_back(byte);

            // Set RX interrupt if threshold reached
            if self.rx_fifo.len() >= 8 { // Threshold
                self.int_raw |= UART_RXFIFO_FULL_INT;
                self.raise_interrupt();
            }
        }
    }

    fn raise_interrupt(&self) {
        if (self.int_raw & self.int_ena) != 0 {
            if let Some(ref raiser) = self.int_raiser {
                if let Ok(mut raiser_lock) = raiser.lock() {
                    raiser_lock.raise(self.int_source);
                }
            }
        }
    }

    fn process_tx(&mut self) {
        while let Some(byte) = self.tx_fifo.pop_front() {
            // Call TX callback if registered
            if let Some(ref callback) = self.tx_callback {
                callback(byte);
            }
        }

        // TX complete - set interrupt
        self.int_raw |= UART_TXFIFO_EMPTY_INT;
        self.raise_interrupt();
    }
}

impl MmioHandler for Uart {
    fn read(&self, addr: u32, _size: u8) -> u32 {
        match addr & 0xFF {
            0x00 => { // UART_FIFO_REG
                // Read from RX FIFO
                if let Some(byte) = self.rx_fifo.front() {
                    *byte as u32
                } else {
                    0
                }
            }
            0x1C => { // UART_STATUS_REG
                let rxfifo_cnt = self.rx_fifo.len() as u32;
                let txfifo_cnt = self.tx_fifo.len() as u32;
                (rxfifo_cnt << 0) | (txfifo_cnt << 16)
            }
            0x20 => self.conf0, // UART_CONF0_REG
            0x04 => self.int_raw, // UART_INT_RAW_REG
            0x0C => self.int_ena, // UART_INT_ENA_REG
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, _size: u8, val: u32) {
        match addr & 0xFF {
            0x00 => { // UART_FIFO_REG
                // Write to TX FIFO
                if self.tx_fifo.len() < self.fifo_size {
                    self.tx_fifo.push_back(val as u8);
                    self.process_tx(); // Process immediately
                }
            }
            0x20 => self.conf0 = val, // UART_CONF0_REG
            0x0C => self.int_ena = val, // UART_INT_ENA_REG
            0x10 => { // UART_INT_CLR_REG
                // Clear interrupts
                self.int_raw &= !val;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyInterruptRaiser {
        raised: Vec<InterruptSource>,
    }

    impl InterruptRaiser for DummyInterruptRaiser {
        fn raise(&mut self, source: InterruptSource) {
            self.raised.push(source);
        }
    }

    #[test]
    fn test_uart_tx() {
        use std::sync::{Arc, Mutex};

        let mut uart = Uart::new(InterruptSource::Uart0);
        let output = Arc::new(Mutex::new(Vec::new()));

        uart.set_tx_callback({
            let output = output.clone();
            move |byte| {
                output.lock().unwrap().push(byte);
            }
        });

        // Write byte to TX FIFO
        uart.write(0x00, 1, b'A' as u32);

        // Verify byte was transmitted
        let output_vec = output.lock().unwrap();
        assert_eq!(*output_vec, vec![b'A']);
    }

    #[test]
    fn test_uart_rx() {
        let mut uart = Uart::new(InterruptSource::Uart0);

        // Inject byte into RX FIFO
        uart.inject_rx(b'B');

        // Read status - should show 1 byte in RX FIFO
        let status = uart.read(0x1C, 4);
        assert_eq!(status & 0xFF, 1);

        // Read byte from FIFO
        let byte = uart.read(0x00, 4);
        assert_eq!(byte as u8, b'B');
    }

    #[test]
    fn test_uart_rx_interrupt() {
        let mut uart = Uart::new(InterruptSource::Uart0);
        let raiser = Arc::new(Mutex::new(DummyInterruptRaiser { raised: Vec::new() }));
        uart.set_interrupt_raiser(raiser.clone());

        // Enable RX interrupt
        uart.write(0x0C, 4, UART_RXFIFO_FULL_INT);

        // Inject enough bytes to trigger threshold (8 bytes)
        for _ in 0..8 {
            uart.inject_rx(b'X');
        }

        // Verify interrupt was raised
        let raiser_lock = raiser.lock().unwrap();
        assert!(!raiser_lock.raised.is_empty());
        assert_eq!(raiser_lock.raised[0] as u8, InterruptSource::Uart0 as u8);
    }

    #[test]
    fn test_uart_interrupt_clear() {
        let mut uart = Uart::new(InterruptSource::Uart0);

        // Manually set interrupt
        uart.int_raw = UART_RXFIFO_FULL_INT;

        // Clear interrupt
        uart.write(0x10, 4, UART_RXFIFO_FULL_INT);

        // Verify cleared
        assert_eq!(uart.int_raw, 0);
    }
}
