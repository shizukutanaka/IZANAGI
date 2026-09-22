//! Number-theoretic-transform convolution over `ℤ_p` with
//! `p = 998244353 = 119·2²³ + 1` and primitive root `3` — the
//! standard competitive-programming modulus, chosen so `O(n log n)`
//! polynomial multiplication needs only `u64`/`u128` integer math.
//! Use cases: "how many ways do two dice tables sum to k" as exact
//! counts, inventory-stack convolutions, polynomial products for
//! generating-function puzzles — anywhere `f32` FFT would be wrong.
//!
//! ```
//! use izanagi_kit::conv::convolve;
//! // (1 + 2x)(1 + 3x) = 1 + 5x + 6x²
//! assert_eq!(convolve(&[1, 2], &[1, 3]), vec![1, 5, 6]);
//! ```

/// The NTT prime `119·2²³ + 1`.
pub const MOD: u64 = 998_244_353;
/// Primitive root of [`MOD`].
const ROOT: u64 = 3;

fn modpow(mut b: u64, mut e: u64) -> u64 {
    let mut r = 1u64;
    b %= MOD;
    while e > 0 {
        if e & 1 == 1 {
            r = (r as u128 * b as u128 % MOD as u128) as u64;
        }
        b = (b as u128 * b as u128 % MOD as u128) as u64;
        e >>= 1;
    }
    r
}

fn addmod(a: u64, b: u64) -> u64 {
    let s = a + b;
    if s >= MOD {
        s - MOD
    } else {
        s
    }
}

fn submod(a: u64, b: u64) -> u64 {
    if a >= b {
        a - b
    } else {
        a + MOD - b
    }
}

/// In-place iterative NTT. `invert` selects the inverse transform
/// (caller divides by `n` afterwards — see [`convolve`]).
fn ntt(a: &mut [u64], invert: bool) {
    let n = a.len();
    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        // wlen^(len) = 1, primitive len-th root.
        let e = (MOD - 1) / len as u64;
        let wlen = if invert {
            modpow(ROOT, (MOD - 1) - e)
        } else {
            modpow(ROOT, e)
        };
        for i in (0..n).step_by(len) {
            let mut w = 1u64;
            for k in 0..len / 2 {
                let u = a[i + k];
                let v = (a[i + k + len / 2] as u128 * w as u128 % MOD as u128) as u64;
                a[i + k] = addmod(u, v);
                a[i + k + len / 2] = submod(u, v);
                w = (w as u128 * wlen as u128 % MOD as u128) as u64;
            }
        }
        len *= 2;
    }
}

/// `a * b` mod [`MOD`] — coefficient list of the product polynomial,
/// length `a.len() + b.len() − 1` (empty input → empty output).
pub fn convolve(a: &[u64], b: &[u64]) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let n = (a.len() + b.len() - 1).next_power_of_two();
    let mut fa: Vec<u64> = a.iter().map(|&x| x % MOD).collect();
    let mut fb: Vec<u64> = b.iter().map(|&x| x % MOD).collect();
    fa.resize(n, 0);
    fb.resize(n, 0);
    ntt(&mut fa, false);
    ntt(&mut fb, false);
    for i in 0..n {
        fa[i] = (fa[i] as u128 * fb[i] as u128 % MOD as u128) as u64;
    }
    ntt(&mut fa, true);
    let inv_n = modpow(n as u64, MOD - 2);
    for v in fa.iter_mut() {
        *v = (*v as u128 * inv_n as u128 % MOD as u128) as u64;
    }
    fa.truncate(a.len() + b.len() - 1);
    fa
}

/// Exact `i64` convolution — valid only when every true coefficient
/// satisfies `|c| ≤ (MOD−1)/2` (input magnitudes × `min(len)` must fit);
/// results that exceed the modulus wrap back into `[-(MOD−1)/2, …]`
/// silently, so this returns `None` when the bound can't be guaranteed
/// by `i64` arithmetic on the inputs.
pub fn convolve_i64(a: &[i64], b: &[i64]) -> Option<Vec<i64>> {
    let am = a.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
    let bm = b.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0);
    let k = a.len().min(b.len()) as u128;
    if am as u128 * bm as u128 * k > ((MOD - 1) / 2) as u128 {
        return None;
    }
    let fa: Vec<u64> = a.iter().map(|&v| v.rem_euclid(MOD as i64) as u64).collect();
    let fb: Vec<u64> = b.iter().map(|&v| v.rem_euclid(MOD as i64) as u64).collect();
    let c = convolve(&fa, &fb);
    Some(
        c.iter()
            .map(|&v| {
                if v > (MOD - 1) / 2 {
                    v as i64 - MOD as i64
                } else {
                    v as i64
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive(a: &[u64], b: &[u64]) -> Vec<u64> {
        if a.is_empty() || b.is_empty() {
            return Vec::new();
        }
        let mut c = vec![0u64; a.len() + b.len() - 1];
        for (i, &x) in a.iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                c[i + j] = ((c[i + j] as u128 + x as u128 * y as u128) % MOD as u128) as u64;
            }
        }
        c
    }

    fn naive_i64(a: &[i64], b: &[i64]) -> Vec<i64> {
        let mut c = vec![0i64; a.len() + b.len() - 1];
        for (i, &x) in a.iter().enumerate() {
            for (j, &y) in b.iter().enumerate() {
                c[i + j] += x * y;
            }
        }
        c
    }

    #[test]
    fn matches_naive_convolution() {
        let mut rng = SplitMix64::new(0xC04F_CAFE);
        for _ in 0..200 {
            let n = rng.below(24) as usize;
            let m = rng.below(24) as usize;
            let a: Vec<u64> = (0..n).map(|_| rng.next_u64() % MOD).collect();
            let b: Vec<u64> = (0..m).map(|_| rng.next_u64() % MOD).collect();
            assert_eq!(convolve(&a, &b), naive(&a, &b));
            assert_eq!(convolve(&a, &b), convolve(&b, &a));
        }
        // Exact small-integer mode.
        for _ in 0..200 {
            let n = rng.below(16) as usize;
            let m = rng.below(16) as usize;
            let a: Vec<i64> = (0..n)
                .map(|_| (rng.next_u64() % 200) as i64 - 100)
                .collect();
            let b: Vec<i64> = (0..m)
                .map(|_| (rng.next_u64() % 200) as i64 - 100)
                .collect();
            if a.is_empty() || b.is_empty() {
                continue;
            }
            assert_eq!(convolve_i64(&a, &b), Some(naive_i64(&a, &b)));
        }
        assert_eq!(convolve(&[], &[1, 2]), Vec::<u64>::new());
        // Two uniform six-entry tables: coefficient counts form the
        // dice-sum triangle 1,2,3,4,5,6,5,4,3,2,1 (total 36).
        let d6 = vec![1u64; 6];
        let d6d6 = convolve(&d6, &d6);
        assert_eq!(d6d6, vec![1, 2, 3, 4, 5, 6, 5, 4, 3, 2, 1]);
        // Overflowing bound returns None instead of a wrong number.
        let big = vec![1_000_000_000i64; 1000];
        assert_eq!(convolve_i64(&big, &big), None);
    }
}
