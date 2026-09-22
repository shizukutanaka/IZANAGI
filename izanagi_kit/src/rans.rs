//! rANS entropy codec — Duda's range asymmetric numeral system, the
//! integer-only replacement for arithmetic coding. A `Rans` table is
//! built from observed symbol counts, normalized deterministically to
//! a power-of-two total `L`, and then `encode`/`decode` stream bytes
//! through a single `u64` state. The encoder processes the input in
//! reverse so the decoder emits the original order; the wire format
//! is `[len u32le][final state u32le][u16 emission chunks, reversed]`
//! — a pure function of `(table, input)`.
//!
//! Normalization uses largest-remainder rounding: every present
//! symbol keeps `freq >= 1`, leftover slots go to the largest
//! remainders with the smaller symbol index winning ties — the same
//! `(counts, L)` always yields the same table.
//!
//! This completes the kit's compression ladder (`rle` → `lzss`/`lzw`
//! → `huffman`/`rans`): Huffman is bit-aligned (minimum 1 bit per
//! symbol); rANS approaches the entropy bound below 1 bit/symbol.
//!
//! ```
//! use izanagi_kit::rans::Rans;
//! use std::collections::BTreeMap;
//! let mut counts = BTreeMap::new();
//! for &b in b"aabbc" { *counts.entry(b).or_insert(0u64) += 1; }
//! let t = Rans::build(&counts).unwrap();
//! let wire = t.encode(b"aabbc");
//! assert_eq!(t.decode(&wire), Some(b"aabbc".to_vec()));
//! ```

/// Normalization precision: normalized frequencies sum to `L = 1 << L_BITS`.
const L_BITS: u32 = 12;
const L: u64 = 1 << L_BITS;
/// I/O base for state renormalization — emitted chunks are `u16`.
const IO_BITS: u32 = 16;
const IO: u64 = 1 << IO_BITS;

/// A normalized rANS table over byte-valued symbols.
#[derive(Clone, Debug)]
pub struct Rans {
    /// `freq[s]` — normalized frequency (0 = absent).
    freq: [u64; 256],
    /// `cum[s]` — cumulative frequency start of symbol `s`.
    cum: [u64; 256],
}

impl Rans {
    /// Build a table from observed counts. `None` on an empty map;
    /// zero counts are dropped (a symbol absent from the map is
    /// simply never encodable). At most `L` distinct symbols are
    /// supported — always true for `u8`.
    pub fn build(counts: &std::collections::BTreeMap<u8, u64>) -> Option<Self> {
        if counts.is_empty() {
            return None;
        }
        let total: u64 = counts.values().sum();
        if total == 0 {
            return None;
        }
        // Largest-remainder normalization to sum L, keeping every
        // present symbol nonzero.
        let mut base: Vec<(u8, u64)> = counts
            .iter()
            .map(|(&s, &c)| (s, (c * L / total).max(1)))
            .collect();
        let mut used: u64 = base.iter().map(|&(_, f)| f).sum();
        // used >= n; may exceed L when n > total-spread — shave
        // surplus from the largest frequencies (ties: larger symbol
        // index loses first for canonical order).
        while used > L {
            let (idx, _) = base
                .iter()
                .enumerate()
                .filter(|(_, &(_, f))| f > 1)
                .max_by_key(|&(i, &(_, f))| (f, i))
                .or_else(|| base.iter().enumerate().max_by_key(|&(i, _)| i))?;
            base[idx].1 -= 1;
            used -= 1;
        }
        // Distribute the leftover to largest remainders (ties:
        // smaller symbol index wins).
        let mut rems: Vec<(u8, u64)> = counts.iter().map(|(&s, &c)| (s, (c * L) % total)).collect();
        rems.sort_by_key(|&(_, r)| core::cmp::Reverse(r));
        // stable on symbol index already (BTreeMap order preserved
        // under equal remainders? sort_by_key is stable — input was
        // in symbol order, so equal remainders keep symbol order).
        let mut left = L - used;
        for (s, _) in &rems {
            if left == 0 {
                break;
            }
            if let Some(e) = base.iter_mut().find(|e| e.0 == *s) {
                e.1 += 1;
                left -= 1;
            }
        }
        let mut freq = [0u64; 256];
        let mut cum = [0u64; 256];
        let mut acc = 0u64;
        for (s, f) in base {
            freq[s as usize] = f;
            cum[s as usize] = acc;
            acc += f;
        }
        Some(Self { freq, cum })
    }

    /// Symbol of a decoder slot `x mod L` — linear scan is fine at
    /// 256 entries and keeps the code table-free.
    fn slot_symbol(&self, slot: u64) -> Option<u8> {
        let mut last: Option<u8> = None;
        for s in 0..256usize {
            if self.freq[s] > 0 && slot >= self.cum[s] && slot < self.cum[s] + self.freq[s] {
                last = Some(s as u8);
            }
        }
        last
    }

    /// Encode `data`; the empty input still produces a valid wire.
    ///
    /// Symbols are consumed in reverse so the decoder reproduces the
    /// original order.
    pub fn encode(&self, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        let mut x: u64 = L; // start anywhere in [L, IO*L)
        let mut emitted: Vec<u16> = Vec::new();
        for &b in data.iter().rev() {
            let s = b as usize;
            let f = self.freq[s];
            // `build` guarantees every counted symbol has freq >= 1;
            // an absent symbol is unencodable — encoding stops, and
            // decode() then rejects the wire by its length field.
            if f == 0 {
                break;
            }
            while x >= f << IO_BITS {
                emitted.push((x & (IO - 1)) as u16);
                x >>= IO_BITS;
            }
            x = (x / f) * L + self.cum[s] + (x % f);
        }
        out.extend_from_slice(&(x as u32).to_le_bytes());
        // Emitted chunks reversed so decode reads them forward.
        for &c in emitted.iter().rev() {
            out.extend_from_slice(&c.to_le_bytes());
        }
        out
    }

    /// Decode a wire produced by `encode`. `None` on truncation or
    /// trailing garbage.
    pub fn decode(&self, wire: &[u8]) -> Option<Vec<u8>> {
        if wire.len() < 8 {
            return None;
        }
        let len = u32::from_le_bytes(wire[0..4].try_into().ok()?) as usize;
        let mut x = u32::from_le_bytes(wire[4..8].try_into().ok()?) as u64;
        let chunks = &wire[8..];
        if chunks.len() % 2 != 0 {
            return None;
        }
        let mut out = Vec::with_capacity(len);
        let mut at = 0usize;
        for _ in 0..len {
            let slot = x & (L - 1);
            let s = self.slot_symbol(slot)? as usize;
            out.push(s as u8);
            x = self.freq[s] * (x >> L_BITS) + slot - self.cum[s];
            while x < L {
                if at + 2 > chunks.len() {
                    return None;
                }
                let c = u16::from_le_bytes(chunks[at..at + 2].try_into().ok()?) as u64;
                at += 2;
                x = (x << IO_BITS) | c;
            }
        }
        if at != chunks.len() || out.len() != len {
            return None;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    fn counts_of(data: &[u8]) -> BTreeMap<u8, u64> {
        let mut m = BTreeMap::new();
        for &b in data {
            *m.entry(b).or_insert(0) += 1;
        }
        m
    }

    #[test]
    fn round_trip_random_streams() {
        let mut rng = SplitMix64::new(9);
        let data: Vec<u8> = (0..3000)
            .map(|_| (rng.below(6) * rng.below(6)) as u8)
            .collect();
        let t = Rans::build(&counts_of(&data)).unwrap();
        let wire = t.encode(&data);
        assert_eq!(t.decode(&wire), Some(data.clone()));
        // Deterministic: same build, same wire.
        assert_eq!(t.encode(&data), wire);
    }

    #[test]
    fn single_symbol_and_empty() {
        let mut m = BTreeMap::new();
        m.insert(b'z', 7u64);
        let t = Rans::build(&m).unwrap();
        let data = vec![b'z'; 100];
        assert_eq!(t.decode(&t.encode(&data)), Some(data));
        assert_eq!(t.decode(&t.encode(&[])), Some(vec![]));
    }

    #[test]
    fn truncated_or_garbled_wire_rejected() {
        let data = b"deterministic wire".to_vec();
        let t = Rans::build(&counts_of(&data)).unwrap();
        let wire = t.encode(&data);
        assert_eq!(t.decode(&wire[..4]), None);
        let mut bad = wire.clone();
        bad.pop();
        assert!(t.decode(&bad).is_none());
        let mut extra = wire.clone();
        extra.push(0);
        assert!(t.decode(&extra).is_none());
    }

    #[test]
    fn normalization_sums_to_l_and_preserves_presence() {
        let mut rng = SplitMix64::new(31);
        for _ in 0..50 {
            let mut m = BTreeMap::new();
            for _ in 0..rng.below(30) + 1 {
                m.insert(rng.below(256) as u8, rng.below(50) as u64 + 1);
            }
            let t = Rans::build(&m).unwrap();
            assert_eq!(t.freq.iter().sum::<u64>(), L);
            for &s in m.keys() {
                assert!(t.freq[s as usize] >= 1, "symbol {s} starved");
            }
        }
    }

    #[test]
    fn bit_efficient_on_skewed_input() {
        let mut rng = SplitMix64::new(77);
        let data: Vec<u8> = (0..10000)
            .map(|_| if rng.below(32) == 0 { 1 } else { 0 })
            .collect();
        let t = Rans::build(&counts_of(&data)).unwrap();
        let wire = t.encode(&data);
        assert_eq!(t.decode(&wire), Some(data));
        // ~0.2 bits/symbol entropy — wire must be far under raw.
        assert!(wire.len() < 800, "wire {}B", wire.len());
    }
}
