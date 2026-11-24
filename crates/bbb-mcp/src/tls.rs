//! TLS for a remotely reachable management endpoint.
//!
//! Vanilla owner: `JsonRpcSslContextProvider`
//! (security/JsonRpcSslContextProvider.java), which builds an SSL context from
//! a configured keystore for the same reason — a management socket that leaves
//! the host carries credentials and administrative answers.
//!
//! bbb reads PEM rather than a JKS keystore, and terminates with rustls over
//! the ring provider the rest of the workspace already installs, so no C
//! toolchain is inherited.

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rustls::{
    pki_types::{
        CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer,
    },
    ServerConfig,
};
use thiserror::Error;
use tokio_rustls::TlsAcceptor;

/// The certificate chain and private key a TLS endpoint is served with, already
/// read from the files the CLI named.
///
/// The bytes are held rather than the paths so the endpoint cannot be asked to
/// re-read a file that changed underneath it mid-connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsMaterial {
    pub certificate_chain_pem: Vec<u8>,
    pub private_key_pem: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum TlsError {
    #[error("the certificate chain holds no CERTIFICATE block")]
    NoCertificate,
    #[error("the private key holds no PRIVATE KEY block")]
    NoPrivateKey,
    #[error("a PEM block is not base64: {0}")]
    MalformedBlock(String),
    #[error("rustls refused the certificate and key: {0}")]
    Refused(String),
}

/// Every DER block of one label in a PEM document.
pub fn pem_blocks(document: &[u8], label: &str) -> Result<Vec<Vec<u8>>, TlsError> {
    let text = String::from_utf8_lossy(document);
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let mut blocks = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find(&begin) {
        let body = &rest[start + begin.len()..];
        let Some(stop) = body.find(&end) else { break };
        let encoded = body[..stop]
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        let decoded = STANDARD
            .decode(encoded)
            .map_err(|error| TlsError::MalformedBlock(error.to_string()))?;
        blocks.push(decoded);
        rest = &body[stop + end.len()..];
    }
    Ok(blocks)
}

/// Builds the acceptor one management listener terminates with.
pub fn tls_acceptor(material: &TlsMaterial) -> Result<TlsAcceptor, TlsError> {
    // The same explicit ring provider the HTTP clients install, for the same
    // reason: no aws-lc C toolchain in a headless server build. Losing the
    // one-time race is harmless because every installer installs ring.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let certificates = pem_blocks(&material.certificate_chain_pem, "CERTIFICATE")?;
    if certificates.is_empty() {
        return Err(TlsError::NoCertificate);
    }
    let certificates = certificates
        .into_iter()
        .map(CertificateDer::from)
        .collect::<Vec<_>>();

    let private_key = if let Some(key) = pem_blocks(&material.private_key_pem, "PRIVATE KEY")?.pop()
    {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key))
    } else if let Some(key) = pem_blocks(&material.private_key_pem, "RSA PRIVATE KEY")?.pop() {
        PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(key))
    } else if let Some(key) = pem_blocks(&material.private_key_pem, "EC PRIVATE KEY")?.pop() {
        PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(key))
    } else {
        return Err(TlsError::NoPrivateKey);
    };

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .map_err(|error| TlsError::Refused(error.to_string()))?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}
