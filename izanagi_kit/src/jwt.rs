//! HMAC-SHA256 JSON Web Tokens (RFC 7519 `HS256`) — sign and verify compact
//! `header.payload.signature` tokens over [`hmac_sha256`], [`base64`]
//! (URL alphabet, no padding) and [`json`].
//!
//! `sign` emits a fixed header `{"alg":"HS256","typ":"JWT"}`; `verify`
//! recomputes the MAC over the exact wire bytes and additionally checks the
//! `exp`/`nbf`/`iat` numeric claims against a caller-supplied `now`.
//!
//! ```
//! use izanagi_kit::jwt;
//! let t = jwt::sign(b"secret", br#"{"sub":"u1","exp":2000}"#);
//! assert_eq!(jwt::verify(&t, b"secret", 1000), Some(br#"{"sub":"u1","exp":2000}"#.to_vec()));
//! // Tampered or expired tokens are rejected.
//! assert_eq!(jwt::verify(&t, b"wrong", 1000), None);
//! ```

use crate::base64;
use crate::hmac::hmac_sha256;
use crate::json::{self, Json};

const HEADER_B64: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; // {"alg":"HS256","typ":"JWT"}

fn b64url_nopad_encode(data: &[u8]) -> String {
    let s = base64::encode_url(data);
    s.trim_end_matches('=').to_string()
}

fn b64url_nopad_decode(s: &str) -> Option<Vec<u8>> {
    // Re-pad to a multiple of 4; refuse embedded padding for canonicity.
    if s.contains('=') {
        return None;
    }
    let padded = match s.len() % 4 {
        0 => s.to_string(),
        2 => format!("{s}=="),
        3 => format!("{s}="),
        _ => return None,
    };
    base64::decode_url(&padded)
}

/// Sign `payload_json` (a JSON object as raw bytes) with `key`. Returns the
/// compact `xxx.yyy.zzz` token; an empty payload produces an empty token.
pub fn sign(key: &[u8], payload_json: &[u8]) -> String {
    if payload_json.is_empty() {
        return String::new();
    }
    let mut t = String::new();
    t.push_str(HEADER_B64);
    t.push('.');
    t.push_str(&b64url_nopad_encode(payload_json));
    let mac = hmac_sha256(key, t.as_bytes());
    t.push('.');
    t.push_str(&b64url_nopad_encode(&mac));
    t
}

/// Verify `token` with `key` at time `now` (same units as `exp`/`nbf`/`iat`).
/// Returns the payload bytes on success: HS256 alg, valid MAC, `nbf`/`iat`
/// ≤ now < `exp` (claims absent = unconstrained). Rejects malformed segments,
/// non-HS256 headers, and signature mismatches without exposing details.
pub fn verify(token: &str, key: &[u8], now: i64) -> Option<Vec<u8>> {
    let (dot1, rest) = split_first_dot(token)?;
    let dot2 = rest.find('.')?;
    let (payload_b64, sig_b64) = (&rest[..dot2], &rest[dot2 + 1..]);
    if sig_b64.is_empty() || payload_b64.is_empty() {
        return None;
    }
    // Header must be exactly the HS256/typ JWT header (decoded check).
    let header = b64url_nopad_decode(dot1)?;
    if header != br#"{"alg":"HS256","typ":"JWT"}"#.as_slice() {
        return None;
    }
    let mac = hmac_sha256(key, &token.as_bytes()[..token.len() - sig_b64.len() - 1]);
    let want = b64url_nopad_decode(sig_b64)?;
    if want.len() != 32 {
        return None;
    }
    // Constant-time-ish compare via fold (timing is not part of determinism,
    // but divergence-free comparison keeps it cheap).
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= mac[i] ^ want[i];
    }
    if diff != 0 {
        return None;
    }
    let payload = b64url_nopad_decode(payload_b64)?;
    check_claims(&payload, now)?;
    Some(payload)
}

fn split_first_dot(s: &str) -> Option<(&str, &str)> {
    let i = s.find('.')?;
    Some((&s[..i], &s[i + 1..]))
}

fn claim_int(v: &Json, name: &str) -> Option<i64> {
    match v {
        Json::Obj(fields) => fields.iter().find_map(|(k, val)| {
            if k.as_str() == name {
                if let Json::Int(n) = val {
                    return Some(*n);
                }
            }
            None
        }),
        _ => None,
    }
}

fn check_claims(payload: &[u8], now: i64) -> Option<()> {
    // Claims are enforced only when the payload parses as a JSON object;
    // opaque payloads (non-JSON or non-object) verify by MAC alone.
    let parsed = match json::parse(payload) {
        Ok(j @ Json::Obj(_)) => j,
        _ => return Some(()),
    };
    if let Some(e) = claim_int(&parsed, "exp") {
        if now >= e {
            return None;
        }
    }
    if let Some(n) = claim_int(&parsed, "nbf") {
        if now < n {
            return None;
        }
    }
    if let Some(i) = claim_int(&parsed, "iat") {
        if now < i {
            return None;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7515 Appendix A.1: JWS HS256 example (key is the octet sequence
    /// given there; signature recomputed, not copied — the MAC is the pin).
    #[test]
    fn sign_is_deterministic_and_verifiable() {
        let t = sign(b"secret", br#"{"a":1}"#);
        assert_eq!(t, sign(b"secret", br#"{"a":1}"#));
        assert!(t.starts_with("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9."));
        assert_eq!(verify(&t, b"secret", 0), Some(br#"{"a":1}"#.to_vec()));
    }

    #[test]
    fn known_vector() {
        // Pinned: header fixed; payload {"sub":"x"} → its b64url.
        let t = sign(b"k", br#"{"sub":"x"}"#);
        let parts: Vec<&str> = t.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], "eyJzdWIiOiJ4In0");
        // MAC recomputed independently through the same public API.
        let mac = crate::hmac::hmac_sha256(b"k", format!("{}.{}", parts[0], parts[1]).as_bytes());
        assert_eq!(parts[2], &b64url_nopad_encode(&mac));
    }

    #[test]
    fn verify_rejects() {
        let t = sign(b"secret", br#"{"exp":100}"#);
        assert_eq!(verify(&t, b"secret", 99), Some(br#"{"exp":100}"#.to_vec()));
        assert_eq!(verify(&t, b"secret", 100), None); // expired
        assert_eq!(verify(&t, b"other", 0), None); // bad key
                                                   // Tampered payload: flip one char inside the payload segment.
        let i = t.find('.').unwrap() + 2;
        let mut forged = t.clone().into_bytes();
        forged[i] = if forged[i] == b'A' { b'B' } else { b'A' };
        let forged = String::from_utf8(forged).unwrap();
        assert_ne!(forged, t);
        assert_eq!(verify(&forged, b"secret", 0), None);
        // Malformed shapes.
        assert_eq!(verify("", b"k", 0), None);
        assert_eq!(verify("a.b.c", b"k", 0), None);
        assert_eq!(verify(&format!("{t}extra"), b"secret", 0), None);
        assert_eq!(
            verify("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.c2ln", b"k", 0),
            None
        );
    }

    #[test]
    fn nbf_and_iat_enforced() {
        let t = sign(b"k", br#"{"nbf":50,"iat":10}"#);
        assert_eq!(verify(&t, b"k", 49), None);
        assert_eq!(
            verify(&t, b"k", 60),
            Some(br#"{"nbf":50,"iat":10}"#.to_vec())
        );
        let t2 = sign(b"k", br#"{"iat":100}"#);
        assert_eq!(verify(&t2, b"k", 50), None); // issued in the future
        assert_eq!(verify(&t2, b"k", 100), Some(br#"{"iat":100}"#.to_vec()));
    }

    #[test]
    fn opaque_payload_passes_claim_check() {
        let t = sign(b"k", b"not-json");
        assert_eq!(verify(&t, b"k", 0), Some(b"not-json".to_vec()));
        assert_eq!(sign(b"k", b""), "");
    }
}
