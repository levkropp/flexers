/// ROM symbol entry
#[derive(Debug, Clone)]
pub struct RomSymbol {
    /// Function name (e.g., "esp_rom_printf")
    pub name: String,

    /// ROM address (e.g., 0x40007ABC)
    pub address: u32,

    /// Function signature (for validation)
    pub signature: FunctionSignature,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Number of arguments
    pub num_args: u8,

    /// Argument types (for debugging)
    pub arg_types: Vec<ArgType>,

    /// Return type
    pub return_type: ArgType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Void,
    Int32,
    UInt32,
    Pointer,
    VarArgs,  // For printf-like functions
}

impl RomSymbol {
    /// Create a new ROM symbol with simple signature (no detailed arg types)
    pub fn new_simple(name: String, address: u32, num_args: u8) -> Self {
        Self {
            name,
            address,
            signature: FunctionSignature {
                num_args,
                arg_types: vec![ArgType::UInt32; num_args as usize],
                return_type: ArgType::UInt32,
            },
        }
    }

    /// Create a new ROM symbol with detailed signature
    pub fn new(name: String, address: u32, signature: FunctionSignature) -> Self {
        Self {
            name,
            address,
            signature,
        }
    }
}
