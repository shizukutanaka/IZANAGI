//! ULID — 128-bit **U**niversally Unique **L**exicographically
//! **S**ortable **ID**entifier: 48-bit milliseconds-since-epoch in
//! the high bits, 80 bits of randomness below, rendered as 26
//! Crockford Base32 characters. The string order matches the
//! timestamp order, so sorting ULIDs sorts by creation time — no
//! index required.
//!
//! `ulid(ts, rand)` is a pure encoder; `Ulid` is a seeded monotonic
//! generator: same `ts` again → increment the random component (and
//! spill into the timestamp on overflow, like the reference
//! implementations' overflow handling). Everything is seeded —
//! replays get identical IDs.
//!
//! ```
//! use izanagi_kit::ulid::{decode, ulid, Ulid};
//!
//! let s = ulid(0x01ae_9c5f_1fc0, &[0x42; 10]);
//! assert_eq!(s.len(), 26);
//! let (ts, rand) = decode(&s).unwrap();
//! assert_eq!((ts, rand), (0x01ae_9c5f_1fc0, [0x42; 10]));
//! let mut g = Ulid::new(7);
//! assert!(g.next(100) < g.next(100)); // monotonic under a frozen clock
//! ```

/// Crockford Base32 alphabet (`I`,`L`,`O`,`U` excluded).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Encode `ts_ms` (48-bit — higher bits masked off) + 80-bit `rand`
/// to a 26-char Crockford Base32 ULID.
pub fn ulid(ts_ms: u64, rand: &[u8; 10]) -> [u8; 26] {
    let ts = ts_ms & 0xffff_ffff_ffff;
    let mut out = [0u8; 26];
    // Top 10 chars: 48-bit timestamp, MSB-first.
    for (i, c) in out.iter_mut().enumerate().take(10) {
        let shift = 5 * (9 - i) as u32;
        *c = ALPHABET[((ts >> shift) & 0x1f) as usize];
    }
    // Remaining 16 chars: 80 bits of randomness, MSB-first.
    for i in 0..16 {
        let bit = i * 5;
        let byte = bit / 8;
        let off = bit % 8;
        let v = if off <= 3 {
            rand[byte] >> (3 - off)
        } else {
            (rand[byte] << (off - 3)) | (rand[byte + 1] >> (11 - off))
        };
        out[10 + i] = ALPHABET[(v & 0x1f) as usize];
    }
    out
}

/// Decode a 26-char ULID back to `(ts_ms, rand)`; `None` on an
/// invalid character or a timestamp field above 48 bits.
/// `i`/`l` → `1`, `o` → `0` (Crockford canonicalization),
/// case-insensitive.
pub fn decode(s: &[u8; 26]) -> Option<(u64, [u8; 10])> {
    fn val(c: u8) -> Option<u8> {
        let c = c.to_ascii_uppercase();
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'A'..=b'H' => Some(c - b'A' + 10),
            b'J' | b'K' => Some(c - b'J' + 18),
            b'M' | b'N' => Some(c - b'M' + 20),
            b'P'..=b'T' => Some(c - b'P' + 22),
            b'V'..=b'Z' => Some(c - b'V' + 27),
            b'I' | b'L' => Some(1),
            b'O' => Some(0),
            _ => None,
        }
    }
    let mut ts = 0u64;
    for &c in &s[..10] {
        ts = (ts << 5) | val(c)? as u64;
    }
    if ts > 0xffff_ffff_ffff {
        return None; // timestamp must fit 48 bits
    }
    let mut rand = [0u8; 10];
    for (i, &c) in s[10..].iter().enumerate() {
        let v = val(c)? as u16;
        let bit = i * 5;
        let byte = bit / 8;
        let off = bit % 8;
        if off <= 3 {
            rand[byte] |= (v << (3 - off)) as u8;
        } else {
            rand[byte] |= (v >> (off - 3)) as u8;
            rand[byte + 1] |= ((v << (11 - off)) & 0xff) as u8;
        }
    }
    Some((ts, rand))
}

/// Seeded monotonic ULID generator. `next(ts)` returns a ULID; when
/// `ts` repeats the random part increments (strictly-greater IDs
/// within one timestamp), spilling into the timestamp like the
/// spec's reference implementation on a full-random overflow.
pub struct Ulid {
    state: u64,
    last_ts: u64,
    rand: [u8; 10],
}

impl Ulid {
    /// New generator seeded by SplitMix64.
    pub fn new(seed: u64) -> Self {
        let mut g = Ulid {
            state: seed,
            last_ts: u64::MAX,
            rand: [0; 10],
        };
        g.advance();
        g
    }

    fn splitmix(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn advance(&mut self) {
        for i in 0..self.rand.len() {
            self.rand[i] = self.splitmix() as u8;
        }
    }

    /// Next ULID at `ts_ms`. Repeated `ts` increments `rand`; an
    /// all-0xFF random tail spills: `ts` is bumped past `ts_ms`
    /// (the spec's canonical overflow answer).
    pub fn next(&mut self, ts_ms: u64) -> [u8; 26] {
        let mut ts = ts_ms & 0xffff_ffff_ffff;
        if ts == self.last_ts {
            // Increment the 80-bit random tail.
            let mut carry = true;
            for b in self.rand.iter_mut().rev() {
                if carry {
                    let (nb, c) = b.overflowing_add(1);
                    *b = nb;
                    carry = c;
                }
            }
            if carry {
                // No randomness left: bump the clock (spec choice).
                ts = ts.wrapping_add(1);
            }
        } else {
            self.advance();
        }
        self.last_ts = ts;
        ulid(ts, &self.rand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for &(ts, fill) in &[
            (0u64, 0u8),
            (0xffff_ffff_ffff, 0xff),
            (0x01ae_9c5f_1fc0, 0x42),
        ] {
            let rand = [fill; 10];
            let s = ulid(ts, &rand);
            assert_eq!(decode(&s), Some((ts, rand)));
        }
        // Canonical spec vector: 01ARZ3NDEKTSV4RRFFQ69G5FAV.
        let s = *b"01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let (ts, rand) = decode(&s).unwrap();
        assert_eq!(ts, 1_469_922_850_259);
        assert_eq!(s, ulid(ts, &rand));
    }

    #[test]
    fn crockford_canonicalization() {
        let s = ulid(1, &[0x11; 10]);
        let mut lower = s;
        for c in lower.iter_mut() {
            *c = c.to_ascii_lowercase();
        }
        assert_eq!(decode(&lower), decode(&s));
    }

    #[test]
    fn sortable_and_monotonic() {
        let mut g = Ulid::new(42);
        let t0 = g.next(1000);
        let t1 = g.next(1000); // same ts → rand increments
        let t2 = g.next(2000); // new ts → fresh rand
        assert!(t0 < t1 && t1 < t2);
        assert!(decode(&t0).unwrap().1 < decode(&t1).unwrap().1);
        assert_eq!(decode(&t2).unwrap().0, 2000);
    }

    #[test]
    fn decode_rejects_bad_input() {
        assert_eq!(decode(b"0000000000000000000000000!"), None);
        // 'U' is excluded from Crockford.
        assert_eq!(decode(b"U0000000000000000000000000"), None);
        // Leading char '8' pushes the timestamp past 48 bits.
        assert_eq!(decode(b"80000000000000000000000000"), None);
        // …while '7' is exactly the top of the range.
        assert!(decode(b"7ZZZZZZZZZZZZZZZZZZZZZZZZZ").is_some());
    }

    #[test]
    fn deterministic_twice() {
        let (mut a, mut b) = (Ulid::new(9), Ulid::new(9));
        for ts in [5u64, 5, 7] {
            assert_eq!(a.next(ts), b.next(ts));
        }
    }
}
