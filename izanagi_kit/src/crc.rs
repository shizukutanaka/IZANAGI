//! CRC-32 (IEEE 802.3, polynomial `0xEDB88320`, reflected) —
//! table-driven, streaming. The wire-integrity complement to
//! [`crate::merkle`]: where Merkle proves *which leaf* differs,
//! a CRC cheaply proves *whether* a byte stream was corrupted,
//! with no secrets and no contention. Incremental `write` calls
//! chain exactly like one contiguous stream — chunk boundaries
//! are invisible to the checksum, which is the point.
//!
//! ```
//! use izanagi_kit::crc::crc32;
//! // RFC-classic check vector
//! assert_eq!(crc32(b"123456789"), 0xCBF43926);
//! assert_eq!(crc32(b""), 0);
//! ```

/// Reflected IEEE polynomial.
const POLY: u32 = 0xEDB8_8320;

const fn table() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { POLY ^ (c >> 1) } else { c >> 1 };
            k += 1;
        }
        t[i] = c;
        i += 1;
    }
    t
}

const T: [u32; 256] = table();

/// One-shot CRC-32 over `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = Crc32::new();
    c.write(data);
    c.finish()
}

/// A streaming CRC-32 state — `write` in any chunking; `finish`
/// yields the same value as [`crc32`] over the concatenation.
#[derive(Clone, Debug)]
pub struct Crc32 {
    state: u32,
}

impl Crc32 {
    /// Fresh state (`0xFFFFFFFF` seed).
    pub fn new() -> Self {
        Crc32 { state: !0 }
    }

    /// Feed bytes — order and chunking do not matter, only the
    /// concatenated stream.
    pub fn write(&mut self, data: &[u8]) {
        for &b in data {
            let idx = ((self.state ^ b as u32) & 0xFF) as usize;
            self.state = T[idx] ^ (self.state >> 8);
        }
    }

    /// Finalize — the CRC of everything written so far.
    pub fn finish(&self) -> u32 {
        !self.state
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Bit-level oracle: direct polynomial long division, no table.
    fn oracle_crc32(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { POLY ^ (c >> 1) } else { c >> 1 };
            }
        }
        !c
    }

    #[test]
    fn matches_bit_level_oracle_and_known_vectors() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414FA339
        );
        let mut rng = SplitMix64::new(0xC2C);
        for _ in 0..500 {
            let n = rng.below(200) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            assert_eq!(crc32(&data), oracle_crc32(&data));
        }
    }

    #[test]
    fn streaming_equals_oneshot_for_any_chunking() {
        let mut rng = SplitMix64::new(0x57EA9);
        for _ in 0..200 {
            let n = rng.below(300) as usize;
            let data: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            let mut c = Crc32::default();
            let mut i = 0;
            while i < data.len() {
                let step = 1 + rng.below(50) as usize;
                let end = (i + step).min(data.len());
                c.write(&data[i..end]);
                i = end;
            }
            assert_eq!(c.finish(), crc32(&data));
            assert_eq!(crc32(&data), oracle_crc32(&data));
        }
    }

    #[test]
    fn detects_single_bit_and_byte_corruption() {
        let data = b"deterministic state sync payload";
        let good = crc32(data);
        for i in 0..data.len() {
            for bit in 0..8 {
                let mut d = data.to_vec();
                d[i] ^= 1 << bit;
                assert_ne!(crc32(&d), good, "undetected flip at byte {i} bit {bit}");
            }
        }
        assert_eq!(crc32(b""), 0x0000_0000);
        assert_eq!(crc32(&[0u8]), 0xD202_EF8D);
    }
}
