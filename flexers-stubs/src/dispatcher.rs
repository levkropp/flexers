use std::collections::HashMap;
use std::sync::Arc;
use flexers_core::cpu::{XtensaCpu, RomStubDispatcherTrait};
use crate::handler::RomStubHandler;
use crate::symbol_table::SymbolTable;

/// ROM stub dispatcher
pub struct RomStubDispatcher {
    /// Symbol table (address → function name)
    symbol_table: Arc<SymbolTable>,

    /// Stub handlers (function name → handler)
    handlers: HashMap<String, Box<dyn RomStubHandler>>,
}

impl RomStubDispatcher {
    pub fn new(symbol_table: Arc<SymbolTable>) -> Self {
        Self {
            symbol_table,
            handlers: HashMap::new(),
        }
    }

    /// Register a stub handler for a ROM function
    pub fn register<H: RomStubHandler + 'static>(&mut self, handler: H) {
        self.handlers.insert(handler.name().to_string(), Box::new(handler));
    }

    /// Check if address is in ROM range
    pub fn is_rom_address(addr: u32) -> bool {
        // ESP32 ROM range: 0x4000_0000 - 0x4006_FFFF (448 KB)
        addr >= 0x4000_0000 && addr < 0x4007_0000
    }

    /// Dispatch ROM function call
    pub fn dispatch(&mut self, cpu: &mut XtensaCpu) -> Result<(), StubError> {
        let pc = cpu.pc();

        // Lookup function name from address
        let symbol = self.symbol_table.lookup_address(pc)
            .ok_or(StubError::UnknownFunction(pc))?;

        // Find handler
        let handler = self.handlers.get(&symbol.name)
            .ok_or_else(|| StubError::UnimplementedStub(symbol.name.clone()))?;

        // Call stub handler
        let return_value = handler.call(cpu);

        // Write return value to a2
        cpu.set_register(2, return_value);

        // Return to caller (PC ← a0)
        let return_addr = cpu.get_register(0);
        cpu.set_pc(return_addr);

        Ok(())
    }
}

#[derive(Debug)]
pub enum StubError {
    UnknownFunction(u32),
    UnimplementedStub(String),
}

impl std::fmt::Display for StubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StubError::UnknownFunction(addr) => {
                write!(f, "Unknown ROM function at address 0x{:08X}", addr)
            }
            StubError::UnimplementedStub(name) => {
                write!(f, "Unimplemented ROM stub: {}", name)
            }
        }
    }
}

impl std::error::Error for StubError {}

/// Implement the RomStubDispatcherTrait for core integration
impl RomStubDispatcherTrait for RomStubDispatcher {
    fn is_rom_address(&self, addr: u32) -> bool {
        Self::is_rom_address(addr)
    }

    fn dispatch(&mut self, cpu: &mut XtensaCpu) -> Result<(), String> {
        self.dispatch(cpu).map_err(|e| e.to_string())
    }
}
