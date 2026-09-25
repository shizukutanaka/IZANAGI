//! Radix-2 Cooley–Tukey FFT on `Fixed` — the transform the `biquad`/
//! `goertzel` family was missing.
//!
//! `X[k] = Σ_n x[n]·e^{−2πikn/N}` computed with twiddles from
//! [`Fixed::sin_cos`]. Powers of two take the O(n log n) butterfly;
//! other lengths fall back to an exact O(n²) DFT so every size is
//! supported — document your N to stay fast.
//!
//! Fixed-point caution: each butterfly stage adds ~1 bit of magnitude
//! headroom; signals above ~±300 will saturate the i32 raw range. The
//! implementation accumulates in i64 and clamps instead of panicking.
//!
//! ```
//! use izanagi_kit::{fft::fft, fixed::Fixed};
//!
//! // A DC input transforms to a single bin.
//! let x: Vec<(Fixed, Fixed)> = (0..8).map(|_| (Fixed::ONE, Fixed::ZERO)).collect();
//! let spec = fft(&x);
//! // DC bin ≈ 8 (Q16 twiddle rounding leaves a few ulps).
//! assert!((spec[0].0 - Fixed::from_int(8)).raw().abs() <= 8);
//! assert!(spec.iter().skip(1).all(|c| c.0.raw().abs() + c.1.raw().abs() < 400));
//! ```

use crate::fixed::Fixed;
use std::vec::Vec;

/// Complex number over `Fixed`: `(re, im)`.
pub type Cx = (Fixed, Fixed);

fn cx_add(a: Cx, b: Cx) -> Cx {
    (a.0 + b.0, a.1 + b.1)
}
fn cx_sub(a: Cx, b: Cx) -> Cx {
    (a.0 - b.0, a.1 - b.1)
}
/// `(a + bi)(c + di)` in i64 raw space; components clamp on overflow.
fn cx_mul(a: Cx, b: Cx) -> Cx {
    let re = (a.0.raw() as i64 * b.0.raw() as i64 - a.1.raw() as i64 * b.1.raw() as i64) >> 16;
    let im = (a.0.raw() as i64 * b.1.raw() as i64 + a.1.raw() as i64 * b.0.raw() as i64) >> 16;
    (
        Fixed::from_raw(re.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
        Fixed::from_raw(im.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
    )
}
/// Twiddle `e^{−iθ}` from angle θ (radians, `Fixed`).
fn twiddle(theta: Fixed) -> Cx {
    let (s, c) = theta.sin_cos();
    (c, Fixed::ZERO - s)
}

fn bit_reverse(x: &mut [Cx]) {
    let n = x.len();
    let bits = n.trailing_zeros();
    for i in 0..n {
        let r = (i as u32).reverse_bits() >> (32 - bits);
        let r = r as usize;
        if i < r {
            x.swap(i, r);
        }
    }
}

/// Forward FFT. Length must be > 0; non-power-of-two lengths use the
/// exact O(n²) fallback [`dft`].
pub fn fft(input: &[Cx]) -> Vec<Cx> {
    let n = input.len();
    if n == 0 {
        return Vec::new();
    }
    if !n.is_power_of_two() {
        return dft(input, false);
    }
    let mut a = input.to_vec();
    bit_reverse(&mut a);
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let step = n / size;
        for base in (0..n).step_by(size) {
            for k in 0..half {
                // twiddle(θ) = e^{−iθ} so forward needs θ = +2πk·step/n.
                let theta = Fixed::TWO_PI.mul(Fixed::from_ratio((k * step) as i32, n as i32));
                let w = twiddle(theta);
                let u = a[base + k];
                let t = cx_mul(w, a[base + k + half]);
                a[base + k] = cx_add(u, t);
                a[base + k + half] = cx_sub(u, t);
            }
        }
        size *= 2;
    }
    a
}

/// Inverse FFT: `ifft(fft(x)) ≈ x` within fixed-point round-off.
/// Conjugates the spectrum, transforms, conjugates, and scales by 1/N.
pub fn ifft(input: &[Cx]) -> Vec<Cx> {
    let n = input.len() as i32;
    let mut conj: Vec<Cx> = input.iter().map(|&(r, i)| (r, Fixed::ZERO - i)).collect();
    conj = fft(&conj);
    let inv_n = Fixed::from_ratio(1, n.max(1));
    conj.iter()
        .map(|&(r, i)| (r.mul(inv_n), (Fixed::ZERO - i).mul(inv_n)))
        .collect()
}

/// Exact O(n²) DFT — used automatically for non-power-of-two lengths,
/// and the reference oracle the radix-2 path is tested against.
pub fn dft(input: &[Cx], inverse: bool) -> Vec<Cx> {
    let n = input.len();
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let mut acc: Cx = (Fixed::ZERO, Fixed::ZERO);
        for (m, &x) in input.iter().enumerate() {
            // twiddle(θ) = e^{−iθ}: forward θ = +2πkm/n, inverse −2πkm/n.
            let sgn: i64 = if inverse { -1 } else { 1 };
            let num = sgn * (k as i64) * (m as i64);
            // Reduce num mod n to keep from_ratio inside i32.
            let num = num.rem_euclid(n as i64) as i32;
            let theta = Fixed::TWO_PI.mul(Fixed::from_ratio(num, n as i32));
            acc = cx_add(acc, cx_mul(x, twiddle(theta)));
        }
        out.push(acc);
    }
    out
}

/// `|X[k]|` magnitudes in raw Q16.16 — `sqrt(re² + im²)` on i64.
pub fn magnitudes(spec: &[Cx]) -> Vec<Fixed> {
    spec.iter()
        .map(|&(r, i)| {
            let rr = r.raw() as i64;
            let ii = i.raw() as i64;
            Fixed::from_raw(isqrt_u64((rr * rr + ii * ii) as u64) as i32)
        })
        .collect()
}

/// Bin index of the largest magnitude in `0..spec.len()/2`.
pub fn peak_bin(spec: &[Cx]) -> usize {
    let m = magnitudes(spec);
    let half = m.len() / 2;
    m[..half.max(1)]
        .iter()
        .enumerate()
        .max_by_key(|v| v.1.raw())
        .map(|v| v.0)
        .unwrap_or(0)
}

/// Integer square root (floor) — binary search, `isqrt(0) = 0`.
fn isqrt_u64(v: u64) -> u64 {
    if v == 0 {
        return 0;
    }
    let mut lo = 1u64;
    let mut hi = 1u64 << 32;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if mid as u128 * mid as u128 <= v as u128 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed as F;

    fn real(x: &[i32]) -> Vec<Cx> {
        x.iter().map(|&v| (F::from_int(v), F::ZERO)).collect()
    }

    #[test]
    fn delta_function_is_flat() {
        // FFT of δ[n] is all ones.
        let mut x = vec![(F::ZERO, F::ZERO); 8];
        x[0] = (F::ONE, F::ZERO);
        let s = fft(&x);
        for c in &s {
            assert_eq!(*c, (F::ONE, F::ZERO));
        }
    }

    #[test]
    fn dc_is_single_bin() {
        let x = real(&[2; 8]);
        let s = fft(&x);
        // Twiddle quantization leaves ~1e-4 residue per stage.
        assert!(
            (s[0].0.raw() - 16 * 65536).abs() < 400,
            "dc {}",
            s[0].0.raw()
        );
        for c in &s[1..] {
            assert!(c.0.raw().abs() + c.1.raw().abs() < 500);
        }
    }

    #[test]
    fn pure_sine_lands_in_its_bin() {
        // x[n] = sin(2π·3n/16) → spectrum peaks at bins 3 and 13.
        let n = 16;
        let x: Vec<Cx> = (0..n)
            .map(|k| {
                let th = F::TWO_PI.mul(F::from_ratio(3 * k, n));
                (th.sin_cos().0, F::ZERO)
            })
            .collect();
        let s = fft(&x);
        assert_eq!(peak_bin(&s), 3);
        let m = magnitudes(&s);
        assert!(m[3].raw() > 300_000, "bin3 {}", m[3].raw());
        assert!(m[5].raw() < m[3].raw() / 4);
    }

    #[test]
    fn radix2_matches_dft_oracle() {
        // Same spectrum via both paths — tolerance accounts for
        // twiddle table vs direct-angle rounding.
        let x = real(&[1, 0, -1, 2, 0, -1, 1, -2]);
        let fast = fft(&x);
        let slow = dft(&x, false);
        for k in 0..x.len() {
            let d = (fast[k].0.raw() - slow[k].0.raw()).abs()
                + (fast[k].1.raw() - slow[k].1.raw()).abs();
            assert!(d < 2500, "bin {k}: {d}");
        }
    }

    #[test]
    fn ifft_recovers_input() {
        let x = real(&[1, 2, -3, 4, 0, -1, 5, -2]);
        let spec = fft(&x);
        let back = ifft(&spec);
        for k in 0..x.len() {
            assert!((back[k].0.raw() - x[k].0.raw()).abs() < 400, "n={k}");
        }
    }

    #[test]
    fn non_power_of_two_falls_back() {
        let x = real(&[1, 0, -1]);
        let s = fft(&x);
        assert_eq!(s.len(), 3);
        // DC bin = sum.
        assert_eq!(s[0].0, F::ZERO);
        // Matches the direct DFT exactly (same code path).
        assert_eq!(s, dft(&x, false));
    }

    #[test]
    fn parseval_energy_preserved() {
        // Σ|x|² ≈ Σ|X|²/N (within fixed-point slack).
        let x = real(&[3, 1, -2, 0, 1, 1, -1, 2]);
        let s = fft(&x);
        let e_in: i128 = x
            .iter()
            .map(|v| v.0.raw() as i128 * v.0.raw() as i128)
            .sum();
        let e_out: i128 = s
            .iter()
            .map(|v| {
                (v.0.raw() as i128 * v.0.raw() as i128) + (v.1.raw() as i128 * v.1.raw() as i128)
            })
            .sum::<i128>()
            / x.len() as i128;
        let diff = (e_in - e_out).abs() * 1000 / e_in.max(1);
        assert!(diff < 10, "rel err {}‰", diff);
    }
}
