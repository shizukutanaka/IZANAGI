//! `otpauth://` URIs — the Google-Authenticator/KeyURI convention:
//! `otpauth://totp/Issuer:account?secret=B32&issuer=I&algorithm=SHA1&digits=6&period=30`
//! and `otpauth://hotp/label?secret=B32&counter=42`. Secrets are
//! RFC 4648 base32 (padding optional on decode, emitted on encode).
//!
//! [`Otp::code`] computes the current value via [`crate::otp`] — only
//! `SHA1` is supported (the only algorithm the otp module implements);
//! other `algorithm` params parse but `code` returns `None` for them.
//!
//! ```
//! use izanagi_kit::otpauth::{parse, Kind};
//!
//! let o = parse("otpauth://totp/Issuer:alice?secret=JBSWY3DPEHPK3PXP&issuer=Issuer&period=30&digits=6").unwrap();
//! assert_eq!(o.kind, Kind::Totp);
//! assert_eq!(o.account, "alice");
//! ```

use crate::otp;
use crate::uri::Uri;
use std::string::String;
use std::vec::Vec;

/// HOTP or TOTP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Counter-based (`otpauth://hotp/`).
    Hotp,
    /// Time-based (`otpauth://totp/`).
    Totp,
}

/// A decoded otpauth URI.
#[derive(Clone, Debug, PartialEq)]
pub struct Otp {
    /// Which mechanism.
    pub kind: Kind,
    /// Full label text (pct-decoded).
    pub label: String,
    /// `issuer` parameter, else the label's `Issuer:` prefix, else "".
    pub issuer: String,
    /// Label part after `:` (or whole label when no colon).
    pub account: String,
    /// Base32-decoded secret bytes.
    pub secret: Vec<u8>,
    /// `algorithm` param verbatim (`SHA1` default).
    pub algorithm: String,
    /// Code length (default 6).
    pub digits: u32,
    /// TOTP period in seconds (default 30).
    pub period: u64,
    /// HOTP counter (default 0).
    pub counter: u64,
}

impl Otp {
    /// Current code. `now_secs` is unix seconds for TOTP, ignored for
    /// HOTP. `None` unless `algorithm == "SHA1"`.
    pub fn code(&self, now_secs: u64) -> Option<u32> {
        if self.algorithm != "SHA1" || self.secret.is_empty() || !(1..=9).contains(&self.digits) {
            return None;
        }
        Some(match self.kind {
            Kind::Hotp => otp::hotp(&self.secret, self.counter, self.digits),
            Kind::Totp => otp::totp(
                &self.secret,
                now_secs,
                if self.period == 0 { 30 } else { self.period },
                self.digits,
            ),
        })
    }
    /// Canonical URI emission.
    pub fn to_uri(&self) -> String {
        let k = match self.kind {
            Kind::Hotp => "hotp",
            Kind::Totp => "totp",
        };
        let mut q = std::format!("secret={}", base32_encode(&self.secret));
        if !self.issuer.is_empty() {
            q.push_str(&std::format!("&issuer={}", Uri::pct_encode(&self.issuer)));
        }
        if self.algorithm != "SHA1" {
            q.push_str(&std::format!("&algorithm={}", self.algorithm));
        }
        if self.digits != 6 {
            q.push_str(&std::format!("&digits={}", self.digits));
        }
        match self.kind {
            Kind::Totp => {
                if self.period != 30 {
                    q.push_str(&std::format!("&period={}", self.period));
                }
            }
            Kind::Hotp => {
                q.push_str(&std::format!("&counter={}", self.counter));
            }
        }
        std::format!("otpauth://{k}/{}?{}", Uri::pct_encode(&self.label), q)
    }
}

fn param(q: &str, key: &str) -> Option<String> {
    for kv in q.split('&') {
        if let Some(v) = kv.strip_prefix(key) {
            if let Some(v) = v.strip_prefix('=') {
                return Some(Uri::pct_decode_str(v));
            }
        }
    }
    None
}

/// Parse an `otpauth://` URI; `None` on bad scheme/kind, missing or
/// undecodable secret.
pub fn parse(s: &str) -> Option<Otp> {
    let u = Uri::parse(s);
    if u.scheme.as_deref() != Some("otpauth") {
        return None;
    }
    let kind = match u.host.as_deref() {
        Some("hotp") => Kind::Hotp,
        Some("totp") => Kind::Totp,
        _ => return None,
    };
    let q = u.query.as_deref()?;
    let secret = base32_decode(param(q, "secret")?.as_str())?;
    if secret.is_empty() {
        return None;
    }
    let label = Uri::pct_decode_str(u.path.strip_prefix('/').unwrap_or(u.path.as_str()));
    let (issuer_pre, account) = match label.split_once(':') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => (String::new(), label.clone()),
    };
    let issuer = param(q, "issuer").unwrap_or(issuer_pre);
    Some(Otp {
        kind,
        label,
        issuer,
        account,
        secret,
        algorithm: param(q, "algorithm").unwrap_or_else(|| "SHA1".to_string()),
        digits: param(q, "digits").and_then(|v| v.parse().ok()).unwrap_or(6),
        period: param(q, "period")
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        counter: param(q, "counter")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
    })
}

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn b32val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a'),
        b'2'..=b'7' => Some(c - b'2' + 26),
        _ => None,
    }
}

/// RFC 4648 base32 decode; `=` padding optional, ignored.
pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc: u64 = 0;
    let mut bits = 0u32;
    let mut saw_pad = false;
    for &c in s.as_bytes() {
        match c {
            b'=' | b' ' | b'\t' | b'\r' | b'\n' | b'-' => {
                saw_pad |= c == b'=';
                continue;
            }
            _ => {}
        }
        if saw_pad {
            return None;
        }
        let v = b32val(c)?;
        acc = (acc << 5) | v as u64;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // leftover bits must be < 8 and all-zero (canonical encoders pad)
    if bits >= 5 || (acc & ((1 << bits) - 1)) != 0 {
        // tolerate nonzero trailing bits? canonical decoders reject.
        return None;
    }
    Some(out)
}

/// RFC 4648 base32 encode with `=` padding.
pub fn base32_encode(d: &[u8]) -> String {
    let mut out = String::new();
    let mut acc: u64 = 0;
    let mut bits = 0u32;
    for &c in d {
        acc = (acc << 8) | c as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[(acc >> bits) as usize & 31] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((acc << (5 - bits)) as usize) & 31] as char);
    }
    while out.len() % 8 != 0 {
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_uri() {
        let o = parse(
            "otpauth://totp/Issuer:alice?secret=JBSWY3DPEHPK3PXP&issuer=Issuer&period=30&digits=6",
        )
        .unwrap();
        assert_eq!(o.kind, Kind::Totp);
        assert_eq!(o.label, "Issuer:alice");
        assert_eq!(o.issuer, "Issuer");
        assert_eq!(o.account, "alice");
        // "Hello!\0xde\xad\xbe\xef" is the classic base32 vector
        assert_eq!(o.secret, b"Hello!\xde\xad\xbe\xef");
        assert_eq!(o.digits, 6);
        assert_eq!(o.period, 30);
    }

    #[test]
    fn hotp_and_code() {
        let o =
            parse("otpauth://hotp/x:y?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=0").unwrap();
        assert_eq!(o.kind, Kind::Hotp);
        assert_eq!(o.counter, 0);
        // RFC 4226 SHA-1 vector: counter 0 → 755224
        assert_eq!(o.code(0), Some(755224));
        let t = parse("otpauth://totp/x:y?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ").unwrap();
        assert_eq!(t.code(59), Some(287082)); // RFC 6238 T=59 → 287082
    }

    #[test]
    fn to_uri_roundtrip() {
        let o =
            parse("otpauth://hotp/Issuer:bob?secret=GEZDGNBVGY3TQOJQ&counter=5&digits=7").unwrap();
        let back = parse(&o.to_uri()).unwrap();
        assert_eq!(o, back);
    }

    #[test]
    fn base32_self_and_vectors() {
        assert_eq!(base32_encode(b"f"), "MY======");
        assert_eq!(base32_encode(b"fo"), "MZXQ====");
        assert_eq!(base32_encode(b"foo"), "MZXW6===");
        assert_eq!(base32_encode(b"foob"), "MZXW6YQ=");
        assert_eq!(base32_encode(b"fooba"), "MZXW6YTB");
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI======");
        assert_eq!(base32_decode("MY======").unwrap(), b"f");
        assert_eq!(base32_decode("MZXW6YTBOI======").unwrap(), b"foobar");
        assert_eq!(base32_decode("mzxw6ytboi").unwrap(), b"foobar"); // lowercase ok
        assert_eq!(base32_decode("MZXW6YTB!"), None);
        // invalid final-group lengths (mod 8 ∈ {1,3,6}) → reject
        assert_eq!(base32_decode("AAA"), None);
        assert_eq!(base32_decode("AAAAAA"), None);
        // non-canonical pad bits (last char 'J' has low 2 bits = 01) → reject
        assert_eq!(base32_decode("MZXW6YTBOJ"), None);
        assert_eq!(
            base32_decode(&base32_encode(b"foobar\0")).unwrap(),
            b"foobar\0"
        );
        let mut d = Vec::new();
        for i in 0u32..40 {
            d.push((i * 37 + 11) as u8);
        }
        assert_eq!(base32_decode(&base32_encode(&d)).unwrap(), d);
    }

    #[test]
    fn sha256_parses_but_code_is_none() {
        let o = parse("otpauth://totp/x:y?secret=GEZDGNBVGY3TQOJQ&algorithm=SHA256").unwrap();
        assert_eq!(o.algorithm, "SHA256");
        assert_eq!(o.code(59), None);
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "otpauth://x/y?secret=AAAA",
            "otpauth://totp/x:y",
            "otpauth://totp/x:y?secret=!",
            "http://totp/x:y?secret=AAAA",
            "otpauth://totp/x:y?secret=",
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn defaults() {
        let o = parse("otpauth://totp/label?secret=AAAA").unwrap();
        assert_eq!(
            (o.digits, o.period, o.counter, &o.algorithm[..]),
            (6, 30, 0, "SHA1")
        );
    }

    #[test]
    fn determinism_twice() {
        let s = "otpauth://totp/Issuer:alice?secret=JBSWY3DPEHPK3PXP&issuer=Issuer";
        assert_eq!(parse(s), parse(s));
        assert_eq!(parse(s).unwrap().to_uri(), parse(s).unwrap().to_uri());
    }
}
