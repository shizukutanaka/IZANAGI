//! UUIDv7 (RFC 9562 §5.7) — the time-ordered sibling of
//! [`crate::uuid`] v4: a 48-bit Unix-millisecond timestamp in the most
//! significant bits gives k-sortable identifiers, with the remaining
//! 74 bits random.
//!
//! [`uuid7`] draws one ID for an explicit timestamp; [`Uuid7`] is the
//! monotonic generator — when `ms` does not advance, it still produces
//! increasing IDs by bumping the embedded timestamp (the simplest
//! RFC-sanctioned monotonicity method).
//!
//! Format/parse reuse [`crate::uuid::format`]/[`crate::uuid::parse`].
//!
//! ```
//! use izanagi_kit::{rng::SplitMix64, uuid7::Uuid7};
//!
//! let mut g = Uuid7::new(SplitMix64::new(1));
//! let a = g.next(1_700_000_000_000);
//! let b = g.next(1_700_000_000_000); // same ms → still strictly greater
//! assert!(a < b);
//! assert_eq!(Uuid7::timestamp_of(b), 1_700_000_000_001); // virtual bump
//! ```

use crate::rng::SplitMix64;

/// Draw a UUIDv7 for Unix-millisecond `ms`: `ts(48) | 7 | rand_a(12) |
/// 0b10 | rand_b(62)`. Timestamp bits past 48 are dropped, matching
/// implementations that stop caring past the year 10889.
pub fn uuid7(ms: u64, rng: &mut SplitMix64) -> [u8; 16] {
    let rand_a = rng.next_u64() & 0x0fff;
    let rand_b = rng.next_u64() & 0x3fff_ffff_ffff_ffff;
    let mut u = [0u8; 16];
    // 48-bit big-endian timestamp.
    for (i, b) in u.iter_mut().enumerate().take(6) {
        *b = (ms >> (8 * (5 - i))) as u8;
    }
    u[6] = 0x70 | (rand_a >> 8) as u8; // version 7 + top of rand_a
    u[7] = rand_a as u8;
    u[8] = 0x80 | (rand_b >> 56) as u8; // variant 1 + top of rand_b
    for (i, b) in u[9..].iter_mut().enumerate() {
        *b = (rand_b >> (8 * (6 - i))) as u8;
    }
    u
}

/// Monotonic UUIDv7 generator.
pub struct Uuid7 {
    rng: SplitMix64,
    last_ms: u64,
}

impl Uuid7 {
    /// Seed the generator.
    pub fn new(rng: SplitMix64) -> Self {
        Self { rng, last_ms: 0 }
    }

    /// Next ID for wall-clock `ms`. When `ms` is not strictly greater
    /// than the previous call, the embedded timestamp advances by one
    /// anyway, preserving a strictly increasing sequence.
    pub fn next(&mut self, ms: u64) -> [u8; 16] {
        let eff = if ms > self.last_ms {
            ms
        } else {
            self.last_ms + 1
        };
        self.last_ms = eff;
        uuid7(eff, &mut self.rng)
    }

    /// Extract the embedded 48-bit millisecond timestamp.
    pub fn timestamp_of(u: [u8; 16]) -> u64 {
        ((u[0] as u64) << 40)
            | ((u[1] as u64) << 32)
            | ((u[2] as u64) << 24)
            | ((u[3] as u64) << 16)
            | ((u[4] as u64) << 8)
            | u[5] as u64
    }

    /// Version (upper nibble of byte 6) and variant check — `Some(())`
    /// iff `u` is a well-formed v7 UUID.
    pub fn is_v7(u: [u8; 16]) -> bool {
        u[6] >> 4 == 0x7 && u[8] >> 6 == 0b10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_rfc9562() {
        let mut rng = SplitMix64::new(9);
        let u = uuid7(0x1234_5678_9abc, &mut rng);
        assert_eq!(&u[..6], &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        assert_eq!(u[6] >> 4, 0x7);
        assert_eq!(u[8] >> 6, 0b10);
        assert!(Uuid7::is_v7(u));
    }

    #[test]
    fn timestamp_roundtrip() {
        let mut rng = SplitMix64::new(3);
        let ms = 1_800_000_000_000u64;
        let u = uuid7(ms, &mut rng);
        assert_eq!(Uuid7::timestamp_of(u), ms);
    }

    #[test]
    fn monotonic_same_ms() {
        let mut g = Uuid7::new(SplitMix64::new(5));
        let mut prev = g.next(100);
        for _ in 0..64 {
            let next = g.next(100); // clock pinned → virtual bump
            assert!(next > prev);
            prev = next;
        }
        // Newer ms → lexicographically later: byte order == time order.
        let mut g3 = Uuid7::new(SplitMix64::new(7));
        let a = g3.next(10);
        let b = g3.next(20);
        assert!(a < b);
    }

    #[test]
    fn format_parses() {
        let mut g = Uuid7::new(SplitMix64::new(11));
        let u = g.next(1_700_000_000_000);
        let s = crate::uuid::format(u);
        assert_eq!(crate::uuid::parse(&s), Some(u));
    }
}
