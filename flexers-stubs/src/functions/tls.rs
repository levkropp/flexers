/// mbedTLS ROM stub implementations
///
/// This module provides stubs for ESP32 mbedTLS ROM functions, bridging
/// firmware TLS calls to rustls-based TLS implementation.
///
/// mbedTLS is the TLS library used in ESP-IDF. These stubs emulate the
/// mbedTLS API while using rustls for actual TLS operations.

use flexers_core::cpu::XtensaCpu;
use crate::handler::RomStubHandler;
use super::socket_manager::{SOCKET_MANAGER, TlsHandshakeState};

// mbedTLS error codes
const MBEDTLS_SUCCESS: u32 = 0;
const MBEDTLS_ERR_SSL_WANT_READ: u32 = 0xFFFF_9000;  // -0x6E00
const MBEDTLS_ERR_SSL_WANT_WRITE: u32 = 0xFFFF_9080; // -0x6F80
const MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE: u32 = 0xFFFF_7080; // -0x7280
const MBEDTLS_ERR_SSL_BAD_INPUT_DATA: u32 = 0xFFFF_8900; // -0x7700
const MBEDTLS_ERR_X509_INVALID_FORMAT: u32 = 0xFFFF_4C80; // -0x2080

// Verification modes
const MBEDTLS_SSL_VERIFY_NONE: u32 = 0;
const MBEDTLS_SSL_VERIFY_OPTIONAL: u32 = 1;
const MBEDTLS_SSL_VERIFY_REQUIRED: u32 = 2;

/// Helper: Read string from firmware memory
fn read_string_from_memory(cpu: &XtensaCpu, ptr: u32, max_len: usize) -> String {
    let mut result = String::new();
    for i in 0..max_len {
        let byte = cpu.memory().read_u8(ptr + i as u32);
        if byte == 0 {
            break;
        }
        result.push(byte as char);
    }
    result
}

/// Helper: Write string to firmware memory
fn write_string_to_memory(cpu: &mut XtensaCpu, ptr: u32, s: &str) {
    for (i, &byte) in s.as_bytes().iter().enumerate() {
        cpu.memory().write_u8(ptr + i as u32, byte);
    }
    cpu.memory().write_u8(ptr + s.len() as u32, 0); // Null terminator
}

// =============================================================================
// SSL Context Initialization
// =============================================================================

/// mbedtls_ssl_init - Initialize SSL context
///
/// Firmware signature: void mbedtls_ssl_init(mbedtls_ssl_context *ssl)
pub struct MbedtlsSslInit;

impl RomStubHandler for MbedtlsSslInit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);

        // Initialize SSL context structure in firmware memory
        // Structure layout (simplified):
        // Offset 0: socket FD (u32)
        // Offset 4: server name pointer (u32)
        // Offset 8: handshake state (u32)
        // Offset 12: verification mode (u32)

        cpu.memory().write_u32(ssl_ctx_ptr + 0, 0xFFFFFFFF); // Invalid FD
        cpu.memory().write_u32(ssl_ctx_ptr + 4, 0); // No server name
        cpu.memory().write_u32(ssl_ctx_ptr + 8, 0); // NotStarted
        cpu.memory().write_u32(ssl_ctx_ptr + 12, MBEDTLS_SSL_VERIFY_REQUIRED); // Default verify mode

        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_init"
    }
}

/// mbedtls_ssl_config_init - Initialize SSL configuration
pub struct MbedtlsSslConfigInit;

impl RomStubHandler for MbedtlsSslConfigInit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _conf_ptr = cpu.get_ar(2);
        // Config is global in our implementation (TLS_MANAGER)
        // Just return success
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_config_init"
    }
}

/// mbedtls_ssl_config_defaults - Set default SSL configuration
///
/// Firmware signature:
/// int mbedtls_ssl_config_defaults(
///     mbedtls_ssl_config *conf,
///     int endpoint,  // 0 = client, 1 = server
///     int transport, // 0 = stream (TLS), 1 = datagram (DTLS)
///     int preset     // 0 = default
/// )
pub struct MbedtlsSslConfigDefaults;

impl RomStubHandler for MbedtlsSslConfigDefaults {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _conf_ptr = cpu.get_ar(2);
        let endpoint = cpu.get_ar(3);
        let _transport = cpu.get_ar(4);
        let _preset = cpu.get_ar(5);

        // Only support client mode for now
        if endpoint != 0 {
            return MBEDTLS_ERR_SSL_BAD_INPUT_DATA;
        }

        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_config_defaults"
    }
}

/// mbedtls_ssl_setup - Setup SSL context with configuration
pub struct MbedtlsSslSetup;

impl RomStubHandler for MbedtlsSslSetup {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ssl_ctx_ptr = cpu.get_ar(2);
        let _conf_ptr = cpu.get_ar(3);

        // Configuration is applied globally, nothing to do here
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_setup"
    }
}

/// mbedtls_ssl_free - Free SSL context
pub struct MbedtlsSslFree;

impl RomStubHandler for MbedtlsSslFree {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);

        // Clear SSL context
        cpu.memory().write_u32(ssl_ctx_ptr + 0, 0xFFFFFFFF);
        cpu.memory().write_u32(ssl_ctx_ptr + 4, 0);
        cpu.memory().write_u32(ssl_ctx_ptr + 8, 0);

        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_free"
    }
}

// =============================================================================
// Certificate Management
// =============================================================================

/// mbedtls_x509_crt_init - Initialize X509 certificate
pub struct MbedtlsX509CrtInit;

impl RomStubHandler for MbedtlsX509CrtInit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _crt_ptr = cpu.get_ar(2);
        // Certificates are managed by TLS_MANAGER
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_x509_crt_init"
    }
}

/// mbedtls_x509_crt_parse - Parse X509 certificate from PEM/DER
///
/// Firmware signature:
/// int mbedtls_x509_crt_parse(
///     mbedtls_x509_crt *chain,
///     const unsigned char *buf,
///     size_t buflen
/// )
pub struct MbedtlsX509CrtParse;

impl RomStubHandler for MbedtlsX509CrtParse {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _crt_ptr = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let buf_len = cpu.get_ar(4) as usize;

        // Read certificate from firmware memory
        let mut cert_data = vec![0u8; buf_len];
        for i in 0..buf_len {
            cert_data[i] = cpu.memory().read_u8(buf_ptr + i as u32);
        }

        // Add to TLS_MANAGER
        if let Ok(mut manager) = super::tls_manager::TLS_MANAGER.lock() {
            match manager.add_custom_cert(&cert_data) {
                Ok(_) => MBEDTLS_SUCCESS,
                Err(_) => MBEDTLS_ERR_X509_INVALID_FORMAT,
            }
        } else {
            MBEDTLS_ERR_X509_INVALID_FORMAT
        }
    }

    fn name(&self) -> &str {
        "mbedtls_x509_crt_parse"
    }
}

/// mbedtls_x509_crt_free - Free X509 certificate
pub struct MbedtlsX509CrtFree;

impl RomStubHandler for MbedtlsX509CrtFree {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _crt_ptr = cpu.get_ar(2);
        // Certificates are managed by TLS_MANAGER
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_x509_crt_free"
    }
}

// =============================================================================
// SSL Configuration
// =============================================================================

/// mbedtls_ssl_conf_authmode - Set certificate verification mode
///
/// Firmware signature:
/// void mbedtls_ssl_conf_authmode(
///     mbedtls_ssl_config *conf,
///     int authmode  // MBEDTLS_SSL_VERIFY_*
/// )
pub struct MbedtlsSslConfAuthmode;

impl RomStubHandler for MbedtlsSslConfAuthmode {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);
        let authmode = cpu.get_ar(3);

        // Store verification mode in SSL context
        cpu.memory().write_u32(ssl_ctx_ptr + 12, authmode);

        // Update TLS_MANAGER
        if let Ok(mut manager) = super::tls_manager::TLS_MANAGER.lock() {
            use super::tls_manager::CertVerifyMode;
            let mode = match authmode {
                MBEDTLS_SSL_VERIFY_NONE => CertVerifyMode::None,
                MBEDTLS_SSL_VERIFY_OPTIONAL => CertVerifyMode::Optional,
                MBEDTLS_SSL_VERIFY_REQUIRED => CertVerifyMode::Required,
                _ => CertVerifyMode::Required,
            };
            manager.set_verify_mode(mode);
        }

        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_conf_authmode"
    }
}

/// mbedtls_ssl_conf_ca_chain - Set trusted CA chain
pub struct MbedtlsSslConfCaChain;

impl RomStubHandler for MbedtlsSslConfCaChain {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _conf_ptr = cpu.get_ar(2);
        let _ca_chain_ptr = cpu.get_ar(3);
        let _ca_crl_ptr = cpu.get_ar(4);

        // CA chain is managed by TLS_MANAGER (Mozilla roots + custom certs)
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_conf_ca_chain"
    }
}

/// mbedtls_ssl_conf_rng - Set random number generator
pub struct MbedtlsSslConfRng;

impl RomStubHandler for MbedtlsSslConfRng {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _conf_ptr = cpu.get_ar(2);
        let _f_rng = cpu.get_ar(3);
        let _p_rng = cpu.get_ar(4);

        // rustls handles RNG internally
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_conf_rng"
    }
}

/// mbedtls_ssl_set_hostname - Set SNI hostname
///
/// Firmware signature:
/// int mbedtls_ssl_set_hostname(
///     mbedtls_ssl_context *ssl,
///     const char *hostname
/// )
pub struct MbedtlsSslSetHostname;

impl RomStubHandler for MbedtlsSslSetHostname {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);
        let hostname_ptr = cpu.get_ar(3);

        // Store hostname pointer in SSL context
        cpu.memory().write_u32(ssl_ctx_ptr + 4, hostname_ptr);

        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_set_hostname"
    }
}

/// mbedtls_ssl_set_bio - Set underlying I/O (socket)
///
/// Firmware signature:
/// void mbedtls_ssl_set_bio(
///     mbedtls_ssl_context *ssl,
///     void *p_bio,  // Socket FD in our case
///     mbedtls_ssl_send_t *f_send,
///     mbedtls_ssl_recv_t *f_recv,
///     mbedtls_ssl_recv_timeout_t *f_recv_timeout
/// )
pub struct MbedtlsSslSetBio;

impl RomStubHandler for MbedtlsSslSetBio {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);
        let p_bio = cpu.get_ar(3); // Socket FD
        let _f_send = cpu.get_ar(4);
        let _f_recv = cpu.get_ar(5);
        let _f_recv_timeout = cpu.get_ar(6);

        // Store socket FD in SSL context
        cpu.memory().write_u32(ssl_ctx_ptr + 0, p_bio);

        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_set_bio"
    }
}

// =============================================================================
// TLS Handshake
// =============================================================================

/// mbedtls_ssl_handshake - Perform TLS handshake (non-blocking)
///
/// Firmware signature:
/// int mbedtls_ssl_handshake(mbedtls_ssl_context *ssl)
///
/// Returns:
/// - 0 on success (handshake complete)
/// - MBEDTLS_ERR_SSL_WANT_READ if handshake in progress
/// - MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE on failure
pub struct MbedtlsSslHandshake;

impl RomStubHandler for MbedtlsSslHandshake {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);

        // Read SSL context
        let sockfd = cpu.memory().read_u32(ssl_ctx_ptr + 0);
        let hostname_ptr = cpu.memory().read_u32(ssl_ctx_ptr + 4);
        let state = cpu.memory().read_u32(ssl_ctx_ptr + 8);

        if sockfd == 0xFFFFFFFF {
            return MBEDTLS_ERR_SSL_BAD_INPUT_DATA;
        }

        let result = if state == 0 {
            // First call: start handshake
            let hostname = if hostname_ptr != 0 {
                read_string_from_memory(cpu, hostname_ptr, 256)
            } else {
                String::from("localhost")
            };

            if let Ok(mut manager) = SOCKET_MANAGER.lock() {
                if let Some(socket) = manager.get_mut(sockfd) {
                    match socket.start_tls_handshake(&hostname) {
                        Ok(_) => {
                            let hs_state = socket.get_tls_handshake_state();
                            match hs_state {
                                TlsHandshakeState::Complete => {
                                    cpu.memory().write_u32(ssl_ctx_ptr + 8, 2);
                                    MBEDTLS_SUCCESS
                                }
                                TlsHandshakeState::InProgress => {
                                    cpu.memory().write_u32(ssl_ctx_ptr + 8, 1);
                                    MBEDTLS_ERR_SSL_WANT_READ
                                }
                                _ => {
                                    cpu.memory().write_u32(ssl_ctx_ptr + 8, 3);
                                    MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE
                                }
                            }
                        }
                        Err(_) => {
                            cpu.memory().write_u32(ssl_ctx_ptr + 8, 3);
                            MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE
                        }
                    }
                } else {
                    MBEDTLS_ERR_SSL_BAD_INPUT_DATA
                }
            } else {
                MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE
            }
        } else if state == 1 {
            // Subsequent call: continue handshake
            if let Ok(mut manager) = SOCKET_MANAGER.lock() {
                if let Some(socket) = manager.get_mut(sockfd) {
                    match socket.continue_tls_handshake() {
                        Ok(TlsHandshakeState::Complete) => {
                            cpu.memory().write_u32(ssl_ctx_ptr + 8, 2);
                            MBEDTLS_SUCCESS
                        }
                        Ok(TlsHandshakeState::InProgress) => {
                            MBEDTLS_ERR_SSL_WANT_READ
                        }
                        _ => {
                            cpu.memory().write_u32(ssl_ctx_ptr + 8, 3);
                            MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE
                        }
                    }
                } else {
                    MBEDTLS_ERR_SSL_BAD_INPUT_DATA
                }
            } else {
                MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE
            }
        } else {
            // Already complete or failed
            if state == 2 {
                MBEDTLS_SUCCESS
            } else {
                MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE
            }
        };

        result
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_handshake"
    }
}

// =============================================================================
// Encrypted I/O
// =============================================================================

/// mbedtls_ssl_read - Read encrypted data
///
/// Firmware signature:
/// int mbedtls_ssl_read(
///     mbedtls_ssl_context *ssl,
///     unsigned char *buf,
///     size_t len
/// )
///
/// Returns:
/// - Number of bytes read on success
/// - MBEDTLS_ERR_SSL_WANT_READ if would block
/// - Negative error code on failure
pub struct MbedtlsSslRead;

impl RomStubHandler for MbedtlsSslRead {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4) as usize;

        let sockfd = cpu.memory().read_u32(ssl_ctx_ptr + 0);

        if sockfd == 0xFFFFFFFF {
            return MBEDTLS_ERR_SSL_BAD_INPUT_DATA;
        }

        // Receive data (SocketState handles TLS decryption)
        let result = if let Ok(mut manager) = SOCKET_MANAGER.lock() {
            if let Some(socket) = manager.get_mut(sockfd) {
                match socket.recv(len) {
                    Ok(data) => {
                        if data.is_empty() {
                            MBEDTLS_ERR_SSL_WANT_READ
                        } else {
                            // Copy to firmware memory
                            for (i, &byte) in data.iter().enumerate() {
                                cpu.memory().write_u8(buf_ptr + i as u32, byte);
                            }
                            data.len() as u32
                        }
                    }
                    Err(_) => MBEDTLS_ERR_SSL_WANT_READ,
                }
            } else {
                MBEDTLS_ERR_SSL_BAD_INPUT_DATA
            }
        } else {
            MBEDTLS_ERR_SSL_WANT_READ
        };

        result
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_read"
    }
}

/// mbedtls_ssl_write - Write encrypted data
///
/// Firmware signature:
/// int mbedtls_ssl_write(
///     mbedtls_ssl_context *ssl,
///     const unsigned char *buf,
///     size_t len
/// )
///
/// Returns:
/// - Number of bytes written on success
/// - MBEDTLS_ERR_SSL_WANT_WRITE if would block
/// - Negative error code on failure
pub struct MbedtlsSslWrite;

impl RomStubHandler for MbedtlsSslWrite {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let ssl_ctx_ptr = cpu.get_ar(2);
        let buf_ptr = cpu.get_ar(3);
        let len = cpu.get_ar(4) as usize;

        let sockfd = cpu.memory().read_u32(ssl_ctx_ptr + 0);

        if sockfd == 0xFFFFFFFF {
            return MBEDTLS_ERR_SSL_BAD_INPUT_DATA;
        }

        // Copy from firmware memory
        let mut data = vec![0u8; len];
        for i in 0..len {
            data[i] = cpu.memory().read_u8(buf_ptr + i as u32);
        }

        // Send data (SocketState handles TLS encryption)
        let result = if let Ok(mut manager) = SOCKET_MANAGER.lock() {
            if let Some(socket) = manager.get_mut(sockfd) {
                match socket.send(&data) {
                    Ok(sent) => sent as u32,
                    Err(_) => MBEDTLS_ERR_SSL_WANT_WRITE,
                }
            } else {
                MBEDTLS_ERR_SSL_BAD_INPUT_DATA
            }
        } else {
            MBEDTLS_ERR_SSL_WANT_WRITE
        };

        result
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_write"
    }
}

/// mbedtls_ssl_close_notify - Send close_notify alert
pub struct MbedtlsSslCloseNotify;

impl RomStubHandler for MbedtlsSslCloseNotify {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ssl_ctx_ptr = cpu.get_ar(2);

        // rustls handles close_notify automatically
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ssl_close_notify"
    }
}

// =============================================================================
// RNG (Random Number Generator)
// =============================================================================

/// mbedtls_ctr_drbg_init - Initialize CTR_DRBG context
pub struct MbedtlsCtrDrbgInit;

impl RomStubHandler for MbedtlsCtrDrbgInit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ctx_ptr = cpu.get_ar(2);
        // rustls handles RNG internally
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ctr_drbg_init"
    }
}

/// mbedtls_ctr_drbg_seed - Seed CTR_DRBG
pub struct MbedtlsCtrDrbgSeed;

impl RomStubHandler for MbedtlsCtrDrbgSeed {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ctx_ptr = cpu.get_ar(2);
        let _f_entropy = cpu.get_ar(3);
        let _p_entropy = cpu.get_ar(4);
        let _custom = cpu.get_ar(5);
        let _len = cpu.get_ar(6);

        // rustls handles RNG internally
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_ctr_drbg_seed"
    }
}

/// mbedtls_entropy_init - Initialize entropy context
pub struct MbedtlsEntropyInit;

impl RomStubHandler for MbedtlsEntropyInit {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ctx_ptr = cpu.get_ar(2);
        // rustls handles entropy internally
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_entropy_init"
    }
}

/// mbedtls_entropy_free - Free entropy context
pub struct MbedtlsEntropyFree;

impl RomStubHandler for MbedtlsEntropyFree {
    fn call(&self, cpu: &mut XtensaCpu) -> u32 {
        let _ctx_ptr = cpu.get_ar(2);
        // rustls handles entropy internally
        MBEDTLS_SUCCESS
    }

    fn name(&self) -> &str {
        "mbedtls_entropy_free"
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mbedtls_error_codes() {
        // Verify error codes are in valid range
        assert_eq!(MBEDTLS_SUCCESS, 0);
        assert_ne!(MBEDTLS_ERR_SSL_WANT_READ, 0);
        assert_ne!(MBEDTLS_ERR_SSL_HANDSHAKE_FAILURE, 0);
    }

    #[test]
    fn test_handler_names() {
        // Verify handler names
        assert_eq!(MbedtlsSslInit.name(), "mbedtls_ssl_init");
        assert_eq!(MbedtlsSslHandshake.name(), "mbedtls_ssl_handshake");
        assert_eq!(MbedtlsSslRead.name(), "mbedtls_ssl_read");
        assert_eq!(MbedtlsSslWrite.name(), "mbedtls_ssl_write");
    }

    #[test]
    fn test_verify_modes_defined() {
        // Verify verification mode constants
        assert_eq!(MBEDTLS_SSL_VERIFY_NONE, 0);
        assert_eq!(MBEDTLS_SSL_VERIFY_OPTIONAL, 1);
        assert_eq!(MBEDTLS_SSL_VERIFY_REQUIRED, 2);
    }
}
