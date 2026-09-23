//! Subbotin carry-less range coder — a pure-integer arithmetic
//! codec that complements `rans` (which needs a decode-order
//! reversal) with a streaming `encode`/`decode` pair that reads
//! symbols in input order. Symbols come from a caller-supplied
//! frequency table `(cum, freq, total)` — the model is the
//! caller's problem, the coder only needs it identical on both
//! ends, so adaptive models are as deterministic as static ones.
//!
//! The state is a `[low, low+range)` interval of `u32`s. Each
//! symbol narrows it to the `(cum/total, freq/total)` slice; when
//! the top byte of `low` can no longer change it is emitted and
//! the interval renormalized. The carry-less variant deliberately
//! truncates `range` when a carry could propagate past an emitted
//! byte (`range = -low & (BOT-1)`), sacrificing a few bits of
//! compression for a codec with no unbounded carry buffer — and
//! therefore a state that is a pure function of the input.
//!
//! ```
//! use izanagi_kit::arith::{Decoder, Encoder};
//! let mut e = Encoder::new();
//! for &b in b"abracadabra" { e.encode(b as u32, 1, 256); }
//! let wire = e.finish();
//! let mut d = Decoder::new(&wire);
//! let mut out = Vec::new();
//! for _ in 0..11 {
//!     let v = d.decode(256);
//!     out.push(v as u8);
//!     d.update(v, 1, 256);
//! }
//! assert_eq!(out, b"abracadabra");
//! ```
//!
//! References: Subbotin (1999), "A simple scheme for coding a
//! bit stream with a varying source distribution"; Witten, Neal
//! & Cleary (1987), CACM 30(6) — the carry-free renormalization
//! is what makes the coder byte-oriented and stackless.

const TOP: u64 = 1 << 24;
const BOT: u64 = 1 << 16;
const FULL: u64 = 0xffff_ffff;

/// Arithmetic encoder emitting bytes in input order. Feed it
/// `encode(cum_freq, freq, total)` with the same frequency model
/// the decoder will use; `total` must be `< 1<<16`.
pub struct Encoder {
    low: u64,
    range: u64,
    out: Vec<u8>,
}

impl Default for Encoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Encoder {
    /// New encoder with a full-width interval.
    pub fn new() -> Self {
        Encoder {
            low: 0,
            range: FULL,
            out: Vec::new(),
        }
    }

    /// Encode the symbol occupying `[cum, cum+freq)` of `total`.
    /// `0 <= cum`, `1 <= freq`, `cum + freq <= total < 1<<16`.
    pub fn encode(&mut self, cum: u32, freq: u32, total: u32) {
        if total == 0 || freq == 0 || (cum as u64) + (freq as u64) > total as u64 {
            return;
        }
        self.range /= total as u64;
        self.low += cum as u64 * self.range;
        self.range *= freq as u64;
        loop {
            if self.low.wrapping_add(self.range) ^ self.low < TOP {
                // top byte stable
            } else if self.range < BOT {
                // carry might spill: shrink range to the byte edge
                self.range = self.low.wrapping_neg() & (BOT - 1);
            } else {
                break;
            }
            self.out.push((self.low >> 24) as u8);
            self.low = (self.low << 8) & FULL;
            self.range = (self.range << 8) & FULL;
        }
    }

    /// Flush the remaining state (four bytes) and return the wire.
    pub fn finish(mut self) -> Vec<u8> {
        for _ in 0..4 {
            self.out.push((self.low >> 24) as u8);
            self.low = (self.low << 8) & FULL;
        }
        self.out
    }
}

/// Arithmetic decoder over a byte wire produced by `Encoder`.
/// Mirrors the encoder's `low`/`range` exactly and keeps the
/// stream residue in `code`; missing/truncated input reads as
/// zero bytes, so decoding is always total.
pub struct Decoder<'a> {
    wire: &'a [u8],
    pos: usize,
    low: u64,
    range: u64,
    code: u64,
}

impl<'a> Decoder<'a> {
    /// New decoder over `wire` (reads the initial 4-byte window).
    pub fn new(wire: &'a [u8]) -> Self {
        let mut d = Decoder {
            wire,
            pos: 0,
            low: 0,
            range: FULL,
            code: 0,
        };
        for _ in 0..4 {
            d.code = (d.code << 8) | d.next_byte() as u64;
        }
        d
    }

    fn next_byte(&mut self) -> u8 {
        let b = *self.wire.get(self.pos).unwrap_or(&0);
        self.pos += 1;
        b
    }

    /// Identify which cumulative interval holds the residue:
    /// returns `t` with `cum <= t < cum+freq` for the symbol.
    /// Pair with `update` once the symbol is known.
    pub fn decode(&self, total: u32) -> u32 {
        if total == 0 {
            return 0;
        }
        let r = self.range / total as u64;
        if r == 0 {
            return total - 1;
        }
        let t = self.code / r;
        if t >= total as u64 {
            total - 1
        } else {
            t as u32
        }
    }

    /// Narrow the interval to `[cum, cum+freq)` — must be the
    /// same triple the encoder used for this symbol.
    pub fn update(&mut self, cum: u32, freq: u32, total: u32) {
        if total == 0 || freq == 0 || (cum as u64) + (freq as u64) > total as u64 {
            return;
        }
        self.range /= total as u64;
        let off = cum as u64 * self.range;
        self.code = self.code.wrapping_sub(off) & FULL;
        self.low = self.low.wrapping_add(off) & FULL;
        self.range *= freq as u64;
        loop {
            if self.low.wrapping_add(self.range) ^ self.low < TOP {
                // top byte stable
            } else if self.range < BOT {
                self.range = self.low.wrapping_neg() & (BOT - 1);
            } else {
                break;
            }
            self.low = (self.low << 8) & FULL;
            self.range = (self.range << 8) & FULL;
            self.code = ((self.code << 8) & FULL) | self.next_byte() as u64;
        }
    }
}

/// Decode `count` symbols with a uniform `(byte, 1, 256)` model.
pub fn roundtrip_decode(wire: &[u8], count: usize) -> Vec<u8> {
    let mut d = Decoder::new(wire);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let v = d.decode(256);
        out.push(v as u8);
        d.update(v, 1, 256);
    }
    out
}

/// Encode bytes with a uniform `(byte, 1, 256)` model.
pub fn roundtrip_encode(data: &[u8]) -> Vec<u8> {
    let mut e = Encoder::new();
    for &b in data {
        e.encode(b as u32, 1, 256);
    }
    e.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    fn roundtrip_with_model(data: &[u8], model: &BTreeMap<u32, u32>) {
        let mut cum: BTreeMap<u32, u32> = BTreeMap::new();
        let mut acc = 0u32;
        for (&k, &f) in model {
            cum.insert(k, acc);
            acc += f;
        }
        let total = acc;
        let mut e = Encoder::new();
        for &b in data {
            let f = *model.get(&(b as u32)).unwrap_or(&0);
            if f == 0 {
                return;
            }
            e.encode(cum[&(b as u32)], f, total);
        }
        let wire = e.finish();
        let mut d = Decoder::new(&wire);
        let mut out = Vec::with_capacity(data.len());
        for _ in 0..data.len() {
            let t = d.decode(total);
            let mut found = None;
            for (&k, &c) in &cum {
                let f = model[&k];
                if t >= c && t < c + f {
                    found = Some(k);
                    break;
                }
            }
            let k = match found {
                Some(k) => k,
                None => panic!("no interval holds t={t} of total={total}"),
            };
            out.push(k as u8);
            d.update(cum[&k], model[&k], total);
        }
        assert_eq!(out, data);
    }

    #[test]
    fn uniform_roundtrip() {
        let cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![0],
            b"abracadabra".to_vec(),
            (0..=255).collect(),
            vec![7u8; 1000],
        ];
        for c in &cases {
            let wire = roundtrip_encode(c);
            assert_eq!(roundtrip_decode(&wire, c.len()), *c);
        }
    }

    #[test]
    fn skewed_model_roundtrip() {
        let mut r = SplitMix64::new(0xA817_0001);
        for _ in 0..200 {
            let n = 1 + r.below(8);
            let mut model = BTreeMap::new();
            for _ in 0..n {
                model.insert(r.below(4), 1 + r.below(50));
            }
            let keys: Vec<u32> = model.keys().copied().collect();
            let data: Vec<u8> = (0..1 + r.below(60) as usize)
                .map(|_| keys[r.below(keys.len() as u32) as usize] as u8)
                .collect();
            roundtrip_with_model(&data, &model);
        }
    }

    #[test]
    fn skewed_model_uses_fewer_bytes() {
        // a peaked model should beat the uniform byte code on a
        // peaked stream — sanity of the coder, not a proof
        let mut model = BTreeMap::new();
        model.insert(0u32, 240u32);
        model.insert(1, 16);
        let data = vec![0u8; 512];
        let mut e = Encoder::new();
        for _ in &data {
            e.encode(0, model[&0], 256);
        }
        let skewed = e.finish().len();
        let uniform = roundtrip_encode(&data).len();
        assert!(skewed < uniform, "{skewed} !< {uniform}");
    }

    #[test]
    fn deterministic_and_truncation_safe() {
        let a = roundtrip_encode(b"hello deterministic world");
        let b = roundtrip_encode(b"hello deterministic world");
        assert_eq!(a, b);
        for cut in [0usize, 1, 3, 7, 20] {
            let _ = roundtrip_decode(&a[..cut.min(a.len())], 10);
        }
        let mut d = Decoder::new(&[]);
        assert_eq!(d.decode(0), 0);
        d.update(0, 1, 256);
    }
}
