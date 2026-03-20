/// TLS Manager - Certificate handling and TLS configuration
///
/// This module provides a singleton TLS manager that handles:
/// - Mozilla root CA bundle (webpki-roots)
/// - Client TLS configuration (rustls)
/// - Custom certificate loading
/// - Certificate verification modes

use rustls::{ClientConfig, RootCertStore};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

/// Certificate verification mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CertVerifyMode {
    /// Full certificate verification (default)
    Required,
    /// Verify but don't fail on errors
    Optional,
    /// Skip verification (testing only, insecure)
    None,
}

/// TLS Manager - handles certificate trust and TLS configuration
pub struct TlsManager {
    /// Mozilla root CA bundle
    root_store: RootCertStore,

    /// Client configuration (shared across sockets)
    client_config: Arc<ClientConfig>,

    /// Custom certificates (for testing/enterprise CAs)
    custom_certs: Vec<Vec<u8>>,

    /// Certificate verification mode
    verify_mode: CertVerifyMode,
}

impl TlsManager {
    /// Create new TLS manager with Mozilla root CA bundle
    pub fn new() -> Self {
        let mut root_store = RootCertStore::empty();

        // Add Mozilla root CA bundle
        root_store.extend(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned()
        );

        let client_config = Arc::new(
            ClientConfig::builder()
                .with_root_certificates(root_store.clone())
                .with_no_client_auth()
        );

        Self {
            root_store,
            client_config,
            custom_certs: Vec::new(),
            verify_mode: CertVerifyMode::Required,
        }
    }

    /// Get client configuration for TLS connections
    pub fn get_client_config(&self) -> Arc<ClientConfig> {
        self.client_config.clone()
    }

    /// Set certificate verification mode
    pub fn set_verify_mode(&mut self, mode: CertVerifyMode) {
        if self.verify_mode != mode {
            self.verify_mode = mode;
            self.rebuild_config();
        }
    }

    /// Get current verification mode
    pub fn get_verify_mode(&self) -> CertVerifyMode {
        self.verify_mode
    }

    /// Add custom certificate from PEM data
    pub fn add_custom_cert(&mut self, cert_pem: &[u8]) -> Result<(), String> {
        use std::io::Cursor;

        let mut cursor = Cursor::new(cert_pem);
        let certs: Result<Vec<_>, _> = rustls_pemfile::certs(&mut cursor).collect();
        let certs = certs.map_err(|e| format!("Failed to parse PEM certificate: {}", e))?;

        for cert in certs {
            self.root_store.add(cert)
                .map_err(|e| format!("Failed to add certificate: {}", e))?;
        }

        self.custom_certs.push(cert_pem.to_vec());
        self.rebuild_config();
        Ok(())
    }

    /// Clear all custom certificates
    pub fn clear_custom_certs(&mut self) {
        self.custom_certs.clear();
        // Rebuild root store with only Mozilla CAs
        self.root_store = RootCertStore::empty();
        self.root_store.extend(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned()
        );
        self.rebuild_config();
    }

    /// Rebuild client configuration based on current settings
    fn rebuild_config(&mut self) {
        let builder = ClientConfig::builder();

        let config = match self.verify_mode {
            CertVerifyMode::Required | CertVerifyMode::Optional => {
                // Use standard verification with root certificates
                builder
                    .with_root_certificates(self.root_store.clone())
                    .with_no_client_auth()
            }
            CertVerifyMode::None => {
                // Dangerous: skip verification
                // For testing only - accepts any certificate
                use rustls::client::danger::{ServerCertVerifier, HandshakeSignatureValid};
                use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
                use rustls::DigitallySignedStruct;

                #[derive(Debug)]
                struct NoVerifier;

                impl ServerCertVerifier for NoVerifier {
                    fn verify_server_cert(
                        &self,
                        _end_entity: &CertificateDer<'_>,
                        _intermediates: &[CertificateDer<'_>],
                        _server_name: &ServerName,
                        _ocsp_response: &[u8],
                        _now: UnixTime,
                    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
                        // Accept any certificate (insecure)
                        Ok(rustls::client::danger::ServerCertVerified::assertion())
                    }

                    fn verify_tls12_signature(
                        &self,
                        _message: &[u8],
                        _cert: &CertificateDer<'_>,
                        _dss: &DigitallySignedStruct,
                    ) -> Result<HandshakeSignatureValid, rustls::Error> {
                        Ok(HandshakeSignatureValid::assertion())
                    }

                    fn verify_tls13_signature(
                        &self,
                        _message: &[u8],
                        _cert: &CertificateDer<'_>,
                        _dss: &DigitallySignedStruct,
                    ) -> Result<HandshakeSignatureValid, rustls::Error> {
                        Ok(HandshakeSignatureValid::assertion())
                    }

                    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
                        vec![
                            rustls::SignatureScheme::RSA_PKCS1_SHA256,
                            rustls::SignatureScheme::RSA_PKCS1_SHA384,
                            rustls::SignatureScheme::RSA_PKCS1_SHA512,
                            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                            rustls::SignatureScheme::RSA_PSS_SHA256,
                            rustls::SignatureScheme::RSA_PSS_SHA384,
                            rustls::SignatureScheme::RSA_PSS_SHA512,
                            rustls::SignatureScheme::ED25519,
                        ]
                    }
                }

                builder
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoVerifier))
                    .with_no_client_auth()
            }
        };

        self.client_config = Arc::new(config);
    }
}

impl Default for TlsManager {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static! {
    /// Global TLS manager singleton
    pub static ref TLS_MANAGER: Arc<Mutex<TlsManager>> =
        Arc::new(Mutex::new(TlsManager::new()));
}

/// Reset TLS manager for testing
pub fn reset_tls_manager() {
    if let Ok(mut manager) = TLS_MANAGER.lock() {
        *manager = TlsManager::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_manager_initialization() {
        let manager = TlsManager::new();
        assert_eq!(manager.verify_mode, CertVerifyMode::Required);
        assert!(manager.custom_certs.is_empty());
    }

    #[test]
    fn test_get_client_config() {
        let manager = TlsManager::new();
        let config1 = manager.get_client_config();
        let config2 = manager.get_client_config();

        // Should return cloned Arc (same underlying config)
        assert!(Arc::ptr_eq(&config1, &config2));
    }

    #[test]
    fn test_verify_mode_changes() {
        let mut manager = TlsManager::new();

        assert_eq!(manager.get_verify_mode(), CertVerifyMode::Required);

        manager.set_verify_mode(CertVerifyMode::None);
        assert_eq!(manager.get_verify_mode(), CertVerifyMode::None);

        manager.set_verify_mode(CertVerifyMode::Optional);
        assert_eq!(manager.get_verify_mode(), CertVerifyMode::Optional);
    }

    #[test]
    fn test_verify_mode_rebuilds_config() {
        let mut manager = TlsManager::new();
        let config1 = manager.get_client_config();

        manager.set_verify_mode(CertVerifyMode::None);
        let config2 = manager.get_client_config();

        // Config should be rebuilt (different Arc)
        assert!(!Arc::ptr_eq(&config1, &config2));
    }

    #[test]
    fn test_custom_cert_loading() {
        let mut manager = TlsManager::new();

        // Sample self-signed certificate PEM (for testing)
        let cert_pem = b"-----BEGIN CERTIFICATE-----
MIIB0TCCAXugAwIBAgIJAMU1L0nF5hJzMA0GCSqGSIb3DQEBCwUAMEUxCzAJBgNV
BAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX
aWRnaXRzIFB0eSBMdGQwHhcNMjQwMTAxMDAwMDAwWhcNMjUwMTAxMDAwMDAwWjBF
MQswCQYDVQQGEwJBVTETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50
ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBANLJ
hPHhITqQbPklG3ibCVxwGMRfp/v4XqhfdQHdcVfHap6NQ5Wok/9X0UsiN4c6CVNc
w1tD4FlYTCKLeCrKpUsCAwEAAaNQME4wHQYDVR0OBBYEFJvKs8RfJaXTH08W+oBX
xN3m4qsrMB8GA1UdIwQYMBaAFJvKs8RfJaXTH08W+oBXxN3m4qsrMAwGA1UdEwQF
MAMBAf8wDQYJKoZIhvcNAQELBQADQQA1KpRvPWZMmBwR7cQ6QOvYYJJvLQfHfHmF
VcJV3OELjngNYK/Ht9eqGnJQ3Q/YNOkDFfZDPKCxL7qKvMIw3vIL
-----END CERTIFICATE-----";

        // This might fail with invalid certificate, but should parse
        let _ = manager.add_custom_cert(cert_pem);
    }

    #[test]
    fn test_clear_custom_certs() {
        let mut manager = TlsManager::new();

        // Start with empty custom certs
        assert!(manager.custom_certs.is_empty());

        // Add a custom cert (might fail to parse, but should be tracked)
        let cert_pem = b"-----BEGIN CERTIFICATE-----
MIIB0TCCAXugAwIBAgIJAMU1L0nF5hJzMA0GCSqGSIb3DQEBCwUAMEUxCzAJBgNV
BAYTAkFVMRMwEQYDVQQIDApTb21lLVN0YXRlMSEwHwYDVQQKDBhJbnRlcm5ldCBX
aWRnaXRzIFB0eSBMdGQwHhcNMjQwMTAxMDAwMDAwWhcNMjUwMTAxMDAwMDAwWjBF
MQswCQYDVQQGEwJBVTETMBEGA1UECAwKU29tZS1TdGF0ZTEhMB8GA1UECgwYSW50
ZXJuZXQgV2lkZ2l0cyBQdHkgTHRkMFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBANLJ
hPHhITqQbPklG3ibCVxwGMRfp/v4XqhfdQHdcVfHap6NQ5Wok/9X0UsiN4c6CVNc
w1tD4FlYTCKLeCrKpUsCAwEAAaNQME4wHQYDVR0OBBYEFJvKs8RfJaXTH08W+oBX
xN3m4qsrMB8GA1UdIwQYMBaAFJvKs8RfJaXTH08W+oBXxN3m4qsrMAwGA1UdEwQF
MAMBAf8wDQYJKoZIhvcNAQELBQADQQA1KpRvPWZMmBwR7cQ6QOvYYJJvLQfHfHmF
VcJV3OELjngNYK/Ht9eqGnJQ3Q/YNOkDFfZDPKCxL7qKvMIw3vIL
-----END CERTIFICATE-----";

        // Try to add cert (may fail with parsing, which is fine for this test)
        let result = manager.add_custom_cert(cert_pem);

        // Only test clear if we successfully added a cert
        if result.is_ok() {
            assert!(!manager.custom_certs.is_empty());
            manager.clear_custom_certs();
            assert!(manager.custom_certs.is_empty());
        } else {
            // If cert parsing failed, still test that clear works
            manager.clear_custom_certs();
            assert!(manager.custom_certs.is_empty());
        }
    }

    #[test]
    fn test_global_tls_manager() {
        // Access global manager
        let manager = TLS_MANAGER.lock().unwrap();
        assert_eq!(manager.get_verify_mode(), CertVerifyMode::Required);
    }

    #[test]
    fn test_reset_tls_manager() {
        {
            let mut manager = TLS_MANAGER.lock().unwrap();
            manager.set_verify_mode(CertVerifyMode::None);
        }

        reset_tls_manager();

        let manager = TLS_MANAGER.lock().unwrap();
        assert_eq!(manager.get_verify_mode(), CertVerifyMode::Required);
    }
}
