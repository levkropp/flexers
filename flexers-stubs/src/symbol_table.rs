use std::collections::HashMap;
use crate::symbol::RomSymbol;

/// ROM symbol table
pub struct SymbolTable {
    /// Address → Symbol mapping
    by_address: HashMap<u32, RomSymbol>,

    /// Name → Symbol mapping
    by_name: HashMap<String, RomSymbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            by_address: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Add symbol to table
    pub fn add_symbol(&mut self, symbol: RomSymbol) {
        self.by_address.insert(symbol.address, symbol.clone());
        self.by_name.insert(symbol.name.clone(), symbol);
    }

    /// Lookup symbol by address
    pub fn lookup_address(&self, addr: u32) -> Option<&RomSymbol> {
        self.by_address.get(&addr)
    }

    /// Lookup symbol by name
    pub fn lookup_name(&self, name: &str) -> Option<&RomSymbol> {
        self.by_name.get(name)
    }

    /// Load symbols from embedded data (for common ROM functions)
    pub fn load_esp32_rom_symbols() -> Self {
        let mut table = Self::new();

        // Load from embedded symbol data
        for &(name, address, num_args) in crate::esp32_symbols::ESP32_ROM_SYMBOLS {
            table.add_symbol(RomSymbol::new_simple(
                name.to_string(),
                address,
                num_args,
            ));
        }

        table
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{ArgType, FunctionSignature};

    #[test]
    fn test_symbol_lookup() {
        let mut table = SymbolTable::new();

        table.add_symbol(RomSymbol::new(
            "esp_rom_printf".to_string(),
            0x40007ABC,
            FunctionSignature {
                num_args: 2,
                arg_types: vec![ArgType::Pointer, ArgType::VarArgs],
                return_type: ArgType::Int32,
            },
        ));

        // Lookup by address
        let sym = table.lookup_address(0x40007ABC).unwrap();
        assert_eq!(sym.name, "esp_rom_printf");

        // Lookup by name
        let sym = table.lookup_name("esp_rom_printf").unwrap();
        assert_eq!(sym.address, 0x40007ABC);
    }

    #[test]
    fn test_load_embedded_symbols() {
        let table = SymbolTable::load_esp32_rom_symbols();

        // Should have common ROM functions
        assert!(table.lookup_name("esp_rom_printf").is_some());
        assert!(table.lookup_name("ets_delay_us").is_some());
    }
}
