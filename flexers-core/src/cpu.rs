use crate::memory::Memory;
use std::sync::{Arc, Mutex};

/// Trait for interrupt controller (for dependency injection)
pub trait InterruptControllerTrait: Send + Sync {
    fn get_pending_interrupt(&self) -> Option<(u8, u8)>; // (source, level)
    fn set_current_level(&mut self, level: u8);
}

/// Xtensa CPU state optimized for cache performance
/// Layout: HOT (accessed every instruction) → WARM (branches/exceptions) → COLD (large arrays)
#[repr(align(64))]
pub struct XtensaCpu {
    // ===== HOT SECTION: Accessed every instruction =====
    /// Physical register file (64 registers for windowing)
    ar: [u32; 64],
    /// Program counter
    pc: u32,
    /// Window base register (for register windowing)
    windowbase: u32,
    /// Processor status register
    ps: u32,
    /// Shift amount register
    sar: u32,
    /// Loop begin/end/count registers
    lbeg: u32,
    lend: u32,
    lcount: u32,
    /// Cycle count register
    ccount: u32,
    /// Interrupt enable and pending masks
    intenable: u32,
    interrupt: u32,
    /// Boolean register
    br: u32,
    /// CPU running/halted state
    running: bool,
    halted: bool,
    /// Total cycle count
    cycle_count: u64,
    /// Memory subsystem reference
    mem: Arc<Memory>,
    /// Interrupt controller reference (optional)
    interrupt_controller: Option<Arc<Mutex<dyn InterruptControllerTrait>>>,

    // ===== WARM SECTION: Accessed on branches/exceptions =====
    /// Vector base address
    vecbase: u32,
    /// Exception cause register
    exccause: u32,
    /// Exception PC registers (one per exception level)
    epc: [u32; 7],
    /// Exception PS registers (one per exception level)
    eps: [u32; 7],
    /// Timer compare registers
    ccompare: [u32; 3],

    // ===== COLD SECTION: Large arrays =====
    /// Window spill/fill state (heap-allocated)
    spill_state: Box<SpillState>,
}

/// State for window spill/fill operations
pub struct SpillState {
    /// Spilled window data
    windows: Vec<[u32; 16]>,
}

impl XtensaCpu {
    /// Create new CPU with reset values
    pub fn new(mem: Arc<Memory>) -> Self {
        Self {
            // Initialize AR to zero
            ar: [0; 64],
            // Reset vector (ESP32 starts at 0x40000400)
            pc: 0x40000400,
            windowbase: 0,
            // PS: interrupt level 0, user mode
            ps: 0,
            sar: 0,
            lbeg: 0,
            lend: 0,
            lcount: 0,
            ccount: 0,
            intenable: 0,
            interrupt: 0,
            br: 0,
            running: true,
            halted: false,
            cycle_count: 0,
            mem,
            vecbase: 0,
            exccause: 0,
            epc: [0; 7],
            eps: [0; 7],
            ccompare: [0; 3],
            spill_state: Box::new(SpillState {
                windows: Vec::new(),
            }),
            interrupt_controller: None,
        }
    }

    /// Set interrupt controller
    pub fn set_interrupt_controller(&mut self, ic: Arc<Mutex<dyn InterruptControllerTrait>>) {
        self.interrupt_controller = Some(ic);
    }

    /// Get windowed register value
    /// Register numbering uses window base to index into physical registers
    #[inline(always)]
    pub fn get_register(&self, reg: u32) -> u32 {
        let physical_reg = ((self.windowbase * 4 + reg) & 63) as usize;
        self.ar[physical_reg]
    }

    /// Set windowed register value
    #[inline(always)]
    pub fn set_register(&mut self, reg: u32, val: u32) {
        let physical_reg = ((self.windowbase * 4 + reg) & 63) as usize;
        self.ar[physical_reg] = val;
    }

    /// Get physical register (bypassing windowing)
    #[inline(always)]
    pub fn get_ar(&self, ar: u32) -> u32 {
        self.ar[(ar & 63) as usize]
    }

    /// Set physical register (bypassing windowing)
    #[inline(always)]
    pub fn set_ar(&mut self, ar: u32, val: u32) {
        self.ar[(ar & 63) as usize] = val;
    }

    /// Get program counter
    #[inline(always)]
    pub fn pc(&self) -> u32 {
        self.pc
    }

    /// Set program counter
    #[inline(always)]
    pub fn set_pc(&mut self, pc: u32) {
        self.pc = pc;
    }

    /// Get cycle count
    #[inline(always)]
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// Increment cycle count
    #[inline(always)]
    pub fn inc_cycles(&mut self, n: u64) {
        self.cycle_count += n;
        self.ccount = self.ccount.wrapping_add(n as u32);
    }

    /// Check if CPU is running
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running && !self.halted
    }

    /// Halt the CPU
    pub fn halt(&mut self) {
        self.halted = true;
    }

    /// Get memory reference
    pub fn memory(&self) -> &Arc<Memory> {
        &self.mem
    }

    /// Rotate window (for CALL/ENTRY)
    pub fn rotate_window(&mut self, increment: i32) {
        let new_base = ((self.windowbase as i32 + increment) & 15) as u32;
        self.windowbase = new_base;
    }

    /// Read special register (RSR instruction)
    pub fn read_special_register(&self, sr: u32) -> u32 {
        match sr {
            0 => self.lbeg,
            1 => self.lend,
            2 => self.lcount,
            3 => self.sar,
            12 => self.br,
            72 => self.windowbase,
            73 => 0, // WINDOWSTART (simplified)
            83 => self.ps,
            96 => self.ccount,
            97 => self.vecbase,
            176 => self.exccause,
            177..=183 => self.epc[((sr - 177) & 7) as usize],
            192..=198 => self.eps[((sr - 192) & 7) as usize],
            226 => self.interrupt,
            228 => self.intenable,
            230..=232 => self.ccompare[((sr - 230) & 3) as usize],
            _ => 0, // Unknown SR returns 0
        }
    }

    /// Write special register (WSR instruction)
    pub fn write_special_register(&mut self, sr: u32, val: u32) {
        match sr {
            0 => self.lbeg = val,
            1 => self.lend = val,
            2 => self.lcount = val,
            3 => self.sar = val & 0x1F,
            12 => self.br = val & 0xFFFF,
            72 => self.windowbase = val & 0xF,
            73 => { /* WINDOWSTART - simplified */ }
            83 => self.ps = val,
            96 => self.ccount = val,
            97 => self.vecbase = val,
            176 => self.exccause = val,
            177..=183 => self.epc[((sr - 177) & 7) as usize] = val,
            192..=198 => self.eps[((sr - 192) & 7) as usize] = val,
            226 => self.interrupt = val,
            228 => self.intenable = val,
            230..=232 => self.ccompare[((sr - 230) & 3) as usize] = val,
            _ => { /* Unknown SR - ignore */ }
        }
    }

    /// Get current interrupt level from PS
    #[inline(always)]
    pub fn intlevel(&self) -> u32 {
        self.ps & 0xF
    }

    /// Set interrupt level
    #[inline(always)]
    pub fn set_intlevel(&mut self, level: u32) {
        self.ps = (self.ps & !0xF) | (level & 0xF);
    }

    /// Check for pending interrupts
    pub fn check_pending_interrupt(&self) -> Option<u8> {
        if let Some(ref ic) = self.interrupt_controller {
            if let Ok(ic_lock) = ic.lock() {
                if let Some((_source, level)) = ic_lock.get_pending_interrupt() {
                    // Check if priority higher than current level
                    let current_level = self.intlevel() as u8;
                    if level > current_level {
                        return Some(level);
                    }
                }
            }
        }
        None
    }

    /// Take an interrupt (called when interrupt is pending)
    pub fn take_interrupt(&mut self, level: u8) {
        // Save PC and PS to exception registers
        self.epc[level as usize] = self.pc;
        self.eps[level as usize] = self.ps;

        // Load exception handler address from vector table
        // Vector table: VECBASE + level * 4
        let vec_addr = self.vecbase + (level as u32) * 4;
        let handler_pc = self.mem.read_u32(vec_addr);

        // Update PS to new interrupt level
        self.ps = (self.ps & !0xF) | (level as u32);

        // Jump to handler
        self.pc = handler_pc;

        // Update interrupt controller current level
        if let Some(ref ic) = self.interrupt_controller {
            if let Ok(mut ic_lock) = ic.lock() {
                ic_lock.set_current_level(level);
            }
        }
    }

    /// Return from interrupt (RET instruction should call this)
    pub fn return_from_interrupt(&mut self) {
        let level = self.intlevel() as usize;

        // Restore PC and PS
        self.pc = self.epc[level];
        self.ps = self.eps[level];

        // Update interrupt controller level
        if let Some(ref ic) = self.interrupt_controller {
            if let Ok(mut ic_lock) = ic.lock() {
                ic_lock.set_current_level((self.ps & 0xF) as u8);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;

    #[test]
    fn test_register_windowing() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        // Write to register 2 with windowbase=0
        cpu.set_register(2, 0x1234);
        assert_eq!(cpu.get_register(2), 0x1234);
        assert_eq!(cpu.ar[2], 0x1234);

        // Rotate window forward
        cpu.windowbase = 1;
        // Now register 2 should map to physical AR[6]
        cpu.set_register(2, 0x5678);
        assert_eq!(cpu.get_register(2), 0x5678);
        assert_eq!(cpu.ar[6], 0x5678);

        // Old value should still be in AR[2]
        assert_eq!(cpu.ar[2], 0x1234);
    }

    #[test]
    fn test_special_registers() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        cpu.write_special_register(3, 0x10); // SAR
        assert_eq!(cpu.read_special_register(3), 0x10);

        cpu.write_special_register(83, 0x0F); // PS
        assert_eq!(cpu.read_special_register(83), 0x0F);
        assert_eq!(cpu.intlevel(), 0x0F);
    }

    #[test]
    fn test_cycle_counting() {
        let mem = Arc::new(Memory::new());
        let mut cpu = XtensaCpu::new(mem);

        assert_eq!(cpu.cycle_count(), 0);
        cpu.inc_cycles(100);
        assert_eq!(cpu.cycle_count(), 100);
        assert_eq!(cpu.ccount, 100);
    }
}
