//! Pólya / Burnside orbit counting — how many *distinct*
//! colorings remain once a symmetry group is factored out,
//! counted exactly over [`crate::bigint::BigInt`].
//!
//! Burnside's lemma: `#orbits = (1/|G|)·Σ_{g∈G} k^{cycles(g)}`
//! where `cycles(g)` counts the permutation's cycles on the
//! positions. `burnside` takes the group literally as
//! permutations; `necklaces` and `bracelets` generate the
//! cyclic `C_n` and dihedral `D_n` groups on positions. The
//! numerator is divisible by `|G|` — the exact division is
//! done limb-wise inside `BigInt`, so no rounding exists at
//! any step.
//!
//! ```
//! use izanagi_kit::polya::{burnside, necklaces, bracelets};
//! // 4-bead necklaces, 3 colors: (1/4)(3⁴ + 3² + 2·3) = 24
//! assert_eq!(necklaces(4, 3).unwrap().to_u64(), Some(24));
//! // same with reflections allowed: (1/8)(3⁴ + 2·3 + 3·3² + 2·3³)
//! assert_eq!(bracelets(4, 3).unwrap().to_u64(), Some(21));
//! let g = vec![vec![1, 0], vec![0, 1]]; // Z2 swap on 2 positions
//! assert_eq!(burnside(2, &g).unwrap().to_u64(), Some(3)); // {aa,ab,bb}
//! ```
//!
//! References: Pólya & Read, *Combinatorial Enumeration of
//! Groups, Graphs, and Chemical Compounds* (1987); Burnside's
//! lemma via standard group-action texts.

use crate::bigint::BigInt;

/// Cycle count of a permutation `p` (`p[i]` = image of `i`).
fn cycles(p: &[u32]) -> usize {
    let n = p.len();
    let mut seen = vec![false; n];
    let mut c = 0;
    for i in 0..n {
        if !seen[i] {
            c += 1;
            let mut j = i;
            while !seen[j] {
                seen[j] = true;
                j = p[j] as usize;
            }
        }
    }
    c
}

/// Exact `b / d` for `b` divisible by `d` — schoolbook
/// limb-division from the most significant end.
fn div_u64(b: &BigInt, d: u64) -> BigInt {
    let limbs = b.limbs();
    let mut out = vec![0u64; limbs.len()];
    let mut rem = 0u128;
    for i in (0..limbs.len()).rev() {
        let cur = (rem << 64) | u128::from(limbs[i]);
        out[i] = (cur / u128::from(d)) as u64;
        rem = cur % u128::from(d);
    }
    BigInt::from_limbs(&out)
}

/// `k^{cycles}` summed over the group then divided by `|G|`.
///
/// Each `g` is a permutation as `Vec<u32>` (`g[i]` = image of
/// `i`, a bijection on `0..n`). `None` when `g` is empty, any
/// member isn't a permutation, or members disagree on `n`.
/// `colors` `k = 0` is allowed (empty colorings unless `n=0`,
/// handled naturally by `0^c`).
pub fn burnside(colors: u64, g: &[Vec<u32>]) -> Option<BigInt> {
    if g.is_empty() {
        return None;
    }
    let n = g[0].len();
    let k = BigInt::from_i128(i128::from(colors));
    let mut total = BigInt::zero();
    for p in g {
        if p.len() != n {
            return None;
        }
        let mut sorted: Vec<u32> = p.clone();
        sorted.sort_unstable();
        if sorted != (0..n as u32).collect::<Vec<_>>() {
            return None;
        }
        total = total.add(&k.pow(cycles(p) as u64));
    }
    Some(div_u64(&total, g.len() as u64))
}

/// Distinct `n`-bead necklaces over `colors` colors — cyclic
/// rotations factored out (`C_n`).
pub fn necklaces(n: usize, colors: u64) -> Option<BigInt> {
    if n == 0 {
        return None;
    }
    let g: Vec<Vec<u32>> = (0..n)
        .map(|s| (0..n).map(|i| ((i + s) % n) as u32).collect())
        .collect();
    burnside(colors, &g)
}

/// Distinct `n`-bead bracelets — rotations *and* reflections
/// factored out (`D_n`, the full dihedral group).
pub fn bracelets(n: usize, colors: u64) -> Option<BigInt> {
    if n == 0 {
        return None;
    }
    let mut g = Vec::with_capacity(2 * n);
    for s in 0..n {
        g.push((0..n).map(|i| ((i + s) % n) as u32).collect());
        g.push((0..n).map(|i| ((n + s - i % n) % n) as u32).collect());
    }
    burnside(colors, &g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Brute-force orbit count: enumerate all `k^n` colorings
    /// and take each orbit's canonical (lexicographically
    /// minimal) image under the group.
    fn oracle(colors: u64, g: &[Vec<u32>]) -> u64 {
        let n = g[0].len();
        let mut best = BTreeSet::new();
        let total = colors.pow(n as u32);
        for m in 0..total {
            let mut col = vec![0u8; n];
            let mut x = m;
            for c in col.iter_mut() {
                *c = (x % colors) as u8;
                x /= colors;
            }
            let mut canon = col.clone();
            for p in g {
                let mut img = vec![0u8; n];
                for i in 0..n {
                    img[p[i] as usize] = col[i];
                }
                if img < canon {
                    canon = img;
                }
            }
            best.insert(canon);
        }
        best.len() as u64
    }

    #[test]
    fn known_values() {
        // OEIS A000029: 2-color necklaces: n=1..8 → 2,3,4,6,8,14,20,36
        let want = [2u64, 3, 4, 6, 8, 14, 20, 36];
        for (i, &w) in want.iter().enumerate() {
            assert_eq!(necklaces(i + 1, 2).unwrap().to_u64(), Some(w));
        }
        // bracelets n=4 k=2 → 6, n=6 k=2 → 13
        assert_eq!(bracelets(4, 2).unwrap().to_u64(), Some(6));
        assert_eq!(bracelets(6, 2).unwrap().to_u64(), Some(13));
        // edge cases
        assert!(burnside(2, &[]).is_none());
        assert!(burnside(2, &[vec![1, 1]]).is_none()); // not a perm
        assert!(necklaces(0, 2).is_none());
    }

    #[test]
    fn oracle_random_groups() {
        let mut rng = SplitMix64::new(0xB012);
        for _ in 0..80 {
            let n = 1 + rng.below(6) as usize;
            let colors = 1 + u64::from(rng.below(3));
            // build a random subgroup: powers of a random perm
            // closed under inversion — cyclic is guaranteed a group
            let mut p: Vec<u32> = (0..n as u32).collect();
            rng.shuffle(&mut p);
            let mut g = vec![p.clone()];
            // close under composition
            loop {
                let mut added = false;
                let cur: Vec<Vec<u32>> = g.clone();
                for a in &cur {
                    for b in &cur {
                        let c: Vec<u32> = (0..n).map(|i| a[b[i] as usize]).collect();
                        if !g.contains(&c) {
                            g.push(c);
                            added = true;
                        }
                    }
                }
                if !added || g.len() > 24 {
                    break;
                }
            }
            let got = burnside(colors, &g).unwrap().to_u64().unwrap();
            let want = oracle(colors, &g);
            assert_eq!(got, want, "n={n} colors={colors} |G|={}", g.len());
        }
    }

    /// `k^0` handling and single-position groups.
    #[test]
    fn edge_positions() {
        assert_eq!(burnside(0, &[vec![0]]).unwrap().to_u64(), Some(0));
        assert_eq!(burnside(5, &[vec![0]]).unwrap().to_u64(), Some(5));
        // full symmetric group S3 on 3 positions with 2 colors
        // → multisets of size 3 from 2 colors = 4
        let s3: Vec<Vec<u32>> = vec![
            vec![0, 1, 2],
            vec![0, 2, 1],
            vec![1, 0, 2],
            vec![1, 2, 0],
            vec![2, 0, 1],
            vec![2, 1, 0],
        ];
        assert_eq!(burnside(2, &s3).unwrap().to_u64(), Some(4));
    }
}
