use std::{
    net::SocketAddr,
    sync::{Arc, OnceLock},
    time::Duration,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::{pkcs1v15::Pkcs1v15Sign, traits::PublicKeyParts, RsaPrivateKey, RsaPublicKey};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    auth::{authenticate, challenge, AuthFailure},
    host::Scope,
    http::HeaderMap,
    oauth::{parse_jwks, OAuthIssuer, OAuthResource, OAuthScopeNames, DEFAULT_LEEWAY_SECONDS},
    server::{check_bind, BindRefusal, RemoteAuthority},
    tls::{pem_blocks, tls_acceptor, TlsMaterial},
};

type AuthConfig = crate::AuthConfig<Scope>;

const ISSUER: &str = "https://login.example/realms/bbb";
const RESOURCE: &str = "https://server.example/mcp";
const NOW: i64 = 1_755_000_000_000;

struct TestRng(rand::rngs::StdRng);

impl rsa::rand_core::RngCore for TestRng {
    fn next_u32(&mut self) -> u32 {
        rand::Rng::next_u32(&mut self.0)
    }

    fn next_u64(&mut self) -> u64 {
        rand::Rng::next_u64(&mut self.0)
    }

    fn fill_bytes(&mut self, destination: &mut [u8]) {
        rand::Rng::fill_bytes(&mut self.0, destination);
    }

    fn try_fill_bytes(&mut self, destination: &mut [u8]) -> Result<(), rsa::rand_core::Error> {
        self.fill_bytes(destination);
        Ok(())
    }
}

impl rsa::rand_core::CryptoRng for TestRng {}

/// One issuer key pair, generated once because 2048-bit generation is the
/// slowest thing in this module by an order of magnitude.
fn issuer_key() -> &'static RsaPrivateKey {
    static KEY: OnceLock<RsaPrivateKey> = OnceLock::new();
    KEY.get_or_init(|| {
        use rand::SeedableRng;
        let mut rng = TestRng(rand::rngs::StdRng::seed_from_u64(20260817));
        RsaPrivateKey::new(&mut rng, 2048).expect("an issuer key")
    })
}

fn jwks_document(key_id: &str) -> Vec<u8> {
    let public = RsaPublicKey::from(issuer_key());
    json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": key_id,
            "n": URL_SAFE_NO_PAD.encode(public.n().to_bytes_be()),
            "e": URL_SAFE_NO_PAD.encode(public.e().to_bytes_be()),
        }],
    })
    .to_string()
    .into_bytes()
}

fn resource() -> OAuthResource {
    OAuthResource {
        resource: RESOURCE.to_string(),
        issuers: vec![OAuthIssuer {
            issuer: ISSUER.to_string(),
            keys: parse_jwks(&jwks_document("k-1")).expect("a jwks"),
        }],
        scopes: OAuthScopeNames::default(),
        leeway_seconds: DEFAULT_LEEWAY_SECONDS,
    }
}

fn token(header: Value, claims: Value, sign: bool) -> String {
    let header = URL_SAFE_NO_PAD.encode(header.to_string());
    let claims = URL_SAFE_NO_PAD.encode(claims.to_string());
    let signing_input = format!("{header}.{claims}");
    let signature = if sign {
        issuer_key()
            .sign(
                Pkcs1v15Sign::new::<Sha256>(),
                &Sha256::digest(signing_input.as_bytes()),
            )
            .expect("a signature")
    } else {
        vec![0u8; 256]
    };
    format!("{signing_input}.{}", URL_SAFE_NO_PAD.encode(signature))
}

fn access_token(claims: Value) -> String {
    token(
        json!({ "alg": "RS256", "typ": "at+jwt", "kid": "k-1" }),
        claims,
        true,
    )
}

fn claims(scope: &str) -> Value {
    json!({
        "iss": ISSUER,
        "sub": "release-agent",
        "aud": RESOURCE,
        "exp": NOW / 1_000 + 300,
        "nbf": NOW / 1_000 - 10,
        "scope": scope,
    })
}

#[test]
fn a_valid_access_token_names_its_issuer_subject_and_the_scope_it_grants() {
    let resource = resource();
    let principal = resource
        .validate::<Scope>(&access_token(claims("openid bbb:read bbb:manage")), NOW)
        .expect("the token is accepted");
    assert_eq!(principal.name, format!("{ISSUER}#release-agent"));
    assert_eq!(principal.scope, Scope::Manage);

    let command = resource
        .validate::<Scope>(&access_token(claims("bbb:command")), NOW)
        .expect("the token is accepted");
    assert_eq!(command.scope, Scope::Command);
}

#[test]
fn the_array_scope_form_is_read_as_well() {
    let mut claims = claims("");
    claims["scope"] = Value::Null;
    claims["scp"] = json!(["bbb:read"]);
    let principal = resource()
        .validate::<Scope>(&access_token(claims), NOW)
        .expect("the token is accepted");
    assert_eq!(principal.scope, Scope::Read);
}

#[test]
fn every_reason_a_token_is_not_this_endpoints_is_a_refusal() {
    let resource = resource();
    let expired = {
        let mut claims = claims("bbb:read");
        claims["exp"] = json!(NOW / 1_000 - 3_600);
        claims
    };
    let future = {
        let mut claims = claims("bbb:read");
        claims["nbf"] = json!(NOW / 1_000 + 3_600);
        claims
    };
    let elsewhere = {
        let mut claims = claims("bbb:read");
        claims["aud"] = json!("https://other.example/mcp");
        claims
    };
    let unknown_issuer = {
        let mut claims = claims("bbb:read");
        claims["iss"] = json!("https://attacker.example");
        claims
    };
    let no_expiry = {
        let mut claims = claims("bbb:read");
        claims["exp"] = Value::Null;
        claims
    };
    for (name, claims) in [
        ("expired", expired),
        ("not yet valid", future),
        ("another audience", elsewhere),
        ("an unknown issuer", unknown_issuer),
        ("no expiry", no_expiry),
    ] {
        assert_eq!(
            resource.validate::<Scope>(&access_token(claims), NOW),
            Err(AuthFailure::InvalidCredential),
            "{name}"
        );
    }

    // An unsigned token, a token signed by nobody, and the `none` algorithm
    // are all refused before any claim is believed.
    assert_eq!(
        resource.validate::<Scope>(
            &token(
                json!({ "alg": "RS256", "kid": "k-1" }),
                claims("bbb:read"),
                false
            ),
            NOW
        ),
        Err(AuthFailure::InvalidCredential)
    );
    assert_eq!(
        resource.validate::<Scope>(
            &token(json!({ "alg": "none" }), claims("bbb:read"), false),
            NOW
        ),
        Err(AuthFailure::InvalidCredential)
    );
    assert_eq!(
        resource.validate::<Scope>("not.a.token", NOW),
        Err(AuthFailure::InvalidCredential)
    );

    // A token that authenticates but grants no bbb scope is forbidden, not
    // unauthenticated: the caller is who they say, and may not do this.
    assert!(matches!(
        resource.validate::<Scope>(&access_token(claims("openid profile")), NOW),
        Err(AuthFailure::Forbidden(_))
    ));
}

#[test]
fn a_key_the_issuer_did_not_publish_verifies_nothing() {
    let mut resource = resource();
    resource.issuers[0].keys[0].key_id = Some("k-2".to_string());
    assert_eq!(
        resource.validate::<Scope>(&access_token(claims("bbb:read")), NOW),
        Err(AuthFailure::InvalidCredential)
    );
}

#[test]
fn a_jwks_document_keeps_only_the_keys_this_server_can_verify_with() {
    let document = json!({
        "keys": [
            { "kty": "EC", "crv": "P-256", "x": "AA", "y": "BB" },
            { "kty": "RSA", "use": "enc", "n": "AA", "e": "AQAB" },
            { "kty": "RSA", "n": URL_SAFE_NO_PAD.encode([1u8; 128]), "e": "AQAB" },
        ],
    })
    .to_string();
    // The elliptic key is not RSA, the encryption key is not for signing,
    // and the 1024-bit key is below the floor.
    assert!(parse_jwks(document.as_bytes()).is_err());
    assert_eq!(parse_jwks(&jwks_document("k-1")).expect("keys").len(), 1);
    assert!(parse_jwks(b"{}").is_err());
    assert!(parse_jwks(b"not json").is_err());

    // RSA moduli above 4096 bits are rejected outright in both JWKS parsing and validation.
    let oversized = json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": "too-big",
            "n": URL_SAFE_NO_PAD.encode([0x80u8; 513]),
            "e": URL_SAFE_NO_PAD.encode([0x01, 0x00, 0x01]),
        }],
    })
    .to_string();
    assert!(matches!(
        parse_jwks(oversized.as_bytes()),
        Err(crate::OAuthError::MalformedKey(_))
    ));

    let mut oversized_resource = resource();
    oversized_resource.issuers[0].keys[0].modulus = vec![0x80u8; 513];
    assert_eq!(
        oversized_resource.validate::<Scope>(&access_token(claims("bbb:read")), NOW),
        Err(AuthFailure::InvalidCredential)
    );
}

#[test]
fn rsa_public_keys_are_cached_across_token_validations() {
    let resource = resource();
    let jwk = &resource.issuers[0].keys[0];
    let token = access_token(claims("bbb:read"));
    assert!(resource.validate::<Scope>(&token, NOW).is_ok());
    let first = crate::oauth::cached_public_key_for_test(jwk).expect("cached key");
    assert!(resource.validate::<Scope>(&token, NOW).is_ok());
    let second = crate::oauth::cached_public_key_for_test(jwk).expect("cached key");
    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn the_protected_resource_metadata_says_where_to_get_a_token() {
    let resource = resource();
    assert_eq!(
        resource.metadata_url(),
        "https://server.example/.well-known/oauth-protected-resource/mcp"
    );
    let document = resource.metadata_document();
    assert_eq!(document["resource"], RESOURCE);
    assert_eq!(document["authorization_servers"][0], ISSUER);
    assert_eq!(document["scopes_supported"][0], "bbb:read");
    assert_eq!(document["bearer_methods_supported"][0], "header");

    let config = AuthConfig {
        oauth: Some(resource),
        ..AuthConfig::default()
    };
    assert_eq!(
        challenge(&config, &AuthFailure::MissingCredential),
        "Bearer resource_metadata=\"https://server.example/.well-known/oauth-protected-resource/mcp\""
    );
    assert!(challenge(&config, &AuthFailure::InvalidCredential)
        .starts_with("Bearer error=\"invalid_token\","));
    assert!(
        challenge(&config, &AuthFailure::Forbidden("no scope".to_string()))
            .contains("error=\"insufficient_scope\"")
    );
}

#[test]
fn an_oauth_token_authenticates_through_the_same_door_an_issued_secret_does() {
    let config = AuthConfig {
        loopback_scope: None,
        oauth: Some(resource()),
        ..AuthConfig::default()
    };
    let mut headers = HeaderMap::default();
    headers.insert(
        "authorization",
        format!("Bearer {}", access_token(claims("bbb:read"))),
    );
    let peer: SocketAddr = "203.0.113.7:52000".parse().expect("address");
    let principal = authenticate(&config, &headers, peer, NOW).expect("accepted");
    assert_eq!(principal.scope, Scope::Read);

    let mut headers = HeaderMap::default();
    headers.insert("authorization", "Bearer not-a-token");
    assert_eq!(
        authenticate(&config, &headers, peer, NOW),
        Err(AuthFailure::InvalidCredential)
    );
}

fn tls_material() -> TlsMaterial {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/fixtures/mcp-tls");
    TlsMaterial {
        certificate_chain_pem: std::fs::read(fixtures.join("localhost.crt"))
            .expect("the test certificate"),
        private_key_pem: std::fs::read(fixtures.join("localhost.key")).expect("the test key"),
    }
}

#[test]
fn a_pem_pair_becomes_an_acceptor_and_a_broken_one_says_why() {
    let material = tls_material();
    assert_eq!(
        pem_blocks(&material.certificate_chain_pem, "CERTIFICATE")
            .expect("blocks")
            .len(),
        1
    );
    assert!(tls_acceptor(&material).is_ok());

    let no_certificate = TlsMaterial {
        certificate_chain_pem:
            b"-----BEGIN NOT A CERTIFICATE-----\nAA==\n-----END NOT A CERTIFICATE-----\n".to_vec(),
        ..material.clone()
    };
    assert!(matches!(
        tls_acceptor(&no_certificate),
        Err(crate::tls::TlsError::NoCertificate)
    ));
    let no_key = TlsMaterial {
        private_key_pem: Vec::new(),
        ..material.clone()
    };
    assert!(matches!(
        tls_acceptor(&no_key),
        Err(crate::tls::TlsError::NoPrivateKey)
    ));
    let malformed = TlsMaterial {
        certificate_chain_pem: b"-----BEGIN CERTIFICATE-----\n!!!!\n-----END CERTIFICATE-----\n"
            .to_vec(),
        ..material
    };
    assert!(matches!(
        tls_acceptor(&malformed),
        Err(crate::tls::TlsError::MalformedBlock(_))
    ));
}

#[test]
fn a_remote_bind_needs_the_authority_that_protects_it() {
    let loopback: SocketAddr = "127.0.0.1:25585".parse().expect("address");
    let remote: SocketAddr = "0.0.0.0:25585".parse().expect("address");
    let bare = AuthConfig::default();
    assert_eq!(
        check_bind(loopback, &RemoteAuthority::LoopbackOnly, &bare),
        Ok(())
    );
    assert_eq!(
        check_bind(remote, &RemoteAuthority::LoopbackOnly, &bare),
        Err(BindRefusal::RemoteWithoutAuthority(remote))
    );

    let authority = RemoteAuthority::Tls {
        material: tls_material(),
        oauth: resource(),
    };
    // Declaring TLS is not enough: the resource that validates tokens has
    // to be the one the endpoint actually authenticates against.
    assert_eq!(
        check_bind(remote, &authority, &bare),
        Err(BindRefusal::AuthorityNotWired)
    );
    let wired = AuthConfig {
        oauth: Some(resource()),
        ..AuthConfig::default()
    };
    assert_eq!(check_bind(remote, &authority, &wired), Ok(()));
}

#[tokio::test]
async fn a_tls_endpoint_serves_the_handshake_and_the_metadata_it_advertises() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let config = crate::McpServerConfig::<super::transport::TestHost> {
        auth: AuthConfig {
            loopback_scope: Some(Scope::Manage),
            oauth: Some(resource()),
            ..AuthConfig::default()
        },
        ..crate::McpServerConfig::default()
    };
    let server = crate::McpServer::new(config, super::transport::TestHost::answering());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("address");
    let acceptor = tls_acceptor(&tls_material()).expect("acceptor");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = tokio::spawn(Arc::clone(&server).serve_tls(listener, acceptor, async {
        let _ = stopped.await;
    }));

    let mut roots = rustls::RootCertStore::empty();
    let authority = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/fixtures/mcp-tls/ca.crt"),
    )
    .expect("the test certificate authority");
    for block in pem_blocks(&authority, "CERTIFICATE").expect("blocks") {
        roots
            .add(rustls::pki_types::CertificateDer::from(block))
            .expect("the fixture authority is a root for this test");
    }
    let client = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client));
    let stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect");
    let mut stream = connector
        .connect(
            rustls::pki_types::ServerName::try_from("localhost").expect("name"),
            stream,
        )
        .await
        .expect("the handshake completes");

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "protocolVersion": crate::PROTOCOL_VERSION, "capabilities": {} },
    })
    .to_string();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nhost: localhost\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.expect("write");
    let mut received = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), stream.read_to_end(&mut received))
        .await
        .expect("the answer arrives")
        .expect("read");
    let received = String::from_utf8_lossy(&received).to_string();
    assert!(received.starts_with("HTTP/1.1 200 OK"), "{received}");
    assert!(received.contains(crate::PROTOCOL_VERSION), "{received}");

    // The metadata a refusal points at is served without a credential.
    let stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect");
    let mut stream = connector
        .connect(
            rustls::pki_types::ServerName::try_from("localhost").expect("name"),
            stream,
        )
        .await
        .expect("the handshake completes");
    stream
        .write_all(
            b"GET /.well-known/oauth-protected-resource/mcp HTTP/1.1\r\nhost: localhost\r\nconnection: close\r\n\r\n",
        )
        .await
        .expect("write");
    let mut received = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), stream.read_to_end(&mut received))
        .await
        .expect("the answer arrives")
        .expect("read");
    let received = String::from_utf8_lossy(&received).to_string();
    assert!(received.starts_with("HTTP/1.1 200 OK"), "{received}");
    assert!(received.contains(ISSUER), "{received}");

    let _ = stop.send(());
    let _ = tokio::time::timeout(Duration::from_secs(5), serving).await;
}
