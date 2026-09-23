//! Formal power series over `GF(p)` with `p = 998244353` (the NTT prime
//! shared with [`crate::conv`]): truncated arithmetic `f mod x^n` where
//! the whole point is that every operation is a pure function of its
//! inputs — no floats, no randomness, no approximate anything.
//!
//! Algorithms: multiplication reuses [`crate::conv::convolve`] (`O(n log n)`
//! NTT); `inv`/`log`/`exp` are Newton iterations (`inv`: `g ← g(2 −
//! fg)`; `exp`: `g ← g(1 + f − log g)`) — the classic
//! competitive-programming FPS kit, each doubling the known prefix per
//! step. Division is the schoolbook `O(n²)` fallback since FPS
//! `divmod` via reversed `inv` has fiddly edge cases.
//!
//! `None` on precondition violations (`inv`/`log` need `f[0] ≠ 0`,
//! `log` needs `f[0] = 1`, `exp`/`pow` need `f[0] = 0`/`f[0] = 1`),
//! `Some` otherwise; never panics, never returns wrong coefficients.
//!
//! ```
//! use izanagi_kit::conv::MOD;
//! use izanagi_kit::fps::exp;
//! // e^x = 1 + x + x²/2 + x³/6 + x⁴/24 + … (mod p)
//! let e = exp(&[0, 1], 5).unwrap();
//! assert_eq!(e, vec![1, 1, MOD / 2 + 1, 166_374_059, 291_154_603]);
//! ```
use crate::conv::{convolve, MOD};

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
fn modinv(a: u64) -> u64 {
    modpow(a % MOD, MOD - 2)
}
fn norm(v: &mut Vec<u64>) {
    while v.last() == Some(&0) {
        v.pop();
    }
}
fn take(v: &[u64], n: usize) -> Vec<u64> {
    let mut out = v[..v.len().min(n)].to_vec();
    out.resize(n, 0);
    out
}

/// `f + g` coefficient-wise.
pub fn add(f: &[u64], g: &[u64]) -> Vec<u64> {
    let n = f.len().max(g.len());
    let mut out = vec![0u64; n];
    for (i, v) in out.iter_mut().enumerate() {
        *v = (*f.get(i).unwrap_or(&0) + *g.get(i).unwrap_or(&0)) % MOD;
    }
    out
}

/// `f − g` coefficient-wise.
pub fn sub(f: &[u64], g: &[u64]) -> Vec<u64> {
    let n = f.len().max(g.len());
    let mut out = vec![0u64; n];
    for (i, v) in out.iter_mut().enumerate() {
        *v = (*f.get(i).unwrap_or(&0) + MOD - *g.get(i).unwrap_or(&0)) % MOD;
    }
    out
}

/// `f · g` — `None` only when an operand is empty; every coefficient
/// reduced mod `MOD`.
pub fn mul(f: &[u64], g: &[u64]) -> Vec<u64> {
    convolve(f, g)
}

/// Scalar multiple `c · f`.
pub fn scale(f: &[u64], c: u64) -> Vec<u64> {
    let c = c % MOD;
    f.iter()
        .map(|&x| (x as u128 * c as u128 % MOD as u128) as u64)
        .collect()
}

/// Truncate `f` to the first `n` coefficients (pad-free).
pub fn trunc(f: &[u64], n: usize) -> Vec<u64> {
    let mut out = f[..f.len().min(n)].to_vec();
    norm(&mut out);
    out
}

/// Formal derivative `f'`.
pub fn derivative(f: &[u64]) -> Vec<u64> {
    if f.len() <= 1 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(f.len() - 1);
    for (i, &a) in f.iter().enumerate().skip(1) {
        out.push((a as u128 * i as u128 % MOD as u128) as u64);
    }
    out
}

/// Formal integral `∫f` with `F[0] = 0` — needs `i⁻¹` for each `i`,
/// which always exists in `GF(p)` for `i < p` (a longer series would
/// exceed the field's capacity anyway).
pub fn integral(f: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(f.len() + 1);
    out.push(0);
    for (i, &a) in f.iter().enumerate() {
        out.push((a as u128 * modinv(i as u64 + 1) as u128 % MOD as u128) as u64);
    }
    out
}

/// First `n` coefficients of `f⁻¹` (`f · f⁻¹ ≡ 1 mod x^n`).
/// `None` when `f[0] = 0` (non-unit) or `f` is empty.
pub fn inv(f: &[u64], n: usize) -> Option<Vec<u64>> {
    let f0 = *f.first()?;
    if f0 % MOD == 0 || n == 0 {
        return (n == 0).then(Vec::new);
    }
    let mut g = vec![modinv(f0)];
    let mut m = 1usize;
    while m < n {
        let m2 = (m * 2).min(n);
        let fg = convolve(&take(f, m2), &g);
        // g' = g(2 − f·g) truncated to m2
        let mut ng = Vec::with_capacity(m2);
        for i in 0..m2 {
            let fgi = *fg.get(i).unwrap_or(&0) % MOD;
            let t = if i == 0 {
                (2 + MOD - fgi) % MOD
            } else {
                (MOD - fgi) % MOD
            };
            ng.push(t);
        }
        g = convolve(&g, &ng);
        g.truncate(m2);
        m = m2;
    }
    Some(take(&g, n))
}

/// `log f mod x^n` — formal log, `log(1 + u) = u − u²/2 + u³/3 − …`.
/// `None` unless `f[0] = 1`.
pub fn log(f: &[u64], n: usize) -> Option<Vec<u64>> {
    if f.first().copied().unwrap_or(0) % MOD != 1 {
        return None;
    }
    if n == 0 {
        return Some(Vec::new());
    }
    let d = derivative(&take(f, n));
    let i = inv(f, n)?;
    let mut out = integral(&trunc(&convolve(&d, &i), n.saturating_sub(1)));
    out.truncate(n);
    Some(out)
}

/// `exp f mod x^n` — formal exponential, `exp f = Σfᵏ/k!`.
/// `None` unless `f[0] = 0`.
pub fn exp(f: &[u64], n: usize) -> Option<Vec<u64>> {
    if f.first().copied().unwrap_or(0) % MOD != 0 {
        return None;
    }
    if n == 0 {
        return Some(Vec::new());
    }
    let mut g = vec![1u64];
    let mut m = 1usize;
    while m < n {
        let m2 = (m * 2).min(n);
        // g' = g · (1 + f_{m2} − log g) mod x^{m2}
        let lg = log(&g, m2)?;
        let mut inner = Vec::with_capacity(m2);
        for i in 0..m2 {
            let fi = *f.get(i).unwrap_or(&0) % MOD;
            let gi = *lg.get(i).unwrap_or(&0) % MOD;
            let t = if i == 0 {
                (1 + fi + MOD - gi) % MOD
            } else {
                (fi + MOD - gi) % MOD
            };
            inner.push(t);
        }
        g = convolve(&g, &inner);
        g.truncate(m2);
        m = m2;
    }
    Some(take(&g, n))
}

/// `f^k mod x^n` via `exp(k·log f)` — `None` unless `f[0] = 1`.
pub fn pow(f: &[u64], k: u64, n: usize) -> Option<Vec<u64>> {
    if f.first().copied().unwrap_or(0) % MOD != 1 {
        return None;
    }
    let l = log(f, n)?;
    exp(&scale(&l, k % MOD), n)
}

/// Long division: `a = q·b + r` with `deg r < deg b`. `None` when `b`
/// is the zero polynomial.
pub fn divmod(a: &[u64], b: &[u64]) -> Option<(Vec<u64>, Vec<u64>)> {
    let mut bb = b.to_vec();
    norm(&mut bb);
    if bb.is_empty() {
        return None;
    }
    let mut r = a.to_vec();
    norm(&mut r);
    if r.len() < bb.len() {
        return Some((Vec::new(), r));
    }
    let n = r.len() - bb.len() + 1;
    let mut q = vec![0u64; n];
    let binv = modinv(bb[bb.len() - 1]);
    for i in (0..n).rev() {
        let c = (r[i + bb.len() - 1] as u128 * binv as u128 % MOD as u128) as u64;
        q[i] = c;
        if c != 0 {
            for (j, &bj) in bb.iter().enumerate() {
                let idx = i + j;
                r[idx] = (r[idx] + MOD - (bj as u128 * c as u128 % MOD as u128) as u64) % MOD;
            }
        }
    }
    norm(&mut q);
    r.truncate(bb.len() - 1);
    norm(&mut r);
    Some((q, r))
}

/// `f mod x^n` evaluated at point `x` (Horner).
pub fn eval(f: &[u64], x: u64) -> u64 {
    let x = x % MOD;
    let mut acc = 0u128;
    for &c in f.iter().rev() {
        acc = (acc * x as u128 + c as u128) % MOD as u128;
    }
    acc as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive_mul(f: &[u64], g: &[u64]) -> Vec<u64> {
        if f.is_empty() || g.is_empty() {
            return Vec::new();
        }
        let mut out = vec![0u64; f.len() + g.len() - 1];
        for (i, &a) in f.iter().enumerate() {
            for (j, &b) in g.iter().enumerate() {
                out[i + j] = (out[i + j] as u128 + a as u128 * b as u128 % MOD as u128) as u64;
            }
        }
        out.iter_mut().for_each(|v| *v %= MOD);
        out
    }
    fn rand_poly(rng: &mut SplitMix64, n: usize, lead: u64) -> Vec<u64> {
        let mut v: Vec<u64> = (0..n).map(|_| rng.below(MOD as u32) as u64 % MOD).collect();
        if !v.is_empty() {
            v[0] = lead % MOD;
        }
        v
    }

    #[test]
    fn arith_oracle() {
        let mut rng = SplitMix64::new(0xf950_a117_0001);
        for _ in 0..200 {
            let (na, nb) = (rng.below(9) as usize, rng.below(9) as usize);
            let a = rand_poly(&mut rng, na, 0);
            let b = rand_poly(&mut rng, nb, 0);
            assert_eq!(mul(&a, &b), naive_mul(&a, &b));
            // (a+b)−b = a, (a−b)+b = a  (mod x^maxdeg)
            let n = a.len().max(b.len());
            assert_eq!(trunc(&sub(&add(&a, &b), &b), n), trunc(&a, n));
            assert_eq!(trunc(&add(&sub(&a, &b), &b), n), trunc(&a, n));
            // Horner vs termwise
            let x = rng.below(MOD as u32) as u64 % MOD;
            let mut t = 0u128;
            let mut xp = 1u128;
            for &c in &a {
                t += c as u128 * xp % MOD as u128;
                xp = xp * x as u128 % MOD as u128;
            }
            assert_eq!(u128::from(eval(&a, x)), t % MOD as u128);
            // Derivative: each term a[i]·i·x^(i-1) evaluates coefficientwise.
            let d = derivative(&a);
            for (i, &c) in d.iter().enumerate() {
                assert_eq!(c, a[i + 1] * ((i + 1) as u64 % MOD) % MOD);
            }
            // Integral: evaluating at x=0 gives the constant term (a[0]).
            let g = integral(&a);
            assert_eq!(g.first().copied().unwrap_or(0), 0);
            for (i, &c) in g.iter().enumerate().skip(1) {
                assert_eq!(c, a[i - 1] * modinv(i as u64) % MOD);
            }
        }
    }

    #[test]
    fn divmod_oracle() {
        let mut rng = SplitMix64::new(0xd1a0_000d_f951);
        for _ in 0..300 {
            let (na, nb) = (rng.below(9) as usize + 1, rng.below(6) as usize + 1);
            let a = rand_poly(&mut rng, na, 0);
            let mut b = rand_poly(&mut rng, nb, 0);
            norm(&mut b);
            if b.is_empty() {
                continue;
            }
            let (q, r) = divmod(&a, &b).unwrap();
            assert!(r.len() < b.len() || r.is_empty());
            let recomposed = add(&naive_mul(&q, &b), &r);
            let mut want = a.clone();
            norm(&mut want);
            let mut got = recomposed;
            norm(&mut got);
            assert_eq!(got, want);
        }
        assert!(divmod(&[1, 2, 3], &[0]).is_none());
        assert_eq!(divmod(&[1, 2], &[1, 1, 1, 1]), Some((vec![], vec![1, 2])));
    }

    #[test]
    fn inv_oracle() {
        let mut rng = SplitMix64::new(0x1a0e_e077_000a);
        for _ in 0..150 {
            let n = rng.below(9) as usize + 1;
            let f0 = (rng.below(MOD as u32) as u64 % (MOD - 1)) + 1;
            let f = rand_poly(&mut rng, n, f0);
            let g = inv(&f, n).unwrap();
            assert_eq!(g.len(), n);
            let prod = naive_mul(&f, &g);
            for (i, &pi) in prod.iter().take(n).enumerate() {
                let want = if i == 0 { 1 } else { 0 };
                assert_eq!(pi % MOD, want, "coeff {i}");
            }
        }
        assert!(inv(&[0, 1], 4).is_none());
        assert!(inv(&[], 4).is_none());
    }

    #[test]
    fn log_exp_roundtrip() {
        let mut rng = SplitMix64::new(0xe709_109f_9501);
        for _ in 0..120 {
            let n = rng.below(8) as usize + 1;
            // f[0]=1 → exp(log f) = f; g[0]=0 → log(exp g) = g
            let f = rand_poly(&mut rng, n, 1);
            let back = exp(&log(&f, n).unwrap(), n).unwrap();
            assert_eq!(back, take(&f, n));
            let g = rand_poly(&mut rng, n, 0);
            let back2 = log(&exp(&g, n).unwrap(), n).unwrap();
            assert_eq!(back2, take(&g, n));
        }
        assert!(log(&[0, 1], 4).is_none());
        assert!(exp(&[1, 1], 4).is_none());
        // e^x coefficients are 1/k! — check factorial denominators
        let e = exp(&[0, 1], 8).unwrap();
        let mut fact = 1u64;
        for k in 1..8u64 {
            fact = (fact as u128 * k as u128 % MOD as u128) as u64;
            assert_eq!(e[k as usize], modinv(fact));
        }
    }

    #[test]
    fn pow_oracle() {
        let mut rng = SplitMix64::new(0x9077_f950_0057);
        for _ in 0..100 {
            let n = rng.below(7) as usize + 1;
            let k = rng.below(6) as u64;
            let f = rand_poly(&mut rng, n, 1);
            // reference: k-fold naive product truncated to n
            let mut want = vec![1u64];
            for _ in 0..k {
                want = trunc(&naive_mul(&want, &f), n);
            }
            let mut wn = vec![0u64; n];
            wn[..want.len()].copy_from_slice(&want);
            assert_eq!(pow(&f, k, n).unwrap(), wn);
        }
        assert!(pow(&[0, 1], 3, 5).is_none());
    }

    #[test]
    fn known_series() {
        // 1/(1−x) = Σx^k
        let g = inv(&[1, MOD - 1], 6).unwrap();
        assert_eq!(g, vec![1; 6]);
        // log(1+x) = x − x²/2 + x³/3 − x⁴/4 + …
        let l = log(&[1, 1], 5).unwrap();
        let inv2 = MOD / 2 + 1; // p odd → (p+1)/2; u64::div_ceil is 1.73+, above MSRV
        let inv3 = 332_748_118;
        let inv4 = 748_683_265;
        assert_eq!(l, vec![0, 1, MOD - inv2, inv3, MOD - inv4]);
    }
}
