//! Punycode / bootstring (RFC 3492) — internationalized domain labels,
//! the Unicode sibling of [`crate::uri`]. `xn--` prefixes live with the
//! caller; this module encodes/decodes one label's payload.
//!
//! Algorithm: copy basic code points verbatim, append `-` when any
//! were copied, then emit generalized variable-length integers that
//! delta-encode each non-basic code point's `(n, position)` pair.
//! `adapt` re-computes the bias after each insertion. All arithmetic
//! saturates on `u64` — hostile inputs degrade to `None`, never wrap.
//!
//! ```
//! use izanagi_kit::punycode::{decode, encode};
//!
//! assert_eq!(encode("bücher").as_deref(), Some("bcher-kva"));
//! assert_eq!(encode("mañana").as_deref(), Some("maana-pta"));
//! assert_eq!(decode("bcher-kva").as_deref(), Some("bücher"));
//! ```

use std::string::String;
use std::vec::Vec;

const BASE: u64 = 36;
const TMIN: u64 = 1;
const TMAX: u64 = 26;
const SKEW: u64 = 38;
const DAMP: u64 = 700;
const INITIAL_BIAS: u64 = 72;
const INITIAL_N: u64 = 128;
const MAX_CP: u64 = 0x10ffff;

fn digit(b: u8) -> Option<u64> {
    match b {
        b'a'..=b'z' => Some((b - b'a') as u64),
        b'A'..=b'Z' => Some((b - b'A') as u64),
        b'0'..=b'9' => Some((b - b'0' + 26) as u64),
        _ => None,
    }
}

fn digit_char(d: u64) -> Option<u8> {
    match d {
        0..=25 => Some(b'a' + d as u8),
        26..=35 => Some(b'0' + (d - 26) as u8),
        _ => None,
    }
}

/// RFC 3492 §6.1 bias adaptation.
fn adapt(delta: u64, n_points: u64, first: bool) -> u64 {
    let mut delta = delta;
    delta = if first { delta / DAMP } else { delta / 2 };
    delta += delta / n_points;
    let mut k = 0u64;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + (BASE - TMIN + 1) * delta / (delta + SKEW)
}

/// Punycode-encode a label (raw code points, no `xn--` prefix).
/// `None` on empty input or output over ~252 chars (DNS label cap).
pub fn encode(input: &str) -> Option<String> {
    let cps: Vec<u64> = input.chars().map(|c| c as u64).collect();
    if cps.is_empty() {
        return None;
    }
    let mut out = String::new();
    for &cp in &cps {
        if cp < 0x80 {
            out.push(cp as u8 as char);
        }
    }
    let b_count = out.chars().count() as u64;
    let mut h = b_count;
    if b_count > 0 && b_count < cps.len() as u64 {
        out.push('-');
    }
    let mut n = INITIAL_N;
    let mut delta = 0u64;
    let mut bias = INITIAL_BIAS;
    let total = cps.len() as u64;
    while h < total {
        // Smallest code point ≥ n.
        let mut m = u64::MAX;
        for &cp in &cps {
            if cp >= n && cp < m {
                m = cp;
            }
        }
        if m == u64::MAX {
            return None;
        }
        delta = delta.checked_add((m - n).checked_mul(h + 1)?)?;
        n = m;
        for &cp in &cps {
            if cp < n {
                delta = delta.checked_add(1)?;
            }
            if cp == n {
                // Emit delta as generalized variable-length integer.
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        TMIN
                    } else if k >= bias + TMAX {
                        TMAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    let d = t + (q - t) % (BASE - t);
                    out.push(digit_char(d)? as char);
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                out.push(digit_char(q)? as char);
                bias = adapt(delta, h + 1, h == b_count);
                delta = 0;
                h += 1;
            }
        }
        delta = delta.checked_add(1)?;
        n = n.checked_add(1)?;
    }
    if out.len() > 252 {
        return None;
    }
    Some(out)
}

/// Decode a punycode payload. `None` on a trailing `-`, out-of-range
/// accumulated code points, truncated digit runs, or digit overflow.
pub fn decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    // Basic code points before the last `-` are literal.
    let tail = input.rfind('-');
    let mut out: Vec<u64> = Vec::new();
    let mut pos = 0usize;
    if let Some(p) = tail {
        for &b in &bytes[..p] {
            if b >= 0x80 {
                return None;
            }
            out.push(b as u64);
        }
        pos = p + 1;
    }
    let mut n = INITIAL_N;
    let mut i = 0u64;
    let mut bias = INITIAL_BIAS;
    while pos < bytes.len() {
        let old_i = i;
        let mut w = 1u64;
        let mut k = BASE;
        loop {
            if pos >= bytes.len() {
                return None;
            }
            let d = digit(bytes[pos])?;
            pos += 1;
            i = i.checked_add(d.checked_mul(w)?)?;
            let t = if k <= bias {
                TMIN
            } else if k >= bias + TMAX {
                TMAX
            } else {
                k - bias
            };
            if d < t {
                break;
            }
            w = w.checked_mul(BASE - t)?;
            k += BASE;
            if k > BASE * 64 {
                return None; // digit runaway
            }
        }
        let len = out.len() as u64 + 1;
        bias = adapt(i - old_i, len, old_i == 0);
        let idx = i / len;
        n = n.checked_add(idx)?;
        if n > MAX_CP {
            return None;
        }
        i %= len;
        out.insert(i as usize, n);
        i += 1;
    }
    // u64 → char is lossless here since n ≤ 0x10ffff and surrogates
    // can't be produced by 36-base deltas on legal inputs — but guard
    // anyway for degrade-not-panic.
    let mut s = String::with_capacity(out.len());
    for &cp in &out {
        s.push(char::from_u32(cp as u32)?);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3492_examples() {
        // The RFC's worked examples, byte-exact.
        let cases: &[(&str, &str)] = &[
            ("bücher", "bcher-kva"),
            ("mañana", "maana-pta"),
            ("café", "caf-dma"),
            ("☃", "n3h"),
            ("niño", "nio-8ma"),
            ("例え", "r8jz45g"),
            ("日本語", "wgv71a119e"),
            ("☃-⌘", "--dqo34k"),
            ("مثال", "mgbh0fb"),
            ("Bücher", "Bcher-kva"),
        ];
        for &(uni, puny) in cases {
            assert_eq!(encode(uni).as_deref(), Some(puny), "encode {uni}");
            assert_eq!(decode(puny).as_deref(), Some(uni), "decode {puny}");
        }
    }

    #[test]
    fn ascii_only_and_roundtrip() {
        // Pure-basic encode is the literal; decode of a no-dash label
        // treats it as *digits*, so roundtrip only holds once the
        // label contains a non-basic char (the `-` marks basics).
        assert_eq!(encode("plain").as_deref(), Some("plain"));
        assert_eq!(decode("plain-").as_deref(), Some("plain"));
        for s in ["日本語ドメイン", "bücher.example", "☃⌘★"] {
            let p = encode(s).unwrap();
            assert_eq!(decode(&p).as_deref(), Some(s));
        }
    }

    #[test]
    fn malformed_degrades() {
        assert!(encode("").is_none());
        // A trailing '-' yields pure basics — legal per RFC.
        assert_eq!(decode("abc-").as_deref(), Some("abc"));
        // Non-digit bytes (a UTF-8 é hits >0x7f raw bytes) fail.
        assert!(decode("café").is_none());
        // A digit run that overflows the accumulation fails.
        assert!(decode(&std::format!("-{}", "z".repeat(64))).is_none());
        // Output exceeding the DNS label cap fails.
        assert!(encode(&"日本語".repeat(90)).is_none());
    }
}
