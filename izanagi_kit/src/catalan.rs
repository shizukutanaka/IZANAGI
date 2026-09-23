//! Catalan numbers and Dyck paths — `Cₙ` counts balanced
//! parentheses words (and ~150 other objects); computed
//! exactly over [`BigInt`] by the always-integral
//! recurrence `Cᵢ = Cᵢ₋₁·(4i−2)/(i+1)`, plus the ballot
//! numbers `B(a,b) = (a−b)/(a+b)·C(a+b,a)` and an
//! enumerator for the paths themselves.
//!
//! ```
//! use izanagi_kit::catalan::{catalan, binomial, ballot, dyck};
//! use izanagi_kit::bigint::BigInt;
//! assert_eq!(catalan(5), BigInt::from_i64(42));
//! assert_eq!(binomial(10, 4), BigInt::from_i64(210));
//! // C₃ = 5 Dyck words
//! assert_eq!(dyck(3).len(), 5);
//! // ballot 3 vs 2: (3−2)/(3+2)·C(5,3) = 2
//! assert_eq!(ballot(3, 2), BigInt::from_i64(2));
//! ```
//!
//! References: the `Cᵢ = Cᵢ₋₁(4i−2)/(i+1)` chain is the
//! standard exact-arithmetic implementation (each prefix
//! product is integral — a classic lemma); the ballot
//! numbers are the classical Bertrand (1887) result
//! `B(a,b) = (a−b)·C(a+b,a)/(a+b)`; Dyck-word generation
//! is textbook backtracking (Concrete Mathematics §7.5).

use crate::bigint::BigInt;

/// Exact scalar division `b / d` — `b` must be divisible
/// by `d` (school-method on little-endian limbs; the
/// remainder is asserted by construction sites, never
/// silently dropped).
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

/// `C(n, k)` over [`BigInt`] — the running product is an
/// integer after every step (`C(n,i)` itself), so the
/// chain needs no fractions.
pub fn binomial(n: u32, k: u32) -> BigInt {
    if k > n {
        return BigInt::zero();
    }
    let k = k.min(n - k);
    let mut res = BigInt::from_i64(1);
    for i in 1..=k {
        res = res.mul(&BigInt::from_i64((n + 1 - i) as i64));
        res = div_u64(&res, u64::from(i));
    }
    res
}

/// The `n`-th Catalan number `Cₙ` — `C₀ = C₁ = 1`,
/// `C₁₀ = 16796`, `C₃₀ ≈ 3.8e15`.
pub fn catalan(n: u32) -> BigInt {
    let mut c = BigInt::from_i64(1);
    for i in 1..=n {
        c = c.mul(&BigInt::from_i64(4 * i as i64 - 2));
        c = div_u64(&c, u64::from(i + 1));
    }
    c
}

/// Ballot number `B(a,b)` — lattice paths from `(0,0)`
/// to `(a,b)` with `a ≥ b` that stay strictly above the
/// diagonal after the start: `(a−b)/(a+b)·C(a+b,a)`.
/// `0` when `a < b` or `a + b == 0`; `B(a,a) = 0`.
pub fn ballot(a: u32, b: u32) -> BigInt {
    if a <= b {
        return BigInt::zero();
    }
    let top = binomial(a + b, a);
    let b_times = top.mul(&BigInt::from_i64((a - b) as i64));
    div_u64(&b_times, u64::from(a + b))
}

/// All Dyck words of semilength `n` — strings of `n`
/// opens and `n` closes that never close more than they
/// opened, lexicographic with `'(' < ')'`. Exponential
/// output (`Cₙ` words of length `2n`); keep `n ≤ 8`.
pub fn dyck(n: u32) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::with_capacity(2 * n as usize);
    let n = n as usize;
    fn rec(open: usize, close: usize, n: usize, cur: &mut String, out: &mut Vec<String>) {
        if cur.len() == 2 * n {
            out.push(cur.clone());
            return;
        }
        if open < n {
            cur.push('(');
            rec(open + 1, close, n, cur, out);
            cur.pop();
        }
        if close < open {
            cur.push(')');
            rec(open, close + 1, n, cur, out);
            cur.pop();
        }
    }
    rec(0, 0, n, &mut cur, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let cat: Vec<i64> = vec![1, 1, 2, 5, 14, 42, 132, 429, 1430, 4862, 16796];
        for (n, &c) in cat.iter().enumerate() {
            assert_eq!(catalan(n as u32), BigInt::from_i64(c), "C_{n}");
        }
        assert_eq!(binomial(10, 4), BigInt::from_i64(210));
        assert_eq!(binomial(10, 0), BigInt::from_i64(1));
        assert_eq!(binomial(4, 5), BigInt::zero());
        assert_eq!(ballot(3, 2), BigInt::from_i64(2));
        assert_eq!(ballot(2, 3), BigInt::zero());
        assert_eq!(ballot(4, 4), BigInt::zero());
        assert_eq!(dyck(3).len(), 5);
        assert_eq!(dyck(0), vec![String::new()]);
    }

    /// Every returned word is a real Dyck word (never
    /// dips below zero, balanced, length `2n`), and the
    /// count equals `Cₙ` — count vs content oracle.
    #[test]
    fn dyck_words_are_valid() {
        for n in 0..7u32 {
            let ws = dyck(n);
            let expect = catalan(n).to_i128().unwrap() as usize;
            assert_eq!(ws.len(), expect, "n={n}");
            let mut seen = std::collections::BTreeSet::new();
            for w in &ws {
                assert!(seen.insert(w), "duplicate word {w}");
                assert_eq!(w.len(), 2 * n as usize);
                let mut bal = 0i64;
                for ch in w.bytes() {
                    bal += if ch == b'(' { 1 } else { -1 };
                    assert!(bal >= 0, "{w} dips below 0");
                }
                assert_eq!(bal, 0, "{w} not balanced");
            }
        }
    }

    /// Brute-force oracle: enumerate all `2^{2n}` words
    /// and filter the Dyck ones — must equal `dyck(n)`.
    #[test]
    fn brute_oracle() {
        let mut rng = SplitMix64::new(0xCA7A);
        for _ in 0..30 {
            let n = 1 + rng.below(4);
            let n = n as usize;
            let mut expect = Vec::new();
            for mask in 0..(1u64 << (2 * n)) {
                let mut w = String::new();
                let mut bal = 0i64;
                let mut ok = true;
                for i in 0..2 * n {
                    if mask >> i & 1 == 1 {
                        w.push('(');
                        bal += 1;
                    } else {
                        w.push(')');
                        bal -= 1;
                        if bal < 0 {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok && bal == 0 {
                    expect.push(w);
                }
            }
            expect.sort();
            let mut got = dyck(n as u32);
            got.sort();
            assert_eq!(got, expect, "n={n}");
        }
    }

    /// Ballot brute oracle: enumerate all `C(a+b,a)`
    /// lattice paths, count those staying strictly above
    /// `y = (b/a)·x`-style — i.e. always more a-steps used
    /// than b-steps used at every prefix.
    #[test]
    fn ballot_oracle() {
        for a in 1..7u32 {
            for b in 0..a {
                let mut count = 0u64;
                // enumerate choose positions of a 'a's among a+b
                let total = (a + b) as usize;
                for mask in 0..(1u64 << total) {
                    if mask.count_ones() != a {
                        continue;
                    }
                    let mut ca = 0i64;
                    let mut cb = 0i64;
                    let mut ok = true;
                    for i in 0..total {
                        if mask >> i & 1 == 1 {
                            ca += 1;
                        } else {
                            cb += 1;
                            if cb >= ca {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if ok {
                        count += 1;
                    }
                }
                assert_eq!(ballot(a, b), BigInt::from_i64(count as i64), "a={a} b={b}");
            }
        }
    }
}
