mod chat_session;
mod chunk_batch;
mod connection;
mod driver;
mod event_stream;
mod exports;
mod login;
mod play_bundle;
mod probe;
mod resource_pack;
mod services_profile_keys;
mod status;
#[cfg(test)]
mod test_support;
mod types;

pub use exports::*;

fn install_http_crypto_provider() {
    // reqwest 0.13's provider-free rustls feature lets BBB keep a portable,
    // explicit provider instead of inheriting the aws-lc C toolchain. Losing
    // this one-time process race is harmless: every caller installs the same
    // ring provider before constructing an HTTP client.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[cfg(test)]
mod tests {
    #[test]
    fn http_clients_use_the_explicit_ring_provider() {
        super::install_http_crypto_provider();
        let provider = rustls::crypto::CryptoProvider::get_default()
            .expect("BBB must install a rustls provider before constructing clients");
        let ring = rustls::crypto::ring::default_provider();
        assert_eq!(provider.cipher_suites, ring.cipher_suites);
        assert!(reqwest::Client::builder().build().is_ok());
        assert!(reqwest::blocking::Client::builder().build().is_ok());
    }
}
