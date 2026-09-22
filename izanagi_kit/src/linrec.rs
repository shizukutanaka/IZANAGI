//! Linear recurrences — `a[k]` in `O(d³ log k)` via companion-matrix
//! exponentiation, where naive iteration is `O(d·k)`. Covers every
//! `aₙ = c₀·aₙ₋₁ + … + c_{d-1}·aₙ₋d` chain: Fibonacci-style population
//! growth, interest/tick accumulation, fixed-seed sequence lookups.
//! Two variants: exact `i128` (with `Option` overflow gate) and
//! modular `u64` (always total, `u128` intermediates).
//!
//! ```
//! use izanagi_kit::linrec::linrec;
//! // Fibonacci: a[n] = a[n-1] + a[n-2], a0=0, a1=1.
//! assert_eq!(linrec(&[1, 1], &[0, 1], 10), Some(55));
//! ```

/// `a[k]` of the recurrence `aₙ = Σᵢ coef[i]·aₙ₋₁₋ᵢ` seeded by
/// `init[0..d]`. `None` when `coef`/`init` lengths differ or are empty,
/// or when `i128` overflows during exponentiation.
pub fn linrec(coef: &[i64], init: &[i64], k: u64) -> Option<i64> {
    let d = coef.len();
    if d == 0 || init.len() != d {
        return None;
    }
    if (k as usize) < d {
        return Some(init[k as usize]);
    }
    // Companion matrix: rows 1..d shift, row 0 holds coef.
    let mut c = vec![vec![0i128; d]; d];
    for i in 1..d {
        c[i][i - 1] = 1;
    }
    for (j, &w) in coef.iter().enumerate() {
        c[0][j] = w as i128;
    }
    let m = mat_pow(&c, k - (d as u64) + 1)?;
    // a[k] = row 0 of C^(k-d+1) applied to the seed column
    // (a[d-1], …, a[0]).
    let mut acc = 0i128;
    for j in 0..d {
        acc = acc.checked_add(m[0][j].checked_mul(init[d - 1 - j] as i128)?)?;
    }
    i64::try_from(acc).ok()
}

/// Modular variant — `a[k] mod m` in `O(d³ log k)`, always total.
/// `m == 0` returns 0.
pub fn linrec_mod(coef: &[i64], init: &[i64], k: u64, m: u64) -> u64 {
    if m == 0 {
        return 0;
    }
    let d = coef.len();
    if d == 0 || init.len() != d {
        return 0;
    }
    if (k as usize) < d {
        return (init[k as usize].rem_euclid(m as i64)) as u64;
    }
    let mut c = vec![vec![0u128; d]; d];
    for i in 1..d {
        c[i][i - 1] = 1;
    }
    for (j, &w) in coef.iter().enumerate() {
        c[0][j] = (w.rem_euclid(m as i64) as u64) as u128;
    }
    let mm = m as u128;
    let mp = mat_pow_mod(&c, k - (d as u64) + 1, mm);
    let mut acc = 0u128;
    for j in 0..d {
        let v = (init[d - 1 - j].rem_euclid(m as i64) as u64) as u128;
        acc = (acc + mp[0][j] * v) % mm;
    }
    acc as u64
}

fn mat_mul(a: &[Vec<i128>], b: &[Vec<i128>]) -> Option<Vec<Vec<i128>>> {
    let d = a.len();
    let mut out = vec![vec![0i128; d]; d];
    for i in 0..d {
        for (l, &ail) in a[i].iter().enumerate() {
            if ail == 0 {
                continue;
            }
            for j in 0..d {
                if b[l][j] != 0 {
                    out[i][j] = out[i][j].checked_add(ail.checked_mul(b[l][j])?)?;
                }
            }
        }
    }
    Some(out)
}

fn mat_pow(m: &[Vec<i128>], mut e: u64) -> Option<Vec<Vec<i128>>> {
    let d = m.len();
    let mut r = vec![vec![0i128; d]; d];
    for (i, row) in r.iter_mut().enumerate() {
        row[i] = 1;
    }
    let mut base = m.to_vec();
    while e > 0 {
        if e & 1 == 1 {
            r = mat_mul(&r, &base)?;
        }
        base = mat_mul(&base, &base)?;
        e >>= 1;
    }
    Some(r)
}

fn mat_mul_mod(a: &[Vec<u128>], b: &[Vec<u128>], m: u128) -> Vec<Vec<u128>> {
    let d = a.len();
    let mut out = vec![vec![0u128; d]; d];
    for i in 0..d {
        for (l, &ail) in a[i].iter().enumerate() {
            if ail == 0 {
                continue;
            }
            for j in 0..d {
                if b[l][j] != 0 {
                    out[i][j] = (out[i][j] + ail * b[l][j]) % m;
                }
            }
        }
    }
    out
}

fn mat_pow_mod(m: &[Vec<u128>], mut e: u64, md: u128) -> Vec<Vec<u128>> {
    let d = m.len();
    let mut r = vec![vec![0u128; d]; d];
    for (i, row) in r.iter_mut().enumerate() {
        row[i] = 1;
    }
    let mut base = m.to_vec();
    while e > 0 {
        if e & 1 == 1 {
            r = mat_mul_mod(&r, &base, md);
        }
        base = mat_mul_mod(&base, &base, md);
        e >>= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive O(d·k) recurrence walk in i128, None on overflow.
    fn oracle(coef: &[i64], init: &[i64], k: u64) -> Option<i64> {
        let d = coef.len();
        if d == 0 || init.len() != d {
            return None;
        }
        if (k as usize) < d {
            return Some(init[k as usize]);
        }
        let mut a: Vec<i128> = init.iter().map(|&v| v as i128).collect();
        for _ in d as u64..=k {
            let mut nxt = 0i128;
            for i in 0..d {
                nxt = nxt.checked_add(a[d - 1 - i].checked_mul(coef[i] as i128)?)?;
            }
            a.remove(0);
            a.push(nxt);
        }
        i64::try_from(a[d - 1]).ok()
    }

    #[test]
    fn matches_naive_recurrence() {
        let mut rng = SplitMix64::new(0x11AB_51EC);
        for _ in 0..300 {
            let d = (rng.below(5) + 1) as usize;
            let coef: Vec<i64> = (0..d).map(|_| (rng.below(7) as i64) - 3).collect();
            let init: Vec<i64> = (0..d).map(|_| (rng.below(11) as i64) - 5).collect();
            let k = rng.below(60) as u64;
            assert_eq!(linrec(&coef, &init, k), oracle(&coef, &init, k));
            // Modular agrees with exact when exact fits and m is prime-ish.
            if let Some(v) = oracle(&coef, &init, k) {
                for m in [7u64, 1000, 1_000_000_007] {
                    assert_eq!(
                        linrec_mod(&coef, &init, k, m) as i128,
                        v.rem_euclid(m as i64) as i128,
                        "mod {m} {coef:?} {init:?} k={k}"
                    );
                }
            }
        }
        // Fibonacci knowns.
        assert_eq!(linrec(&[1, 1], &[0, 1], 0), Some(0));
        assert_eq!(linrec(&[1, 1], &[0, 1], 1), Some(1));
        assert_eq!(linrec(&[1, 1], &[0, 1], 90), Some(2880067194370816120));
        // Degenerate args.
        assert_eq!(linrec(&[], &[], 5), None);
        assert_eq!(linrec(&[1, 1], &[0], 5), None);
        assert_eq!(linrec_mod(&[1, 1], &[0, 1], 10, 0), 0);
        assert_eq!(linrec_mod(&[1, 1], &[0, 1], 10, 7), 55 % 7);
        // Negative coefficients: tribonacci with alternating sign.
        assert_eq!(
            linrec(&[1, -1, 1], &[1, 1, 1], 6),
            oracle(&[1, -1, 1], &[1, 1, 1], 6)
        );
    }
}
