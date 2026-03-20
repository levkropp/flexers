/// Helper module for creating a fully configured ROM stub dispatcher
/// with all common ESP32 ROM functions registered.

use std::sync::Arc;
use crate::{
    dispatcher::RomStubDispatcher,
    symbol_table::SymbolTable,
    functions::{io::*, timing::*, boot::*, memory::*, string::*, conversion::*, wifi::*, network::*},
    freertos::stubs::*,
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

    // Register memory management functions
    dispatcher.register(Malloc);
    dispatcher.register(Free);
    dispatcher.register(Calloc);
    dispatcher.register(Realloc);

    // Register string functions
    dispatcher.register(Strcpy);
    dispatcher.register(Strlen);
    dispatcher.register(Strcmp);
    dispatcher.register(Strcat);
    dispatcher.register(Strncpy);
    dispatcher.register(Strncmp);

    // Register conversion functions
    dispatcher.register(Atoi);
    dispatcher.register(Itoa);
    dispatcher.register(Strtol);

    // Register FreeRTOS task management functions
    dispatcher.register(XTaskCreate);
    dispatcher.register(VTaskDelete);
    dispatcher.register(VTaskDelay);
    dispatcher.register(VTaskDelayUntil);
    dispatcher.register(VTaskSuspend);
    dispatcher.register(VTaskResume);
    dispatcher.register(VTaskPrioritySet);
    dispatcher.register(UxTaskPriorityGet);
    dispatcher.register(XTaskGetCurrentTaskHandle);
    dispatcher.register(VTaskStartScheduler);
    dispatcher.register(TaskYield);
    dispatcher.register(XTaskGetTickCount);

    // Register FreeRTOS semaphore functions
    dispatcher.register(XSemaphoreCreateBinary);
    dispatcher.register(XSemaphoreCreateCounting);
    dispatcher.register(XSemaphoreGive);
    dispatcher.register(XSemaphoreTake);
    dispatcher.register(VSemaphoreDelete);

    // Register FreeRTOS mutex functions
    dispatcher.register(XSemaphoreCreateMutex);
    dispatcher.register(XSemaphoreCreateRecursiveMutex);
    dispatcher.register(XSemaphoreTakeMutex);
    dispatcher.register(XSemaphoreGiveMutex);

    // Register FreeRTOS queue functions
    dispatcher.register(XQueueCreate);
    dispatcher.register(XQueueSend);
    dispatcher.register(XQueueReceive);
    dispatcher.register(UxQueueMessagesWaiting);
    dispatcher.register(UxQueueSpacesAvailable);
    dispatcher.register(XQueueReset);
    dispatcher.register(VQueueDelete);

    // Register FreeRTOS event group functions
    dispatcher.register(XEventGroupCreate);
    dispatcher.register(XEventGroupSetBits);
    dispatcher.register(XEventGroupClearBits);
    dispatcher.register(XEventGroupWaitBits);
    dispatcher.register(XEventGroupGetBits);
    dispatcher.register(VEventGroupDelete);

    // Register FreeRTOS software timer functions
    dispatcher.register(XTimerCreate);
    dispatcher.register(XTimerStart);
    dispatcher.register(XTimerStop);
    dispatcher.register(XTimerReset);
    dispatcher.register(XTimerChangePeriod);
    dispatcher.register(XTimerIsTimerActive);
    dispatcher.register(XTimerDelete);

    // Register WiFi functions
    dispatcher.register(EspWifiInit);
    dispatcher.register(EspWifiDeinit);
    dispatcher.register(EspWifiSetMode);
    dispatcher.register(EspWifiGetMode);
    dispatcher.register(EspWifiStart);
    dispatcher.register(EspWifiStop);
    dispatcher.register(EspWifiConnect);
    dispatcher.register(EspWifiDisconnect);
    dispatcher.register(EspWifiSetConfig);
    dispatcher.register(EspWifiGetConfig);

    // Register network/socket functions
    dispatcher.register(Socket);
    dispatcher.register(Bind);
    dispatcher.register(Connect);
    dispatcher.register(Listen);
    dispatcher.register(Accept);
    dispatcher.register(Send);
    dispatcher.register(SendTo);
    dispatcher.register(Recv);
    dispatcher.register(RecvFrom);
    dispatcher.register(Close);
    dispatcher.register(SetSockOpt);
    dispatcher.register(GetSockOpt);
    dispatcher.register(Select);
    dispatcher.register(Getaddrinfo);
    dispatcher.register(Freeaddrinfo);

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
