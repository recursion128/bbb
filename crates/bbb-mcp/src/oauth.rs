//! OAuth 2.1 protected-resource validation for a remotely reachable endpoint.
//!
//! Vanilla has no equivalent: its management server checks one shared API key
//! and is expected to sit on a trusted network. A bbb endpoint that an agent
//! reaches over the network is a protected resource in the OAuth 2.1 sense, so
//! it validates access tokens itself rather than trusting whoever forwarded
//! them.
//!
//! The keys are supplied as a JWKS document from disk rather than fetched from
//! the issuer. A dedicated server is offline-only by product decision, and an
//! endpoint that reaches out to an authorization server on every unknown key id
//! would be a network dependency in the middle of an authorization check.

use std::sync::{Arc, Mutex, PoisonError};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rsa::{traits::PublicKeyParts, BigUint, Pkcs1v15Sign, RsaPublicKey};
use serde_json::Value;
use sha2::{Digest, Sha256, Sha384, Sha512};
use thiserror::Error;

use crate::{
    auth::{AuthFailure, Principal},
    host::{ScopeLadder, ScopeOf},
};

/// The smallest RSA modulus this server will verify with.
pub const MIN_MODULUS_BITS: usize = 2048;

/// The largest RSA modulus this server will accept from a JWKS or token key.
pub const MAX_MODULUS_BITS: usize = 4096;

const MAX_MODULUS_BYTES: usize = MAX_MODULUS_BITS / 8;
const MAX_RSA_KEY_CACHE_ENTRIES: usize = 64;
/// Upper bound on RSA keys attempted per token to limit verification CPU work
/// when a token omits `kid` or shares a `kid` across multiple JWKS entries.
pub const MAX_CANDIDATE_KEYS_PER_TOKEN: usize = 4;

struct CachedRsaKey {
    modulus: Vec<u8>,
    exponent: Vec<u8>,
    public_key: Arc<RsaPublicKey>,
}

static RSA_PUBLIC_KEY_CACHE: Mutex<Vec<CachedRsaKey>> = Mutex::new(Vec::new());

/// How much clock skew is tolerated on `exp` and `nbf`, in seconds.
pub const DEFAULT_LEEWAY_SECONDS: i64 = 60;

/// The scope values that grant each rung of the ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthScopeNames {
    pub read: String,
    pub manage: String,
    pub command: String,
}

impl Default for OAuthScopeNames {
    fn default() -> Self {
        Self {
            read: "bbb:read".to_string(),
            manage: "bbb:manage".to_string(),
            command: "bbb:command".to_string(),
        }
    }
}

impl OAuthScopeNames {
    pub fn all(&self) -> [&str; 3] {
        [&self.read, &self.manage, &self.command]
    }

    /// The highest rung a token's scope values grant.
    fn granted<L: ScopeLadder>(&self, scopes: &[String]) -> Option<ScopeOf<L>> {
        let mut granted = None;
        for scope in scopes {
            let rung = if *scope == self.command {
                L::COMMAND
            } else if *scope == self.manage {
                L::MANAGE
            } else if *scope == self.read {
                L::READ
            } else {
                continue;
            };
            granted = Some(granted.map_or(rung, |current: ScopeOf<L>| current.max(rung)));
        }
        granted
    }
}

/// One RSA verification key from a JWKS document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonWebKey {
    pub key_id: Option<String>,
    pub algorithm: Option<String>,
    pub modulus: Vec<u8>,
    pub exponent: Vec<u8>,
}

/// One authorization server whose access tokens are accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthIssuer {
    /// The `iss` claim value, which is also the authorization server URL the
    /// protected-resource metadata advertises.
    pub issuer: String,
    pub keys: Vec<JsonWebKey>,
}

/// This endpoint as an OAuth 2.1 protected resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthResource {
    /// The canonical resource identifier (RFC 8707). An access token has to
    /// name it in `aud`, which is what stops a token minted for another service
    /// from being replayed here.
    pub resource: String,
    pub issuers: Vec<OAuthIssuer>,
    pub scopes: OAuthScopeNames,
    pub leeway_seconds: i64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OAuthError {
    #[error("the JWKS document is not JSON: {0}")]
    MalformedDocument(String),
    #[error("the JWKS document declares no usable RSA signing key")]
    NoUsableKey,
    #[error("a JWKS key is malformed: {0}")]
    MalformedKey(String),
}

/// Reads the RSA signing keys out of a JWKS document.
///
/// Keys of other types, keys marked for encryption and keys below
/// [`MIN_MODULUS_BITS`] are skipped rather than refused: a real authorization
/// server publishes keys this server has no use for, and dropping them is not
/// an error. A document with none left is.
pub fn parse_jwks(document: &[u8]) -> Result<Vec<JsonWebKey>, OAuthError> {
    let document: Value = serde_json::from_slice(document)
        .map_err(|error| OAuthError::MalformedDocument(error.to_string()))?;
    let entries = document
        .get("keys")
        .and_then(Value::as_array)
        .ok_or_else(|| OAuthError::MalformedDocument("no keys array".to_string()))?;
    let mut keys = Vec::new();
    for entry in entries {
        if entry.get("kty").and_then(Value::as_str) != Some("RSA") {
            continue;
        }
        if matches!(entry.get("use").and_then(Value::as_str), Some(purpose) if purpose != "sig") {
            continue;
        }
        let (Some(modulus), Some(exponent)) = (
            entry.get("n").and_then(Value::as_str),
            entry.get("e").and_then(Value::as_str),
        ) else {
            return Err(OAuthError::MalformedKey(
                "an RSA key needs n and e".to_string(),
            ));
        };
        let modulus = URL_SAFE_NO_PAD
            .decode(modulus)
            .map_err(|error| OAuthError::MalformedKey(format!("n: {error}")))?;
        let exponent = URL_SAFE_NO_PAD
            .decode(exponent)
            .map_err(|error| OAuthError::MalformedKey(format!("e: {error}")))?;
        if modulus.len() * 8 > MAX_MODULUS_BITS {
            return Err(OAuthError::MalformedKey(format!(
                "RSA modulus exceeds {MAX_MODULUS_BITS} bits"
            )));
        }
        if exponent.is_empty() || exponent.len() > MAX_MODULUS_BYTES {
            return Err(OAuthError::MalformedKey(
                "RSA exponent length is out of bounds".to_string(),
            ));
        }
        if modulus.len() * 8 < MIN_MODULUS_BITS {
            continue;
        }
        keys.push(JsonWebKey {
            key_id: entry.get("kid").and_then(Value::as_str).map(str::to_string),
            algorithm: entry.get("alg").and_then(Value::as_str).map(str::to_string),
            modulus,
            exponent,
        });
    }
    if keys.is_empty() {
        return Err(OAuthError::NoUsableKey);
    }
    Ok(keys)
}

impl OAuthResource {
    /// The RFC 9728 metadata URL a refusal points a client at.
    pub fn metadata_url(&self) -> String {
        const WELL_KNOWN: &str = "/.well-known/oauth-protected-resource";
        let Some((scheme, rest)) = self.resource.split_once("://") else {
            return self.resource.clone();
        };
        match rest.split_once('/') {
            Some((authority, path)) => format!("{scheme}://{authority}{WELL_KNOWN}/{path}"),
            None => format!("{scheme}://{rest}{WELL_KNOWN}"),
        }
    }

    /// The RFC 9728 protected-resource metadata document.
    pub fn metadata_document(&self) -> Value {
        serde_json::json!({
            "resource": self.resource,
            "authorization_servers": self
                .issuers
                .iter()
                .map(|issuer| issuer.issuer.clone())
                .collect::<Vec<_>>(),
            "scopes_supported": self.scopes.all(),
            "bearer_methods_supported": ["header"],
        })
    }

    /// Validates one access token and answers with the principal it names.
    pub fn validate<L: ScopeLadder>(
        &self,
        token: &str,
        now_millis: i64,
    ) -> Result<Principal<L>, AuthFailure> {
        let now = now_millis.div_euclid(1_000);
        let mut parts = token.split('.');
        let (Some(header), Some(payload), Some(signature), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(AuthFailure::InvalidCredential);
        };
        let header = decode_json(header)?;
        let claims = decode_json(payload)?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| AuthFailure::InvalidCredential)?;
        let algorithm = header
            .get("alg")
            .and_then(Value::as_str)
            .ok_or(AuthFailure::InvalidCredential)?;

        let issuer_claim = claims
            .get("iss")
            .and_then(Value::as_str)
            .ok_or(AuthFailure::InvalidCredential)?;
        let issuer = self
            .issuers
            .iter()
            .find(|issuer| issuer.issuer == issuer_claim)
            .ok_or(AuthFailure::InvalidCredential)?;

        // The signature is checked before any claim is believed, because an
        // unverified claim is only an assertion by whoever sent the request.
        let key_id = header.get("kid").and_then(Value::as_str);
        let (signing_input, _) = token
            .rsplit_once('.')
            .ok_or(AuthFailure::InvalidCredential)?;
        let verified = issuer
            .keys
            .iter()
            .filter(|key| match (key_id, key.key_id.as_deref()) {
                (Some(wanted), Some(available)) => wanted == available,
                // A token without a key id is tried against a bounded number of
                // keys the issuer published, which is what a rotating issuer requires.
                _ => true,
            })
            .filter(|key| match key.algorithm.as_deref() {
                Some(declared) => declared == algorithm,
                None => true,
            })
            .take(MAX_CANDIDATE_KEYS_PER_TOKEN)
            .any(|key| verify(key, algorithm, signing_input.as_bytes(), &signature));
        if !verified {
            return Err(AuthFailure::InvalidCredential);
        }

        let expiry = claims
            .get("exp")
            .and_then(|value| numeric_date_claim(value, false))
            .ok_or(AuthFailure::InvalidCredential)?;
        if expiry.saturating_add(self.leeway_seconds) < now {
            return Err(AuthFailure::InvalidCredential);
        }
        if let Some(raw_nbf) = claims.get("nbf") {
            let not_before =
                numeric_date_claim(raw_nbf, true).ok_or(AuthFailure::InvalidCredential)?;
            if not_before.saturating_sub(self.leeway_seconds) > now {
                return Err(AuthFailure::InvalidCredential);
            }
        }
        if !audience_names(&claims, &self.resource) {
            return Err(AuthFailure::InvalidCredential);
        }

        let scopes = token_scopes(&claims);
        let scope = self
            .scopes
            .granted::<L>(&scopes)
            .ok_or_else(|| AuthFailure::Forbidden("this token grants no bbb scope".to_string()))?;
        let subject = claims
            .get("sub")
            .or_else(|| claims.get("client_id"))
            .and_then(Value::as_str)
            .ok_or(AuthFailure::InvalidCredential)?;
        Ok(Principal {
            // The issuer travels with the subject so two authorization servers
            // that both issue `admin` are two principals in the audit log.
            name: format!("{issuer_claim}#{subject}"),
            scope,
        })
    }
}

fn numeric_date_claim(value: &Value, round_up: bool) -> Option<i64> {
    if let Some(seconds) = value.as_i64() {
        return Some(seconds);
    }
    let seconds = value.as_f64()?;
    if !seconds.is_finite() {
        return None;
    }
    let rounded = if round_up {
        seconds.ceil()
    } else {
        seconds.floor()
    };
    if rounded < (i64::MIN as f64) || rounded >= (i64::MAX as f64) {
        return None;
    }
    Some(rounded as i64)
}

fn decode_json(part: &str) -> Result<Value, AuthFailure> {
    let bytes = URL_SAFE_NO_PAD
        .decode(part)
        .map_err(|_| AuthFailure::InvalidCredential)?;
    serde_json::from_slice(&bytes).map_err(|_| AuthFailure::InvalidCredential)
}

fn cached_public_key(key: &JsonWebKey) -> Option<Arc<RsaPublicKey>> {
    let modulus_bits = key.modulus.len().saturating_mul(8);
    if !(MIN_MODULUS_BITS..=MAX_MODULUS_BITS).contains(&modulus_bits)
        || key.exponent.is_empty()
        || key.exponent.len() > MAX_MODULUS_BYTES
    {
        return None;
    }

    {
        let mut cache = RSA_PUBLIC_KEY_CACHE
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = cache
            .iter()
            .position(|entry| entry.modulus == key.modulus && entry.exponent == key.exponent)
        {
            let entry = cache.remove(index);
            let public_key = Arc::clone(&entry.public_key);
            cache.push(entry);
            return Some(public_key);
        }
    }

    let public_key = RsaPublicKey::new(
        BigUint::from_bytes_be(&key.modulus),
        BigUint::from_bytes_be(&key.exponent),
    )
    .ok()?;
    let actual_bits = public_key.n().bits();
    if !(MIN_MODULUS_BITS..=MAX_MODULUS_BITS).contains(&actual_bits) {
        return None;
    }
    let public_key = Arc::new(public_key);

    let mut cache = RSA_PUBLIC_KEY_CACHE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = cache
        .iter()
        .find(|entry| entry.modulus == key.modulus && entry.exponent == key.exponent)
    {
        return Some(Arc::clone(&existing.public_key));
    }
    if cache.len() >= MAX_RSA_KEY_CACHE_ENTRIES {
        cache.remove(0);
    }
    cache.push(CachedRsaKey {
        modulus: key.modulus.clone(),
        exponent: key.exponent.clone(),
        public_key: Arc::clone(&public_key),
    });
    Some(public_key)
}

#[cfg(test)]
pub(crate) fn cached_public_key_for_test(key: &JsonWebKey) -> Option<Arc<RsaPublicKey>> {
    cached_public_key(key)
}

fn verify(key: &JsonWebKey, algorithm: &str, signing_input: &[u8], signature: &[u8]) -> bool {
    let Some(public_key) = cached_public_key(key) else {
        return false;
    };
    match algorithm {
        "RS256" => public_key
            .verify(
                Pkcs1v15Sign::new::<Sha256>(),
                &Sha256::digest(signing_input),
                signature,
            )
            .is_ok(),
        "RS384" => public_key
            .verify(
                Pkcs1v15Sign::new::<Sha384>(),
                &Sha384::digest(signing_input),
                signature,
            )
            .is_ok(),
        "RS512" => public_key
            .verify(
                Pkcs1v15Sign::new::<Sha512>(),
                &Sha512::digest(signing_input),
                signature,
            )
            .is_ok(),
        // Every other `alg` value, `none` most of all, is not a signature this
        // server knows how to check, so it is not one it accepts.
        _ => false,
    }
}

/// `aud` is a string or an array of strings, and one of them has to be this
/// resource.
fn audience_names(claims: &Value, resource: &str) -> bool {
    match claims.get("aud") {
        Some(Value::String(audience)) => audience == resource,
        Some(Value::Array(audiences)) => audiences
            .iter()
            .filter_map(Value::as_str)
            .any(|audience| audience == resource),
        _ => false,
    }
}

/// `scope` is the space-delimited OAuth form; `scp` is the array form some
/// issuers mint instead.
fn token_scopes(claims: &Value) -> Vec<String> {
    if let Some(scope) = claims.get("scope").and_then(Value::as_str) {
        return scope.split_whitespace().map(str::to_string).collect();
    }
    match claims.get("scp") {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        Some(Value::String(scope)) => scope.split_whitespace().map(str::to_string).collect(),
        _ => Vec::new(),
    }
}
