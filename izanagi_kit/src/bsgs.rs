//! Baby-step giant-step — discrete logarithm `g^x ≡ h (mod p)`
//! in `O(√p)` time and memory, deterministic.
//!
//! Write `x = i·m + j` with `m = ⌈√(p−1)⌉`. A hash-table of
//! baby steps `g^j ↦ j` (minimum `j` kept) is built once; then
//! `h·(g^{−m})^i` is walked for `i = 0, 1, …` — the first hit
//! is the least `x`, because every decomposition below `i·m`
//! has already been ruled out. The giant factor is
//! `g^{p−1−m}` (Fermat: `g^{p−1} ≡ 1`), so no inverse-modulo
//! helper is needed.
//!
//! `p` is required to be prime; `g = 0` and `g = 1` fall out
//! of the same walk naturally (`0^0` is treated as 1, so
//! `discrete_log(0, 1, p) = 0`).
//!
//! ```
//! use izanagi_kit::bsgs::discrete_log;
//! assert_eq!(discrete_log(2, 8, 13), Some(3)); // 2³ = 8
//! assert_eq!(discrete_log(2, 6, 13), Some(5)); // 2⁵ = 32 ≡ 6
//! assert_eq!(discrete_log(2, 7, 13), Some(11));
//! ```
//!
//! References: Shanks (1971), cp-algorithms "Discrete Log".

use std::collections::BTreeMap;

/// Modular multiply — `a·b mod p` in `u128` (`p < 2^63`).
fn mul_mod(a: u64, b: u64, p: u64) -> u64 {
    ((a as u128 * b as u128) % p as u128) as u64
}

fn pow_mod(mut a: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1u64 % p;
    a %= p;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, a, p);
        }
        a = mul_mod(a, a, p);
        e >>= 1;
    }
    r
}

fn isqrt(n: u64) -> u64 {
    let mut lo = 0u64;
    let mut hi = n.saturating_add(1);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if mid <= n / mid {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Least `x ≥ 0` with `g^x ≡ h (mod p)`, `p` prime — `None`
/// when `h` lies outside the subgroup `⟨g⟩`. `O(√p)`.
pub fn discrete_log(g: u64, h: u64, p: u64) -> Option<u64> {
    if p == 1 {
        return None;
    }
    let (g, h) = (g % p, h % p);
    if g == 0 {
        // f = g^{p−1−m} is a real inverse only when g is
        // invertible — the walk would hit `0` spuriously at
        // i ≥ 1, so resolve the zero base directly
        return match h {
            1 => Some(0),
            0 => Some(1),
            _ => None,
        };
    }
    if h == 1 || g == 1 {
        return if h == 1 { Some(0) } else { None };
    }
    // order of g divides p − 1, so √p−1 giant steps span it
    let m = isqrt(p - 1) + 1;
    let mut baby = BTreeMap::new();
    let mut e = 1u64;
    for j in 0..m {
        // first j wins — minimal exponent for this residue
        baby.entry(e).or_insert(j);
        e = mul_mod(e, g, p);
    }
    // g^{-m} ≡ g^{p−1−m} (Fermat) — valid for m ≤ p−1
    let f = if m < p { pow_mod(g, p - 1 - m, p) } else { 0 };
    let mut cur = h;
    for i in 0..=m {
        if let Some(&j) = baby.get(&cur) {
            return Some(i * m + j);
        }
        cur = mul_mod(cur, f, p);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute_log(g: u64, h: u64, p: u64) -> Option<u64> {
        let (g, h) = (g % p, h % p);
        let mut e = 1u64;
        for x in 0..p {
            if e == h {
                return Some(x);
            }
            e = mul_mod(e, g, p);
        }
        None
    }

    fn is_prime(p: u64) -> bool {
        if p < 2 {
            return false;
        }
        let mut d = 2;
        while d * d <= p {
            if p % d == 0 {
                return false;
            }
            d += 1;
        }
        true
    }

    #[test]
    fn known_vectors() {
        assert_eq!(discrete_log(2, 8, 13), Some(3));
        assert_eq!(discrete_log(2, 6, 13), Some(5));
        assert_eq!(discrete_log(2, 1, 13), Some(0));
        assert_eq!(discrete_log(3, 7, 5), Some(3)); // 27≡2? 3³=27≡2 — check: 3^1=3,3^2=4,3^3=2,3^4=1 → 7≡2? 7%5=2 → 3
                                                    // g = 0 and g = 1 edges
        assert_eq!(discrete_log(0, 1, 7), Some(0));
        assert_eq!(discrete_log(0, 0, 7), Some(1));
        assert_eq!(discrete_log(0, 3, 7), None);
        assert_eq!(discrete_log(1, 1, 7), Some(0));
        assert_eq!(discrete_log(1, 5, 7), None);
        // p = 2: group = {1}
        assert_eq!(discrete_log(1, 1, 2), Some(0));
        assert_eq!(discrete_log(0, 0, 2), Some(1));
    }

    #[test]
    fn brute_oracle_small_primes() {
        for p in 2..200u64 {
            if !is_prime(p) {
                continue;
            }
            for g in 0..p.min(20) {
                for h in 0..p {
                    assert_eq!(
                        discrete_log(g, h, p),
                        brute_log(g, h, p),
                        "g={g} h={h} p={p}"
                    );
                }
            }
        }
    }

    #[test]
    fn roundtrip_larger_primes() {
        let mut rng = SplitMix64::new(37);
        for &p in &[1009u64, 10007, 99991, 65537, 998244353, 4294967291] {
            for _ in 0..120 {
                let g = rng.next_u64() % p;
                let x = rng.next_u64() % (p - 1);
                let h = pow_mod(g, x, p);
                match discrete_log(g, h, p) {
                    Some(y) => {
                        // least representative — y ≤ x, same class
                        assert!(y <= x, "x={x} y={y} g={g} p={p}");
                        assert_eq!(pow_mod(g, y, p), h);
                    }
                    None => panic!("g={g} x={x} h={h} p={p} must have a log"),
                }
            }
        }
    }

    #[test]
    fn no_log_when_outside_subgroup() {
        // p = 23, g = 2 has order 11 — subgroup
        // {1,2,3,4,6,8,9,12,13,16,18}; 5 is outside
        assert_eq!(discrete_log(2, 5, 23), brute_log(2, 5, 23));
        assert_eq!(discrete_log(2, 5, 23), None);
        assert_eq!(discrete_log(2, 6, 23), Some(9)); // 2⁹=512≡6
    }
}
