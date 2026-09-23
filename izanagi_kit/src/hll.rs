//! HyperLogLog cardinality estimation, integer-only — `p` index
//! bits split the hash into `m = 1<<p` buckets; each bucket keeps
//! the longest leading-zero run of the remaining hash bits plus
//! one. The raw estimate `α·m²/Σ2^{-rᵢ}` is evaluated with a
//! 64-bit fixed-point denominator, so it is a pure function of
//! the register array — no `f64`, no floating harmonic mean.
//!
//! Merge is elementwise max, exactly the HLL merge: idempotent,
//! commutative, and equal to sketching the union of the streams —
//! the property `kmv` has as well, but HLL trades KMV's exact
//! k-th-minimum bound for `O(log log n)`-space registers and a
//! constant relative error of `~1.04/√m`.
//!
//! The small-range correction (`E ≤ 5m/2` ⇒ `m·ln(m/V)` over the
//! `V` empty registers) is included: `ln` is evaluated by a Q32
//! fixed-point atanh series — no `f64` anywhere in the estimate.
//!
//! ```
//! use izanagi_kit::hll::HyperLogLog;
//! let mut h = HyperLogLog::new(6); // m = 64 registers
//! for i in 0..5_000u64 { h.add(&i); }
//! let e = h.estimate();
//! assert!((4_000..=6_250).contains(&e));
//! ```
//!
//! References: Flajolet, Fusy, Gandouet & Meunier (2007), "the
//! analysis of a near-optimal cardinality estimation algorithm";
//! Heule, Nunkesser & Hall (2013) for the register/merge shape.

use crate::rng::SplitMix64;
use crate::world_hash::hash_state;

/// Minimum precision (`m = 16` registers).
pub const MIN_P: u8 = 4;
/// Maximum precision (`m = 65536` registers).
pub const MAX_P: u8 = 16;

/// `α·m·2^16` per precision. α: m=16→0.673, 32→0.697, 64→0.709,
/// m≥128→0.7213/(1+1.079/m) — evaluated at each m exactly.
const AM_Q16: [i128; (MAX_P + 1) as usize] = [
    0,
    0,
    0,
    0,             // p < MIN_P unused
    705_692,       // p=4
    1_461_710,     // p=5
    2_973_962,     // p=6
    6_001_021,     // p=7
    12_050_679,    // p=8
    24_151_196,    // p=9
    48_352_065,    // p=10
    96_754_553,    // p=11
    193_610_169,   // p=12
    387_323_213,   // p=13
    774_647_600,   // p=14
    1_549_297_339, // p=15
    3_098_595_806, // p=16
];

/// HyperLogLog register array over `m = 1<<p` buckets. `add`
/// hashes the key with `DetHash` then one SplitMix64 avalanche so
/// the `p` index bits and the counted tail are independent —
/// the sketch is a pure function of the item multiset.
pub struct HyperLogLog {
    p: u8,
    regs: Vec<u8>,
}

impl HyperLogLog {
    /// New sketch with `m = 1<<p` registers; `p` is clamped to
    /// `[MIN_P, MAX_P]`.
    pub fn new(p: u8) -> Self {
        let p = p.clamp(MIN_P, MAX_P);
        HyperLogLog {
            p,
            regs: vec![0; 1usize << p],
        }
    }

    /// Number of registers (`1<<p`).
    pub fn registers(&self) -> usize {
        self.regs.len()
    }

    /// Precision actually in use after clamping.
    pub fn precision(&self) -> u8 {
        self.p
    }

    /// Add one key. Duplicate adds are free — the register only
    /// records the maximum zero-run seen for its bucket.
    pub fn add<T: crate::world_hash::DetHash + ?Sized>(&mut self, key: &T) {
        let h = SplitMix64::new(hash_state(key)).next_u64();
        let idx = (h >> (64 - self.p)) as usize;
        let rest = h << self.p;
        let zeros = if rest == 0 {
            64 - self.p
        } else {
            rest.leading_zeros() as u8
        };
        let r = zeros.saturating_add(1);
        if r > self.regs[idx] {
            self.regs[idx] = r;
        }
    }

    /// Merge another sketch of the same precision (elementwise
    /// max — sketching the union stream gives the same registers).
    /// Different `p` returns `None`.
    #[must_use]
    pub fn merge(&mut self, other: &Self) -> Option<()> {
        if self.regs.len() != other.regs.len() {
            return None;
        }
        for (a, b) in self.regs.iter_mut().zip(other.regs.iter()) {
            if *b > *a {
                *a = *b;
            }
        }
        Some(())
    }

    /// Estimated cardinality. Raw: `E = αm·m·2^64/(Z·2^64·2^16)`
    /// with `Z·2^64 = Σ2^{64-rᵢ}`, exact `i128`. When `E` falls in
    /// the small range (`E ≤ 5m/2` with `V` empty registers) the
    /// standard linear-counting correction `m·ln(m/V)` applies —
    /// `ln` computed by a fixed-point atanh series (Q32), so the
    /// whole estimate stays a pure function of the registers.
    pub fn estimate(&self) -> i128 {
        let m = self.regs.len() as i128;
        let mut z: i128 = 0;
        let mut zeros = 0u64;
        for &r in &self.regs {
            z += 1i128 << (64 - r.min(64) as u32);
            if r == 0 {
                zeros += 1;
            }
        }
        if z == 0 {
            return 0;
        }
        let am = AM_Q16[self.p as usize];
        let raw = (am * m * (1i128 << 64) / (z * 65536)).max(0);
        if zeros > 0 && raw * 2 <= m * 5 {
            // m · ln(m/V), rounded
            let ln = ln_ratio_q32(m as u64, zeros);
            return (m * ln + (1i128 << 31)) >> 32;
        }
        raw
    }
}

/// `ln(num/den)` in Q32 fixed point for `num >= den >= 1`,
/// `num/den <= 65536`. Reduce to `ln = k·ln2 + ln(r)` with
/// `r ∈ [1,2)`, then `ln(r) = 2·atanh(t)`, `t = (r-1)/(r+1)` —
/// |t| <= 1/3 so the series converges in ~40 terms. All `i128`.
fn ln_ratio_q32(num: u64, den: u64) -> i128 {
    const LN2_Q32: i128 = 2_977_044_471; // ln2 · 2^32
    if den == 0 || num <= den {
        return 0;
    }
    // find k with den·2^k <= num < den·2^{k+1}
    let mut k = 0u32;
    let mut d2 = den as u128;
    while (d2 << 1) <= num as u128 {
        d2 <<= 1;
        k += 1;
    }
    // t = (num - d2) / (num + d2), Q32
    let n = num as u128;
    let t = (((n - d2) << 32) / (n + d2)) as i128;
    let t2 = (t * t) >> 32;
    let mut sum = 0i128;
    let mut term = t;
    for i in 0..60i128 {
        let add = term / (2 * i + 1);
        if add == 0 && i > 4 {
            break;
        }
        sum += add;
        term = (term * t2) >> 32;
    }
    (k as i128) * LN2_Q32 + 2 * sum
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn stream(seed: u64, n: usize) -> Vec<u64> {
        let mut r = SplitMix64::new(seed);
        let mut v = BTreeSet::new();
        while v.len() < n {
            v.insert(r.next_u64());
        }
        v.into_iter().collect()
    }

    #[test]
    fn estimate_scales_with_cardinality() {
        for &n in &[100usize, 1_000, 10_000, 100_000] {
            let mut h = HyperLogLog::new(10); // m=1024, ~3.2% std err
            for &k in &stream(42, n) {
                h.add(&k);
            }
            let e = h.estimate();
            let lo = (n as i128) * 85 / 100;
            let hi = (n as i128) * 118 / 100;
            assert!(lo <= e && e <= hi, "n={n} e={e} outside [{lo},{hi}]");
        }
    }

    #[test]
    fn estimate_monotone_in_n() {
        let mut h = HyperLogLog::new(11);
        let keys = stream(7, 20_000);
        let mut prev = 0i128;
        for (i, &k) in keys.iter().enumerate() {
            h.add(&k);
            if i % 997 == 996 {
                let e = h.estimate();
                assert!(e >= prev || prev - e < (i as i128) / 5);
                prev = e;
            }
        }
    }

    #[test]
    fn merge_matches_union_stream() {
        let a_items = stream(1, 800);
        let b_items = stream(2, 800);
        let mut ha = HyperLogLog::new(8);
        let mut hb = HyperLogLog::new(8);
        for &k in &a_items {
            ha.add(&k);
        }
        for &k in &b_items {
            hb.add(&k);
        }
        let mut hu = HyperLogLog::new(8);
        for &k in a_items.iter().chain(b_items.iter()) {
            hu.add(&k);
        }
        let mut merged = HyperLogLog::new(8);
        assert_eq!(merged.merge(&ha), Some(()));
        assert_eq!(merged.merge(&hb), Some(()));
        assert_eq!(merged.regs, hu.regs);
        let mut again = HyperLogLog::new(8);
        again.merge(&hb).unwrap();
        again.merge(&ha).unwrap();
        again.merge(&ha).unwrap();
        assert_eq!(again.regs, merged.regs);
        let wrong = HyperLogLog::new(9);
        assert_eq!(merged.merge(&wrong), None);
    }

    #[test]
    fn duplicates_are_free_and_deterministic() {
        let mut h = HyperLogLog::new(6);
        assert_eq!(h.estimate(), 0);
        for _ in 0..50 {
            h.add(&7u64);
        }
        let small = h.estimate();
        let mut big = HyperLogLog::new(6);
        for i in 0..1000u64 {
            big.add(&i);
        }
        assert!(small < big.estimate());
        let mut a = HyperLogLog::new(7);
        let mut b = HyperLogLog::new(7);
        for i in 0..500u64 {
            a.add(&i);
            b.add(&i);
        }
        assert_eq!(a.regs, b.regs);
        assert_eq!(a.estimate(), b.estimate());
        assert_eq!(HyperLogLog::new(255).registers(), 1 << MAX_P);
        assert_eq!(HyperLogLog::new(0).registers(), 1 << MIN_P);
        assert_eq!(HyperLogLog::new(255).precision(), MAX_P);
        assert_eq!(HyperLogLog::new(0).precision(), MIN_P);
        assert_eq!(HyperLogLog::new(10).precision(), 10);
    }
}
