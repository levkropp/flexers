/// Helper module for creating a fully configured ROM stub dispatcher
/// with all common ESP32 ROM functions registered.

use std::sync::Arc;
use crate::{
    dispatcher::RomStubDispatcher,
    symbol_table::SymbolTable,
    functions::{io::*, timing::*, boot::*},
};

/// Create a ROM stub dispatcher with all common ESP32 ROM functions
pub fn create_esp32_dispatcher() -> RomStubDispatcher {
    let symbol_table = Arc::new(SymbolTable::load_esp32_rom_symbols());
    let mut dispatcher = RomStubDispatcher::new(symbol_table);

    // Register I/O functions
    dispatcher.register(EspRomPrintf);
    dispatcher.register(EtsPutc);
    dispatcher.register(EtsInstallPutc1);
    dispatcher.register(Memcpy);
    dispatcher.register(Memset);
    dispatcher.register(Memcmp);
    dispatcher.register(Memmove);
    dispatcher.register(UartTxOneChar);
    dispatcher.register(UartRxOneChar);
    dispatcher.register(UartDivModify);

    // Register timing functions
    dispatcher.register(EtsDelayUs);
    dispatcher.register(EtsGetCpuFrequency);
    dispatcher.register(EtsUpdateCpuFrequency);

    // Register boot/system functions
    dispatcher.register(CacheReadEnable);
    dispatcher.register(CacheReadDisable);
    dispatcher.register(RtcGetResetReason);
    dispatcher.register(SoftwareReset);

    dispatcher
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dispatcher() {
        let _dispatcher = create_esp32_dispatcher();

        // Verify it was created successfully
        // The dispatcher should have all the registered handlers
        // We can't directly inspect the handlers, but we can verify ROM address detection
        assert!(RomStubDispatcher::is_rom_address(0x40007ABC));  // esp_rom_printf
        assert!(!RomStubDispatcher::is_rom_address(0x3FFA_0000)); // DRAM
    }

    #[test]
    fn test_rom_address_range() {
        let _dispatcher = create_esp32_dispatcher();

        // ROM range is 0x4000_0000 - 0x4006_FFFF
        assert!(RomStubDispatcher::is_rom_address(0x4000_0000));
        assert!(RomStubDispatcher::is_rom_address(0x4006_FFFF));
        assert!(!RomStubDispatcher::is_rom_address(0x4007_0000));
        assert!(!RomStubDispatcher::is_rom_address(0x3FFF_FFFF));
    }
}
