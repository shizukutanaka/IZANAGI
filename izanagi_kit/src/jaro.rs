//! Jaro and Jaro–Winkler similarity — the name-matching metric family
//! (`editdist` counts edits; Jaro scores *matching* characters within
//! a window plus half-transpositions, Winkler adds a prefix bonus).
//! Both return `Fixed` in `[0,1]` computed exactly in integers — the
//! similarity is a rational number, and `from_ratio` keeps it exact.
//!
//! ```
//! use izanagi_kit::jaro::{jaro, jaro_winkler};
//! use izanagi_kit::fixed::Fixed;
//!
//! // "MARTHA" / "MARHTA": 6 matches, 1 transposition →
//! // (6/6 + 6/6 + 5/6)/3 = 17/18.
//! assert_eq!(jaro(b"MARTHA", b"MARHTA"), Fixed::from_ratio(17, 18));
//! assert!(jaro_winkler(b"MARTHA", b"MARHTA") > Fixed::from_ratio(17, 18));
//! assert_eq!(jaro(b"", b""), Fixed::ONE);
//! assert_eq!(jaro(b"", b"abc"), Fixed::ZERO);
//! ```

use crate::fixed::Fixed;

/// Jaro similarity in `[0,1]`; `1` for identical strings, `0` when no
/// character matches within the search window `⌊max/2⌋−1`.
/// Both-empty is defined as `1` (identity).
pub fn jaro(a: &[u8], b: &[u8]) -> Fixed {
    let (la, lb) = (a.len(), b.len());
    if la == 0 && lb == 0 {
        return Fixed::ONE;
    }
    if la == 0 || lb == 0 {
        return Fixed::ZERO;
    }
    let window = (la.max(lb) / 2).saturating_sub(1);
    let mut a_used = vec![false; la];
    let mut b_used = vec![false; lb];
    let mut m = 0u32;
    for (i, &ca) in a.iter().enumerate() {
        let lo = i.saturating_sub(window);
        let hi = (i + window + 1).min(lb);
        for (j, &cb) in b.iter().enumerate().take(hi).skip(lo) {
            if !b_used[j] && ca == cb {
                b_used[j] = true;
                a_used[i] = true;
                m += 1;
                break;
            }
        }
    }
    if m == 0 {
        return Fixed::ZERO;
    }
    // Transpositions: matched chars that disagree in order.
    let mut t = 0u32;
    let mut j = 0usize;
    for (i, &ca) in a.iter().enumerate() {
        if !a_used[i] {
            continue;
        }
        while j < lb && !b_used[j] {
            j += 1;
        }
        if j < lb && b[j] != ca {
            t += 1;
        }
        j += 1;
    }
    let t = t / 2;
    // (m/la + m/lb + (m−t)/m)/3 = [m²(la+lb) + (m−t)la·lb] / (3·la·lb·m)
    let (la, lb, m, t) = (la as i64, lb as i64, m as i64, t as i64);
    let num = m * m * (la + lb) + (m - t) * la * lb;
    let den = 3 * la * lb * m;
    Fixed::from_ratio((num / num.gcd(&den)) as i32, (den / num.gcd(&den)) as i32)
}

/// Jaro–Winkler: `jaro + l·0.1·(1−jaro)` where `l` is the common
/// prefix length capped at 4 (Winkler's `p = 0.1`, only applied when
/// `jaro > 0.7` in the original — the boost-gate is kept here too).
pub fn jaro_winkler(a: &[u8], b: &[u8]) -> Fixed {
    let j = jaro(a, b);
    let seven_tenths = Fixed::from_ratio(7, 10);
    if j <= seven_tenths {
        return j;
    }
    let mut l = 0usize;
    while l < a.len().min(b.len()).min(4) && a[l] == b[l] {
        l += 1;
    }
    // j + l·(1/10)·(1−j)
    j + Fixed::from_ratio(l as i32, 10).mul(Fixed::ONE - j)
}

trait Gcd {
    fn gcd(&self, other: &Self) -> Self;
}
impl Gcd for i64 {
    fn gcd(&self, b: &i64) -> i64 {
        let (mut a, mut b) = (self.abs(), b.abs());
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Permille-precision oracle: compute in f64, compare within
    /// 1/500 — exact-match cases use exact rationals instead.
    fn oracle(a: &[u8], b: &[u8]) -> f64 {
        let (la, lb) = (a.len(), b.len());
        if la == 0 && lb == 0 {
            return 1.0;
        }
        if la == 0 || lb == 0 {
            return 0.0;
        }
        let window = (la.max(lb) / 2).saturating_sub(1);
        let mut au = vec![false; la];
        let mut bu = vec![false; lb];
        let mut m = 0f64;
        for (i, &ca) in a.iter().enumerate() {
            for (j, &cb) in b
                .iter()
                .enumerate()
                .take((i + window + 1).min(lb))
                .skip(i.saturating_sub(window))
            {
                if !bu[j] && ca == cb {
                    bu[j] = true;
                    au[i] = true;
                    m += 1.0;
                    break;
                }
            }
        }
        if m == 0.0 {
            return 0.0;
        }
        let mut t = 0f64;
        let mut j = 0;
        for (i, &ca) in a.iter().enumerate() {
            if !au[i] {
                continue;
            }
            while j < lb && !bu[j] {
                j += 1;
            }
            if j < lb && b[j] != ca {
                t += 1.0;
            }
            j += 1;
        }
        (m / la as f64 + m / lb as f64 + (m - t / 2.0) / m) / 3.0
    }

    fn close(f: Fixed, x: f64) -> bool {
        ((f.raw() as f64) / 65536.0 - x).abs() < 0.001
    }

    #[test]
    fn published_values() {
        assert_eq!(jaro(b"MARTHA", b"MARHTA"), Fixed::from_ratio(17, 18));
        // "DIXON"/"DICKSONX": D,I,O,N match in order, m=4, t=0 →
        // (4/5 + 4/8 + 1)/3 = 23/30 (the published value).
        assert_eq!(jaro(b"DIXON", b"DICKSONX"), Fixed::from_ratio(23, 30));
        assert_eq!(jaro(b"DWAYNE", b"DUANE"), Fixed::from_ratio(37, 45));
        assert_eq!(jaro(b"abc", b"abc"), Fixed::ONE);
        assert_eq!(jaro(b"abc", b"xyz"), Fixed::ZERO);
    }

    #[test]
    fn winkler_prefix_boost() {
        let w = jaro_winkler(b"MARTHA", b"MARHTA");
        assert!(w > jaro(b"MARTHA", b"MARHTA"));
        // "MAR"/"MARS": jaro 8/9 <1, prefix 3.
        let w2 = jaro_winkler(b"MARS", b"MAR");
        assert!(w2 > jaro(b"MARS", b"MAR"));
        // Low-jaro pairs get no boost (the 0.7 gate).
        assert_eq!(jaro_winkler(b"abc", b"xyz"), Fixed::ZERO);
        assert_eq!(jaro_winkler(b"abcde", b"wxyde"), jaro(b"abcde", b"wxyde"));
    }

    #[test]
    fn matches_f64_oracle() {
        for seed in 0..200u64 {
            let mk = |shift: u64| -> Vec<u8> {
                (0..(4 + (seed % 6) as usize))
                    .map(|i| {
                        b'a' + ((seed.wrapping_mul(2654435761 + shift + i as u64) >> 24) % 8) as u8
                    })
                    .collect()
            };
            let (a, b) = (mk(0), mk(seed + 1));
            let got = jaro(&a, &b);
            assert!(
                close(got, oracle(&a, &b)),
                "{a:?} {b:?} got {} want {}",
                got.raw() as f64 / 65536.0,
                oracle(&a, &b)
            );
        }
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(jaro(b"hello", b"world"), jaro(b"hello", b"world"));
        assert_eq!(
            jaro_winkler(b"hello", b"world"),
            jaro_winkler(b"hello", b"world")
        );
    }
}
