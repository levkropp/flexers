pub mod symbol;
pub mod symbol_table;
pub mod handler;
pub mod dispatcher;
pub mod esp32_symbols;
pub mod functions;
pub mod registry;

// Re-export commonly used types
pub use symbol::{RomSymbol, FunctionSignature, ArgType};
pub use symbol_table::SymbolTable;
pub use handler::RomStubHandler;
pub use dispatcher::{RomStubDispatcher, StubError};
pub use registry::create_esp32_dispatcher;
