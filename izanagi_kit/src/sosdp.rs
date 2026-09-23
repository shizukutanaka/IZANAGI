//! Transforms on the subset lattice of an `n`-element universe —
//! arrays are indexed by bitmask, `len == 2^n`. Covers the three
//! classic `O(n·2^n)` fast zeta/Möbius pairs (subset-sum,
//! superset-sum, and the xor Walsh–Hadamard transform) plus the
//! convolutions built from them. These are the counting backbone for
//! bitmask DP (per-team ability loads, coverage tables) where
//! [`crate::conv`] handles ordinary polynomial multiplication.
//!
//! ```
//! use izanagi_kit::sosdp::{subset_zeta, subset_mobius, or_convolve, xor_convolve};
//! let f = vec![1i64, 0, 2, 0]; // f[00], f[01], f[10], f[11]
//! let z = subset_zeta(&f);
//! assert_eq!(z, vec![1, 1, 3, 3]); // z[S] = Σ_{T⊆S} f[T]
//! assert_eq!(subset_mobius(&z), f);
//! let g = vec![1i64, 1, 0, 0];
//! assert_eq!(or_convolve(&f, &g).unwrap_or_default(), vec![1, 1, 2, 2]);
//! assert_eq!(xor_convolve(&f, &g).unwrap_or_default(), vec![1, 1, 2, 2]); // coincides here
//! ```

/// Subset zeta transform: `z[S] = Σ_{T ⊆ S} f[T]`, in place order —
/// returns a fresh vector.
pub fn subset_zeta(f: &[i64]) -> Vec<i64> {
    let mut z = f.to_vec();
    let n = z.len();
    let mut b = 1usize;
    while b < n {
        for s in 0..n {
            if s & b != 0 {
                z[s] += z[s ^ b];
            }
        }
        b <<= 1;
    }
    z
}

/// Subset Möbius transform — exact inverse of [`subset_zeta`].
pub fn subset_mobius(z: &[i64]) -> Vec<i64> {
    let mut f = z.to_vec();
    let n = f.len();
    let mut b = 1usize;
    while b < n {
        for s in 0..n {
            if s & b != 0 {
                f[s] -= f[s ^ b];
            }
        }
        b <<= 1;
    }
    f
}

/// Superset zeta transform: `z[S] = Σ_{T ⊇ S} f[T]`.
pub fn superset_zeta(f: &[i64]) -> Vec<i64> {
    let mut z = f.to_vec();
    let n = z.len();
    let mut b = 1usize;
    while b < n {
        for s in 0..n {
            if s & b == 0 {
                z[s] += z[s | b];
            }
        }
        b <<= 1;
    }
    z
}

/// Superset Möbius transform — inverse of [`superset_zeta`].
pub fn superset_mobius(z: &[i64]) -> Vec<i64> {
    let mut f = z.to_vec();
    let n = f.len();
    let mut b = 1usize;
    while b < n {
        for s in 0..n {
            if s & b == 0 {
                f[s] -= f[s | b];
            }
        }
        b <<= 1;
    }
    f
}

/// OR-convolution: `(f ★ g)[S] = Σ_{A ∪ B = S} f[A]·g[B]`.
/// Inputs must have equal length `2^n` — `None` on mismatch.
pub fn or_convolve(f: &[i64], g: &[i64]) -> Option<Vec<i64>> {
    if f.len() != g.len() {
        return None;
    }
    let mut zf = subset_zeta(f);
    let zg = subset_zeta(g);
    for (a, b) in zf.iter_mut().zip(&zg) {
        *a *= b;
    }
    Some(subset_mobius(&zf))
}

/// AND-convolution: `(f ★ g)[S] = Σ_{A ∩ B = S} f[A]·g[B]`.
/// `None` on length mismatch.
pub fn and_convolve(f: &[i64], g: &[i64]) -> Option<Vec<i64>> {
    if f.len() != g.len() {
        return None;
    }
    let mut zf = superset_zeta(f);
    let zg = superset_zeta(g);
    for (a, b) in zf.iter_mut().zip(&zg) {
        *a *= b;
    }
    Some(superset_mobius(&zf))
}

/// Walsh–Hadamard transform, the xor-lattice zeta:
/// `w[S] = Σ_T (−1)^{|S∩T|} f[T]`. Self-inverse up to scale `2^n`.
pub fn walsh_hadamard(f: &[i64]) -> Vec<i64> {
    let mut w = f.to_vec();
    let n = w.len();
    let mut b = 1usize;
    while b < n {
        for s in 0..n {
            if s & b == 0 {
                let (x, y) = (w[s], w[s | b]);
                w[s] = x + y;
                w[s | b] = x - y;
            }
        }
        b <<= 1;
    }
    w
}

/// XOR-convolution: `(f ★ g)[S] = Σ_{A ⊕ B = S} f[A]·g[B]`, via
/// `WHT(f)·WHT(g)` then the inverse transform scaled by `2^n`.
pub fn xor_convolve(f: &[i64], g: &[i64]) -> Option<Vec<i64>> {
    if f.len() != g.len() {
        return None;
    }
    let mut wf = walsh_hadamard(f);
    let wg = walsh_hadamard(g);
    for (a, b) in wf.iter_mut().zip(&wg) {
        *a *= b;
    }
    let mut out = walsh_hadamard(&wf);
    let inv = out.len() as i64;
    for v in out.iter_mut() {
        *v /= inv;
    }
    Some(out)
}

/// Convenience: an all-zero table of size `2^bits` (start point for
/// incremental transforms).
pub fn table(bits: u32) -> Vec<i64> {
    vec![0i64; 1usize << bits]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn or_naive(f: &[i64], g: &[i64]) -> Vec<i64> {
        let n = f.len();
        let mut out = vec![0i64; n];
        for (a, &fa) in f.iter().enumerate() {
            for (b, &gb) in g.iter().enumerate() {
                out[a | b] += fa * gb;
            }
        }
        out
    }

    fn and_naive(f: &[i64], g: &[i64]) -> Vec<i64> {
        let n = f.len();
        let mut out = vec![0i64; n];
        for (a, &fa) in f.iter().enumerate() {
            for (b, &gb) in g.iter().enumerate() {
                out[a & b] += fa * gb;
            }
        }
        out
    }

    fn xor_naive(f: &[i64], g: &[i64]) -> Vec<i64> {
        let n = f.len();
        let mut out = vec![0i64; n];
        for (a, &fa) in f.iter().enumerate() {
            for (b, &gb) in g.iter().enumerate() {
                out[a ^ b] += fa * gb;
            }
        }
        out
    }

    #[test]
    fn zeta_mobius_roundtrip() {
        let mut rng = SplitMix64::new(13);
        for _ in 0..50 {
            let bits = rng.below(5);
            let n = 1usize << bits;
            let f: Vec<i64> = (0..n).map(|_| rng.below(200) as i64 - 100).collect();
            let z = subset_zeta(&f);
            // Direct O(3^n) check on a couple of random sets.
            let s = rng.below(n as u32) as usize;
            let mut want = 0i64;
            let mut t = s;
            loop {
                want += f[t];
                if t == 0 {
                    break;
                }
                t = (t - 1) & s;
            }
            assert_eq!(z[s], want);
            assert_eq!(subset_mobius(&z), f);
            // Superset twin.
            let zs = superset_zeta(&f);
            let mut want2 = 0i64;
            for (t, &ft) in f.iter().enumerate() {
                if t & s == s {
                    want2 += ft;
                }
            }
            assert_eq!(zs[s], want2);
            assert_eq!(superset_mobius(&zs), f);
        }
    }

    #[test]
    fn convolutions_match_naive() {
        let mut rng = SplitMix64::new(29);
        for _ in 0..40 {
            let bits = rng.below(4);
            let n = 1usize << bits;
            let f: Vec<i64> = (0..n).map(|_| rng.below(20) as i64).collect();
            let g: Vec<i64> = (0..n).map(|_| rng.below(20) as i64).collect();
            assert_eq!(or_convolve(&f, &g), Some(or_naive(&f, &g)));
            assert_eq!(and_convolve(&f, &g), Some(and_naive(&f, &g)));
            assert_eq!(xor_convolve(&f, &g), Some(xor_naive(&f, &g)));
        }
    }

    #[test]
    fn walsh_hadamard_involution() {
        let f = vec![3i64, 1, 0, 2];
        let w = walsh_hadamard(&f);
        let back = walsh_hadamard(&w);
        for (a, b) in f.iter().zip(&back) {
            assert_eq!(a * 4, *b);
        }
    }
}
