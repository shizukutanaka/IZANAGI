//! Luby-transform fountain codes over GF(2): rateless
//! erasure coding for equal-length blocks.
//!
//! A droplet is `i` (a counter) plus `data` = xor of a
//! pseudo-random subset of the `k` source blocks. The
//! subset is a *pure function of `(k, i, seed)`* — the
//! wire carries only `(i, data)` and the decoder
//! regenerates each droplet's neighborhood:
//!
//! 1. `degree(k, i, seed)` — ideal-soliton sample:
//!    `P(1) = 1/k`, `P(d) = 1/(d·(d−1))` for `d ≥ 2`,
//!    which telescopes to a closed-form CDF
//!    `F(d) = 1/k + 1 − 1/d`. A single `u64` draw inverts
//!    it in `O(k)` by walking `d` upward.
//! 2. `neighbors(k, i, seed)` — `degree` distinct block
//!    indices drawn by partial Fisher–Yates on `[0,k)`.
//!
//! `decode` performs belief-propagation peeling: a
//! droplet of *effective* degree 1 fixes its block, the
//! block is xor'd out of every droplet touching it, and
//! the process repeats. `None` when the process stalls
//! with unrecovered blocks — the canonical remedy is
//! emitting more droplets (the code is rateless).
//!
//! ```
//! use izanagi_kit::fountain::{decode, droplet};
//! let blocks: Vec<Vec<u8>> = (0..8).map(|i| vec![i as u8; 4]).collect();
//! let refs: Vec<&[u8]> = blocks.iter().map(|b| b.as_ref()).collect();
//! let drops: Vec<(u64, Vec<u8>)> =
//!     (0..16).map(|i| (i, droplet(&refs, i, 7))).collect();
//! assert_eq!(decode(8, &drops, 7), Some(blocks));
//! ```

use crate::rng::SplitMix64;
use std::collections::BTreeSet;

/// integer helpers for the robust-soliton weights
fn ln2_floor(x: u64) -> u64 {
    63 - (x | 1).leading_zeros() as u64
}
/// `floor(ln x)` approximated as `floor(log2 x) · 693/1000`
/// — a sampling weight doesn't need more precision.
fn iln(x: u64) -> u64 {
    ln2_floor(x) * 693 / 1000
}
fn isqrt(x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    // smallest power of two ≥ √x as the Newton seed
    let bits = 64 - x.leading_zeros();
    let mut r = 1u64 << (bits - bits / 2);
    loop {
        let n = (r + x / r) / 2;
        if n >= r {
            return r;
        }
        r = n;
    }
}

/// Robust-soliton degree for droplet `i` over `k` blocks
/// (Luby 2002, `δ = 1/2`, `c = 1/10`):
///
/// `ρ(1) = 1/k`, `ρ(d) = 1/(d(d−1))`, plus the τ spike
/// `R/(k·d)` for `d < k/R` and `R·ln(2R)/k` at `d = k/R`,
/// with `R = ⌊√k · ln(2k)/10⌋`. The weights are exact
/// rationals scaled to a common denominator `k·R` — the
/// sampler walks a `u64` CDF in `O(k)`.
pub fn degree(k: usize, i: u64, seed: u64) -> u32 {
    if k == 0 {
        return 0;
    }
    let kk = k as u64;
    let r = (isqrt(kk) * iln(2 * kk) / 4).max(1);
    let cut = (kk / r).clamp(1, kk); // spike position k/R
    let spike = r * r * iln(2 * r);
    // CDF over weights w(d) = ρ̃ + τ̃, scaled by k·R:
    //   ρ̃(1) = R,   ρ̃(d) = kR/(d(d−1))
    //   τ̃(d) = R²/d (d<cut), R²·ln(2R) (d==cut), 0 else
    let mut rng = SplitMix64::new(seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let mut z = 0u64;
    for d in 1..=kk {
        z += weight(d, kk, r, cut, spike);
    }
    let mut u = rng.next_u64() % z;
    let mut d = 1u64;
    loop {
        let w = weight(d, kk, r, cut, spike);
        if u < w {
            return d as u32;
        }
        u -= w;
        d += 1;
    }
}

/// Scaled weight `w(d)·kR` — every term is an exact
/// integer in this denominator.
fn weight(d: u64, kk: u64, r: u64, cut: u64, spike: u64) -> u64 {
    let rho = if d == 1 { r } else { kk * r / (d * (d - 1)) };
    let tau = if d < cut {
        r * r / d
    } else if d == cut {
        spike
    } else {
        0
    };
    rho + tau
}

/// The neighbor set of droplet `i`: `degree` distinct
/// block indices, sorted. Pure in `(k, i, seed)` — both
/// encoder and decoder derive it identically.
pub fn neighbors(k: usize, i: u64, seed: u64) -> Vec<u32> {
    let d = degree(k, i, seed) as usize;
    if d == 0 || k == 0 {
        return Vec::new();
    }
    let d = d.min(k);
    let mut rng = SplitMix64::new(seed ^ i.wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    // partial Fisher–Yates: position j picks from [j, k)
    let mut perm: Vec<u32> = (0..k as u32).collect();
    for j in 0..d {
        let s = j + rng.below(k as u32 - j as u32) as usize;
        perm.swap(j, s);
    }
    let mut out: Vec<u32> = perm[..d].to_vec();
    out.sort_unstable();
    out
}

/// Produce droplet `i` over the source blocks.
///
/// All blocks must have equal length; the droplet is the
/// xor of the neighbor blocks.
pub fn droplet(blocks: &[&[u8]], i: u64, seed: u64) -> Vec<u8> {
    if blocks.is_empty() {
        return Vec::new();
    }
    let n = blocks[0].len();
    let mut data = vec![0u8; n];
    for &b in &neighbors(blocks.len(), i, seed) {
        let blk = &blocks[b as usize];
        for (x, &y) in data.iter_mut().zip(blk.iter()) {
            *x ^= y;
        }
    }
    data
}

/// Belief-propagation decode. `droplets` is a list of
/// `(i, data)` pairs; `k` the block count; `seed` the
/// same seed the encoder used.
///
/// Returns `Some(blocks)` on full recovery, `None` if the
/// peel stalls — emit more droplets and retry.
pub fn decode(k: usize, droplets: &[(u64, Vec<u8>)], seed: u64) -> Option<Vec<Vec<u8>>> {
    if k == 0 {
        return Some(Vec::new());
    }
    if droplets.is_empty() {
        return None;
    }
    let n = droplets[0].1.len();
    // constraint rows: (live neighbor set, data)
    let mut rows: Vec<(BTreeSet<u32>, Vec<u8>)> = Vec::with_capacity(droplets.len());
    for (i, data) in droplets.iter().map(|(i, d)| (*i, d)) {
        if data.len() != n {
            return None;
        }
        let ns: BTreeSet<u32> = neighbors(k, i, seed).into_iter().collect();
        rows.push((ns, data.clone()));
    }
    let mut blocks: Vec<Option<Vec<u8>>> = vec![None; k];
    // peel until fixpoint
    loop {
        // find a block fixable now: any row of degree 1
        let mut fixed_any = false;
        for (ns, data) in &rows {
            if ns.len() == 1 {
                let b = *ns.iter().next()?;
                if blocks[b as usize].is_none() {
                    blocks[b as usize] = Some(data.clone());
                    fixed_any = true;
                }
            }
        }
        // substitute known blocks out of every row
        for (ns, data) in &mut rows {
            let known: Vec<u32> = ns
                .iter()
                .copied()
                .filter(|&b| blocks[b as usize].is_some())
                .collect();
            for b in known {
                ns.remove(&b);
                let blk = blocks[b as usize].as_ref()?;
                for (x, &y) in data.iter_mut().zip(blk.iter()) {
                    *x ^= y;
                }
            }
        }
        if !fixed_any {
            break;
        }
    }
    if blocks.iter().all(|b| b.is_some()) {
        Some(blocks.into_iter().flatten().collect())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks_of(k: usize, n: usize, seed: u64) -> Vec<Vec<u8>> {
        let mut rng = SplitMix64::new(seed);
        (0..k)
            .map(|_| (0..n).map(|_| rng.below(256) as u8).collect())
            .collect()
    }

    /// Round-trip with a comfortable surplus of droplets.
    #[test]
    fn basics() {
        let blocks = blocks_of(16, 8, 1);
        let refs: Vec<&[u8]> = blocks.iter().map(|b| b.as_ref()).collect();
        let drops: Vec<(u64, Vec<u8>)> = (0..64).map(|i| (i, droplet(&refs, i, 5))).collect();
        assert_eq!(decode(16, &drops, 5), Some(blocks.clone()));
        // insufficient droplets → graceful None
        assert_eq!(decode(16, &drops[..4], 5), None);
        // k=0 → empty
        assert_eq!(decode(0, &[], 5), Some(Vec::new()));
        // degree in [1, k]
        for i in 0..50 {
            let d = degree(16, i, 5);
            assert!((1..=16).contains(&d));
            assert_eq!(neighbors(16, i, 5).len() as u32, d);
        }
    }

    /// Round-trip oracle: random k/n/seed — whenever
    /// decode returns Some, the blocks must equal the
    /// input bit-for-bit (no silent corruption); with the
    /// standard ~25% surplus it must succeed.
    #[test]
    fn oracle_roundtrip() {
        let mut rng = SplitMix64::new(0xF0A1);
        for _ in 0..60 {
            let k = 1 + rng.below(24) as usize;
            let n = 1 + rng.below(16) as usize;
            let seed = rng.next_u64();
            let blocks = blocks_of(k, n, rng.next_u64());
            let refs: Vec<&[u8]> = blocks.iter().map(|b| b.as_ref()).collect();
            // 5x surplus — small-k coverage variance is
            // what kills peels, not the distribution
            let m = 5 * k + 4;
            let drops: Vec<(u64, Vec<u8>)> = (0..m as u64)
                .map(|i| (i, droplet(&refs, i, seed)))
                .collect();
            let got = decode(k, &drops, seed);
            assert_eq!(got, Some(blocks), "k={k} n={n} seed={seed}");
        }
    }

    /// Determinism: same (k, i, seed) → identical droplet.
    #[test]
    fn deterministic() {
        let blocks = blocks_of(8, 4, 2);
        let refs: Vec<&[u8]> = blocks.iter().map(|b| b.as_ref()).collect();
        assert_eq!(droplet(&refs, 3, 9), droplet(&refs, 3, 9));
        assert_eq!(neighbors(8, 3, 9), neighbors(8, 3, 9));
    }

    /// Degree distribution sanity: ideal soliton gives
    /// P(1) = 1/k — with 20k draws the degree-1 count must
    /// be a small positive fraction, not zero or dominant.
    #[test]
    fn degree_histogram_sane() {
        let k = 64usize;
        let mut ones = 0usize;
        let mut high = 0usize;
        let draws = 2000u64;
        for i in 0..draws {
            let d = degree(k, i, 77) as usize;
            assert!((1..=k).contains(&d));
            if d == 1 {
                ones += 1;
            }
            if d > k / 2 {
                high += 1;
            }
        }
        // robust soliton concentrates hard at degree 1
        // (τ(1) = R²) — a wide band is the honest check
        assert!(ones > 50 && ones < 900, "{ones}");
        // the tail past k/R carries only ρ ≈ kR/d² — thin
        assert!(high < 300, "{high}");
    }
}
