//! Shared X.509 SPKI RSA-SHA1 (PKCS#1 v1.5) signature verification with
//! bounded parsed-key caching for Minecraft Services public keys.

use std::sync::Arc;

use num_bigint::BigUint;
use sha1::{Digest, Sha1};

use crate::spin_lock::SpinLock;

const MAX_SPKI_DER_BYTES: usize = 2048;
const MIN_RSA_MODULUS_BITS: usize = 512;
const MAX_RSA_MODULUS_BITS: usize = 8192;
const MAX_RSA_EXPONENT_BITS: usize = 64;
const MAX_CACHED_KEYS: usize = 64;

/// X.509 `AlgorithmIdentifier` for `rsaEncryption` (`1.2.840.113549.1.1.1`) with `NULL` parameters.
const RSA_ENCRYPTION_ALGO_ID: [u8; 15] = [
    0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05, 0x00,
];

/// RFC 8017 Section 9.2 DigestInfo prefix for SHA-1 (`1.3.14.3.2.26`).
const SHA1_DIGEST_INFO_PREFIX: [u8; 15] = [
    0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14,
];

/// Parsed X.509 SPKI RSA public key used to verify Minecraft Services signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicesRsaPublicKey {
    modulus: BigUint,
    exponent: BigUint,
    modulus_bits: usize,
    modulus_bytes: usize,
}

impl ServicesRsaPublicKey {
    /// Parses a canonical X.509 SubjectPublicKeyInfo DER RSA public key.
    pub fn from_spki_der(der: &[u8]) -> Option<Self> {
        if der.is_empty() || der.len() > MAX_SPKI_DER_BYTES {
            return None;
        }
        let (spki_body, trailing) = parse_der_tlv(der, 0x30)?;
        if !trailing.is_empty() {
            return None;
        }
        let rest = spki_body.strip_prefix(&RSA_ENCRYPTION_ALGO_ID)?;
        let (bit_string, rest) = parse_der_tlv(rest, 0x03)?;
        if !rest.is_empty() {
            return None;
        }
        // First byte of BIT STRING is unused-bit count; must be zero for DER-encoded RSAPublicKey.
        let (&unused_bits, key_der) = bit_string.split_first()?;
        if unused_bits != 0 {
            return None;
        }
        let (rsa_seq, trailing) = parse_der_tlv(key_der, 0x30)?;
        if !trailing.is_empty() {
            return None;
        }
        let (n_bytes, rest) = parse_der_positive_integer(rsa_seq)?;
        let (e_bytes, rest) = parse_der_positive_integer(rest)?;
        if !rest.is_empty() {
            return None;
        }
        let modulus = BigUint::from_bytes_be(n_bytes);
        let exponent = BigUint::from_bytes_be(e_bytes);
        let modulus_bits = usize::try_from(modulus.bits()).ok()?;
        let exponent_bits = usize::try_from(exponent.bits()).ok()?;
        if !(MIN_RSA_MODULUS_BITS..=MAX_RSA_MODULUS_BITS).contains(&modulus_bits) || !modulus.bit(0)
        {
            return None;
        }
        if exponent_bits > MAX_RSA_EXPONENT_BITS
            || !exponent.bit(0)
            || exponent < BigUint::from(3u32)
            || exponent >= modulus
        {
            return None;
        }
        let modulus_bytes = modulus_bits.div_ceil(8);
        Some(Self {
            modulus,
            exponent,
            modulus_bits,
            modulus_bytes,
        })
    }

    pub fn modulus_bits(&self) -> usize {
        self.modulus_bits
    }

    /// Verifies an RSASSA-PKCS1-v1_5 SHA-1 signature over `payload`.
    pub fn verify_sha1_pkcs1v15(&self, payload: &[u8], signature: &[u8]) -> bool {
        let digest: [u8; 20] = Sha1::digest(payload).into();
        self.verify_sha1_pkcs1v15_digest(&digest, signature)
    }

    /// Verifies an RSASSA-PKCS1-v1_5 SHA-1 signature against a precomputed SHA-1 digest.
    pub fn verify_sha1_pkcs1v15_digest(&self, digest: &[u8; 20], signature: &[u8]) -> bool {
        let k = self.modulus_bytes;
        let t_len = SHA1_DIGEST_INFO_PREFIX.len() + digest.len();
        if signature.len() != k || k < t_len + 11 {
            return false;
        }
        let sig = BigUint::from_bytes_be(signature);
        if sig >= self.modulus {
            return false;
        }
        let encoded_raw = sig.modpow(&self.exponent, &self.modulus).to_bytes_be();
        if encoded_raw.len() > k {
            return false;
        }
        let mut em = vec![0u8; k];
        em[k - encoded_raw.len()..].copy_from_slice(&encoded_raw);

        let ps_len = k - t_len - 3;
        let mut diff = em[0] | (em[1] ^ 0x01);
        for &byte in &em[2..2 + ps_len] {
            diff |= byte ^ 0xff;
        }
        diff |= em[2 + ps_len];
        let digest_info_start = 3 + ps_len;
        for (&actual, &expected) in em[digest_info_start..digest_info_start + 15]
            .iter()
            .zip(&SHA1_DIGEST_INFO_PREFIX)
        {
            diff |= actual ^ expected;
        }
        for (&actual, &expected) in em[digest_info_start + 15..k].iter().zip(digest) {
            diff |= actual ^ expected;
        }
        diff == 0
    }
}

struct CachedKeyEntry {
    der: Vec<u8>,
    key: Arc<ServicesRsaPublicKey>,
}

static PARSED_KEY_CACHE: SpinLock<Vec<CachedKeyEntry>> = SpinLock::new(Vec::new());

fn with_key_cache<R>(f: impl FnOnce(&mut Vec<CachedKeyEntry>) -> R) -> R {
    PARSED_KEY_CACHE.with(f)
}

/// Returns a cached parsed [`ServicesRsaPublicKey`] for `der`, parsing it once on miss.
pub fn parse_cached_services_rsa_public_key(der: &[u8]) -> Option<Arc<ServicesRsaPublicKey>> {
    if der.is_empty() || der.len() > MAX_SPKI_DER_BYTES {
        return None;
    }
    if let Some(cached) = with_key_cache(|entries| {
        entries
            .iter()
            .find(|entry| entry.der.as_slice() == der)
            .map(|entry| Arc::clone(&entry.key))
    }) {
        return Some(cached);
    }
    let parsed = Arc::new(ServicesRsaPublicKey::from_spki_der(der)?);
    let cached = with_key_cache(|entries| {
        if let Some(existing) = entries.iter().find(|entry| entry.der.as_slice() == der) {
            return Arc::clone(&existing.key);
        }
        if entries.len() >= MAX_CACHED_KEYS {
            entries.remove(0);
        }
        entries.push(CachedKeyEntry {
            der: der.to_vec(),
            key: Arc::clone(&parsed),
        });
        parsed
    });
    Some(cached)
}

/// Verifies an RSA-SHA1 signature against any X.509 SPKI DER key in `services_public_keys`,
/// reusing cached parsed public keys across checks.
pub fn verify_services_rsa_sha1_signature(
    services_public_keys: &[Vec<u8>],
    payload: &[u8],
    signature: &[u8],
) -> bool {
    if services_public_keys.is_empty() {
        return false;
    }
    let digest: [u8; 20] = Sha1::digest(payload).into();
    services_public_keys.iter().any(|der| {
        parse_cached_services_rsa_public_key(der)
            .is_some_and(|key| key.verify_sha1_pkcs1v15_digest(&digest, signature))
    })
}

fn parse_der_tlv(input: &[u8], expected_tag: u8) -> Option<(&[u8], &[u8])> {
    let (&tag, rest) = input.split_first()?;
    if tag != expected_tag {
        return None;
    }
    let (&len_byte, rest) = rest.split_first()?;
    let (len, rest) = if len_byte < 0x80 {
        (usize::from(len_byte), rest)
    } else {
        let count = usize::from(len_byte & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let (len_bytes, tail) = rest.split_at(count);
        if len_bytes[0] == 0 {
            return None;
        }
        let mut len = 0usize;
        for &b in len_bytes {
            len = (len << 8) | usize::from(b);
        }
        if len < 0x80 {
            return None;
        }
        (len, tail)
    };
    if rest.len() < len {
        return None;
    }
    Some(rest.split_at(len))
}

fn parse_der_positive_integer(input: &[u8]) -> Option<(&[u8], &[u8])> {
    let (bytes, rest) = parse_der_tlv(input, 0x02)?;
    if bytes.is_empty() || (bytes[0] & 0x80) != 0 {
        return None;
    }
    if bytes[0] == 0x00 {
        if bytes.len() < 2 || (bytes[1] & 0x80) == 0 {
            return None;
        }
        Some((&bytes[1..], rest))
    } else {
        Some((bytes, rest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_der_len(len: usize, out: &mut Vec<u8>) {
        if len < 0x80 {
            out.push(len as u8);
        } else if len <= 0xff {
            out.push(0x81);
            out.push(len as u8);
        } else {
            out.push(0x82);
            out.push((len >> 8) as u8);
            out.push((len & 0xff) as u8);
        }
    }

    fn encode_der_tlv(tag: u8, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(body.len() + 4);
        out.push(tag);
        encode_der_len(body.len(), &mut out);
        out.extend_from_slice(body);
        out
    }

    fn encode_der_int(bytes: &[u8]) -> Vec<u8> {
        let stripped = bytes
            .iter()
            .position(|&b| b != 0)
            .map_or(&[0u8][..], |i| &bytes[i..]);
        let mut body = Vec::with_capacity(stripped.len() + 1);
        if stripped[0] & 0x80 != 0 {
            body.push(0);
        }
        body.extend_from_slice(stripped);
        encode_der_tlv(0x02, &body)
    }

    fn build_spki_der(n: &BigUint, e: &BigUint) -> Vec<u8> {
        let mut rsa_seq = Vec::new();
        rsa_seq.extend_from_slice(&encode_der_int(&n.to_bytes_be()));
        rsa_seq.extend_from_slice(&encode_der_int(&e.to_bytes_be()));
        let rsa_pub = encode_der_tlv(0x30, &rsa_seq);
        let mut bit_string = Vec::with_capacity(rsa_pub.len() + 1);
        bit_string.push(0);
        bit_string.extend_from_slice(&rsa_pub);
        let mut spki_body = Vec::new();
        spki_body.extend_from_slice(&RSA_ENCRYPTION_ALGO_ID);
        spki_body.extend_from_slice(&encode_der_tlv(0x03, &bit_string));
        encode_der_tlv(0x30, &spki_body)
    }

    #[test]
    fn rejects_malformed_and_undersized_spki_der() {
        assert!(ServicesRsaPublicKey::from_spki_der(&[]).is_none());
        let small_n = BigUint::from(65537u32) * BigUint::from(65539u32);
        let e = BigUint::from(65537u32);
        let small_der = build_spki_der(&small_n, &e);
        assert!(ServicesRsaPublicKey::from_spki_der(&small_der).is_none());

        // Trailing bytes after valid SPKI must be rejected.
        let valid_n = (BigUint::from(1u32) << 1023) + BigUint::from(1u32);
        let mut valid_der = build_spki_der(&valid_n, &e);
        let parsed = ServicesRsaPublicKey::from_spki_der(&valid_der).unwrap();
        assert_eq!(parsed.modulus_bits(), 1024);

        let cached_a = parse_cached_services_rsa_public_key(&valid_der).unwrap();
        let cached_b = parse_cached_services_rsa_public_key(&valid_der).unwrap();
        assert!(Arc::ptr_eq(&cached_a, &cached_b));

        valid_der.push(0);
        assert!(ServicesRsaPublicKey::from_spki_der(&valid_der).is_none());
    }
}
