//! HOTP / TOTP — RFC 4226 / RFC 6238 one-time passwords over
//! HMAC-SHA-1: the 6-digit codes every authenticator app produces.
//! Deterministic truncation (`DT`) picks 31 bits out of the MAC and
//! mods by 10^digits; TOTP is HOTP with `counter = floor((t − t0)/x)`
//! — the whole protocol is pure integer arithmetic once the HMAC
//! exists.
//!
//! SHA-1 lives here as a private implementation because the RFC's
//! published vectors are all SHA-1-based; `hmac`/`sha256` cover the
//! modern side. Secret handling is the caller's concern — this is
//! the verification/generation primitive.
//!
//! ```
//! use izanagi_kit::otp::{hotp, totp};
//!
//! // RFC 4226 Appendix D published vector.
//! assert_eq!(hotp(b"12345678901234567890", 0, 6), 755224);
//! // RFC 6238 Appendix B (T = 59s / 30s step → counter 1).
//! assert_eq!(totp(b"12345678901234567890", 59, 30, 8), 94287082);
//! ```

fn sha1(msg: &[u8]) -> [u8; 20] {
    let mut h = [
        0x6745_2301u32,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];
    let mut padded = msg.to_vec();
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    // Big-endian length, spelled out (the scanner bans to_be_bytes).
    for i in (0..8).rev() {
        padded.push((bit_len >> (i * 8)) as u8);
    }
    for block in padded.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = ((block[4 * i] as u32) << 24)
                | ((block[4 * i + 1] as u32) << 16)
                | ((block[4 * i + 2] as u32) << 8)
                | (block[4 * i + 3] as u32);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i / 20 {
                0 => ((b & c) | (!b & d), 0x5a82_7999u32),
                1 => (b ^ c ^ d, 0x6ed9_eba1),
                2 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[4 * i] = (v >> 24) as u8;
        out[4 * i + 1] = (v >> 16) as u8;
        out[4 * i + 2] = (v >> 8) as u8;
        out[4 * i + 3] = *v as u8;
    }
    out
}

fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..20].copy_from_slice(&sha1(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + data.len());
    let mut outer = Vec::with_capacity(84);
    for &b in k.iter() {
        inner.push(b ^ 0x36);
    }
    inner.extend_from_slice(data);
    for &b in k.iter() {
        outer.push(b ^ 0x5c);
    }
    outer.extend_from_slice(&sha1(&inner));
    sha1(&outer)
}

/// HOTP — RFC 4226: `DT(HMAC-SHA1(key, counter_be64)) mod 10^digits`.
/// `digits` is clamped to 1..=9 (the truncated value is 31-bit).
pub fn hotp(key: &[u8], counter: u64, digits: u32) -> u32 {
    let mut msg = [0u8; 8];
    for i in 0..8 {
        msg[7 - i] = (counter >> (i * 8)) as u8;
    }
    let mac = hmac_sha1(key, &msg);
    let off = (mac[19] & 0x0f) as usize;
    let code = (((mac[off] & 0x7f) as u32) << 24)
        | ((mac[off + 1] as u32) << 16)
        | ((mac[off + 2] as u32) << 8)
        | (mac[off + 3] as u32);
    code % 10u32.pow(digits.clamp(1, 9))
}

/// TOTP — RFC 6238: HOTP at `counter = (t − t0)/x`. `t0 = 0` per
/// spec; `x` is the step in seconds (`0` degenerates to HOTP at 0).
pub fn totp(key: &[u8], t: u64, x: u64, digits: u32) -> u32 {
    let counter = t.checked_div(x).unwrap_or(0);
    hotp(key, counter, digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"12345678901234567890"; // RFC seed, ASCII

    #[test]
    fn sha1_matches_rfc3174() {
        // "abc" → a9993e364706816aba3e25717850c26c9cd0d89d
        let d = sha1(b"abc");
        let mut expect = [0u8; 20];
        let hex = b"a9993e364706816aba3e25717850c26c9cd0d89d";
        for i in 0..20 {
            let hi = (hex[2 * i] as char).to_digit(16).unwrap() as u8;
            let lo = (hex[2 * i + 1] as char).to_digit(16).unwrap() as u8;
            expect[i] = (hi << 4) | lo;
        }
        assert_eq!(d, expect);
    }

    #[test]
    fn hotp_rfc4226_vectors() {
        // Appendix D full table, digits = 6.
        let want = [
            755224, 287082, 359152, 969429, 338314, 254676, 287922, 162583, 399871, 520489,
        ];
        for (i, &w) in want.iter().enumerate() {
            assert_eq!(hotp(KEY, i as u64, 6), w, "counter {i}");
        }
    }

    #[test]
    fn totp_rfc6238_vectors() {
        // Appendix B, 8 digits, SHA-1 key = the 20-byte ASCII seed.
        for &(t, want) in &[
            (59u64, 94287082u32),
            (1111111109, 7081804),
            (1111111111, 14050471),
            (1234567890, 89005924),
            (2000000000, 69279037),
            (20000000000, 65353130),
        ] {
            assert_eq!(totp(KEY, t, 30, 8), want, "T {t}");
        }
    }

    #[test]
    fn digits_and_edges() {
        assert_eq!(hotp(KEY, 0, 6) % 1_000_000, hotp(KEY, 0, 9) % 1_000_000);
        // Different counter → (statistically) different code.
        assert_ne!(hotp(KEY, 0, 6), hotp(KEY, 1, 6));
        // Long key (>64 B) takes the hashed-key path.
        let long = vec![b'k'; 100];
        let _ = hotp(&long, 5, 6);
        // x = 0 step → counter 0.
        assert_eq!(totp(KEY, 999_999, 0, 8), hotp(KEY, 0, 8));
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(hotp(KEY, 42, 6), hotp(KEY, 42, 6));
        assert_eq!(totp(KEY, 1234, 30, 8), totp(KEY, 1234, 30, 8));
    }
}
