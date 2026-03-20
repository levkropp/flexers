/// WiFi function stubs
///
/// These provide minimal WiFi initialization and connection stubs for testing
/// IoT firmware without real WiFi hardware.

use crate::handler::RomStubHandler;
use flexers_core::cpu::XtensaCpu;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

/// WiFi mode (STA/AP/APSTA)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiMode {
    Null = 0,
    Sta = 1,   // Station mode
    Ap = 2,    // Access Point mode
    ApSta = 3, // Both Station and AP
}

/// WiFi state
struct WifiState {
    initialized: bool,
    mode: WifiMode,
    started: bool,
    connected: bool,
}

impl WifiState {
    fn new() -> Self {
        Self {
            initialized: false,
            mode: WifiMode::Null,
            started: false,
            connected: false,
        }
    }
}

lazy_static! {
    static ref WIFI_STATE: Arc<Mutex<WifiState>> = Arc::new(Mutex::new(WifiState::new()));
}

/// ESP_OK return value
const ESP_OK: u32 = 0;

/// esp_wifi_init - Initialize WiFi
///
/// esp_err_t esp_wifi_init(const wifi_init_config_t *config);
pub struct EspWifiInit;

impl RomStubHandler for EspWifiInit {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        let mut state = WIFI_STATE.lock().unwrap();
        state.initialized = true;
        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_init"
    }
}

/// esp_wifi_deinit - Deinitialize WiFi
///
/// esp_err_t esp_wifi_deinit(void);
pub struct EspWifiDeinit;

impl RomStubHandler for EspWifiDeinit {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        let mut state = WIFI_STATE.lock().unwrap();
        state.initialized = false;
        state.started = false;
        state.connected = false;
        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_deinit"
    }
}

/// esp_wifi_set_mode - Set WiFi mode (STA/AP/APSTA)
///
/// esp_err_t esp_wifi_set_mode(wifi_mode_t mode);
pub struct EspWifiSetMode;

impl RomStubHandler for EspWifiSetMode {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let mode_val = cpu.get_ar(2);
        let mut state = WIFI_STATE.lock().unwrap();

        state.mode = match mode_val {
            1 => WifiMode::Sta,
            2 => WifiMode::Ap,
            3 => WifiMode::ApSta,
            _ => WifiMode::Null,
        };

        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_set_mode"
    }
}

/// esp_wifi_get_mode - Get WiFi mode
///
/// esp_err_t esp_wifi_get_mode(wifi_mode_t *mode);
pub struct EspWifiGetMode;

impl RomStubHandler for EspWifiGetMode {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let mode_ptr = cpu.get_ar(2);
        let state = WIFI_STATE.lock().unwrap();

        if mode_ptr != 0 {
            cpu.memory().write_u32(mode_ptr, state.mode as u32);
        }

        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_get_mode"
    }
}

/// esp_wifi_start - Start WiFi
///
/// esp_err_t esp_wifi_start(void);
pub struct EspWifiStart;

impl RomStubHandler for EspWifiStart {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        let mut state = WIFI_STATE.lock().unwrap();
        if state.initialized {
            state.started = true;
            ESP_OK
        } else {
            0x3001 // ESP_ERR_WIFI_NOT_INIT
        }
    }

    fn name(&self) -> &str {
        "esp_wifi_start"
    }
}

/// esp_wifi_stop - Stop WiFi
///
/// esp_err_t esp_wifi_stop(void);
pub struct EspWifiStop;

impl RomStubHandler for EspWifiStop {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        let mut state = WIFI_STATE.lock().unwrap();
        state.started = false;
        state.connected = false;
        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_stop"
    }
}

/// esp_wifi_connect - Connect to AP
///
/// esp_err_t esp_wifi_connect(void);
pub struct EspWifiConnect;

impl RomStubHandler for EspWifiConnect {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        let mut state = WIFI_STATE.lock().unwrap();
        if state.started {
            // Simulate immediate successful connection
            state.connected = true;
            ESP_OK
        } else {
            0x3002 // ESP_ERR_WIFI_NOT_STARTED
        }
    }

    fn name(&self) -> &str {
        "esp_wifi_connect"
    }
}

/// esp_wifi_disconnect - Disconnect from AP
///
/// esp_err_t esp_wifi_disconnect(void);
pub struct EspWifiDisconnect;

impl RomStubHandler for EspWifiDisconnect {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        let mut state = WIFI_STATE.lock().unwrap();
        state.connected = false;
        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_disconnect"
    }
}

/// esp_wifi_set_config - Set WiFi configuration
///
/// esp_err_t esp_wifi_set_config(wifi_interface_t interface, wifi_config_t *conf);
pub struct EspWifiSetConfig;

impl RomStubHandler for EspWifiSetConfig {
    fn call(&self, _cpu: &mut XtensaCpu) -> u32 {
        // Accept any configuration (we don't actually parse it)
        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_set_config"
    }
}

/// esp_wifi_get_config - Get WiFi configuration
///
/// esp_err_t esp_wifi_get_config(wifi_interface_t interface, wifi_config_t *conf);
pub struct EspWifiGetConfig;

impl RomStubHandler for EspWifiGetConfig {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let conf_ptr = cpu.get_ar(3);

        // Return a zeroed config structure
        if conf_ptr != 0 {
            for i in 0..128 {  // wifi_config_t is ~100 bytes
                cpu.memory().write_u8(conf_ptr + i, 0);
            }
        }

        ESP_OK
    }

    fn name(&self) -> &str {
        "esp_wifi_get_config"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flexers_core::memory::Memory;
    use std::sync::Arc;

    fn create_test_cpu() -> XtensaCpu {
        XtensaCpu::new(Arc::new(Memory::new()))
    }

    #[test]
    fn test_wifi_init() {
        let mut cpu = create_test_cpu();
        let stub = EspWifiInit;
        let result = stub.call(&mut cpu);
        assert_eq!(result, ESP_OK);

        let state = WIFI_STATE.lock().unwrap();
        assert!(state.initialized);
    }

    #[test]
    fn test_wifi_set_get_mode() {
        let mut cpu = create_test_cpu();

        // Set mode to STA
        cpu.set_ar(2, 1);
        let set_stub = EspWifiSetMode;
        let result = set_stub.call(&mut cpu);
        assert_eq!(result, ESP_OK);

        // Get mode
        let mode_ptr = 0x3FFE_0000;
        cpu.set_ar(2, mode_ptr);
        let get_stub = EspWifiGetMode;
        let result = get_stub.call(&mut cpu);
        assert_eq!(result, ESP_OK);
        assert_eq!(cpu.memory().read_u32(mode_ptr), 1);
    }

    #[test]
    fn test_wifi_start_without_init() {
        // First deinit to ensure clean state
        let mut cpu = create_test_cpu();
        let deinit_stub = EspWifiDeinit;
        deinit_stub.call(&mut cpu);

        // Now try to start without init
        let stub = EspWifiStart;
        let result = stub.call(&mut cpu);
        assert_ne!(result, ESP_OK);  // Should fail without init
    }

    #[test]
    fn test_wifi_connect_flow() {
        // Reset and initialize
        *WIFI_STATE.lock().unwrap() = WifiState::new();

        let mut cpu = create_test_cpu();

        // Init
        let init_stub = EspWifiInit;
        assert_eq!(init_stub.call(&mut cpu), ESP_OK);

        // Start
        let start_stub = EspWifiStart;
        assert_eq!(start_stub.call(&mut cpu), ESP_OK);

        // Connect
        let connect_stub = EspWifiConnect;
        assert_eq!(connect_stub.call(&mut cpu), ESP_OK);

        let state = WIFI_STATE.lock().unwrap();
        assert!(state.connected);
    }

    #[test]
    fn test_wifi_disconnect() {
        // Setup connected state
        *WIFI_STATE.lock().unwrap() = WifiState {
            initialized: true,
            mode: WifiMode::Sta,
            started: true,
            connected: true,
        };

        let mut cpu = create_test_cpu();
        let stub = EspWifiDisconnect;
        let result = stub.call(&mut cpu);
        assert_eq!(result, ESP_OK);

        let state = WIFI_STATE.lock().unwrap();
        assert!(!state.connected);
    }
}
