//! Configuración TLS del servidor.

use std::sync::Arc;

use rcgen::generate_simple_self_signed;
use rustls::ServerConfig;

/// Construye la configuración TLS con un certificado self-signed.
pub fn build_tls_config() -> Arc<ServerConfig> {
    let cert = generate_simple_self_signed(["roomrtc.local".to_string()]).expect("cert");
    let cert_der = cert.serialize_der().expect("cert der");
    let key_der = cert.serialize_private_key_der();

    let rustls_cert = rustls::Certificate(cert_der);
    let rustls_key = rustls::PrivateKey(key_der);

    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![rustls_cert], rustls_key)
        .expect("config");
    Arc::new(config)
}
