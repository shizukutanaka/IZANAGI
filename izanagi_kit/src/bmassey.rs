//! Berlekamp–Massey — shortest linear feedback shift
//! register (minimal linear recurrence) over GF(p) in
//! `O(n²)`. The dual of [`crate::linrec`]: linrec jumps a
//! known recurrence to its k-th term; this *discovers* the
//! recurrence from observed terms — seed recovery, code
//! identification, checksum reversal.
//!
//! Convention: returns `C` with `C[0] == 1`, length `L+1`,
//! such that for every `n ≥ L`,
//! `Σ_{j=0..=L} C[j]·s[n−j] ≡ 0 (mod p)` — i.e. the
//! recurrence `s[n] = −Σ_{j=1..=L} C[j]·s[n−j]`.
//!
//! `p` must be prime (field arithmetic relies on Fermat
//! inversion); every entry of `s` is taken `mod p`.
//!
//! ```
//! use izanagi_kit::bmassey::massey;
//! // Fibonacci mod 7: s[n] = s[n-1] + s[n-2]
//! let s = [0u64, 1, 1, 2, 3, 5, 8 % 7, 13 % 7, 21 % 7, 34 % 7];
//! let c = massey(&s, 7);
//! assert_eq!(c, vec![1, 6, 6]); // 1, −1, −1 (mod 7)
//! ```

fn mul(a: u64, b: u64, p: u64) -> u64 {
    (a as u128 * b as u128 % p as u128) as u64
}

fn pow_mod(mut a: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1u64;
    a %= p;
    while e > 0 {
        if e & 1 == 1 {
            r = mul(r, a, p);
        }
        a = mul(a, a, p);
        e >>= 1;
    }
    r
}

/// Shortest linear recurrence `C` over GF(p) for `s` —
/// `C[0] == 1`, `C.len() == L+1` where `L` is the linear
/// complexity (LFSR length). Empty `s` → `vec![1]`.
pub fn massey(s: &[u64], p: u64) -> Vec<u64> {
    let n = s.len();
    let s: Vec<u64> = s.iter().map(|&x| x % p).collect();
    let mut c = vec![0u64; n + 1];
    let mut b = vec![0u64; n + 1];
    c[0] = 1;
    b[0] = 1;
    let (mut l, mut m, mut bb) = (0usize, 1usize, 1u64);
    for i in 0..n {
        // discrepancy d = s[i] + Σ_{j=1..L} C[j]·s[i−j]
        let mut d = s[i];
        for j in 1..=l {
            d = (d + mul(c[j], s[i - j], p)) % p;
        }
        if d == 0 {
            m += 1;
            continue;
        }
        let coef = mul(d, pow_mod(bb, p - 2, p), p);
        let t = c.clone();
        // C −= coef·x^m·B
        for (j, &bj) in b.iter().enumerate().take(n - m + 1) {
            if bj != 0 {
                let v = mul(coef, bj, p);
                let k = j + m;
                c[k] = (c[k] + p - v) % p;
            }
        }
        if 2 * l <= i {
            l = i + 1 - l;
            b = t;
            bb = d;
            m = 1;
        } else {
            m += 1;
        }
    }
    c.truncate(l + 1);
    c
}

/// Check `C` is a valid recurrence for `s` over GF(p) —
/// every position past `L` satisfies the zero sum. Useful
/// for verifying recovered connections.
pub fn holds(s: &[u64], c: &[u64], p: u64) -> bool {
    if c.is_empty() {
        return false;
    }
    let l = c.len() - 1;
    let s: Vec<u64> = s.iter().map(|&x| x % p).collect();
    for i in l..s.len() {
        let mut acc = 0u64;
        for j in 0..=l {
            acc = (acc + mul(c[j], s[i - j], p)) % p;
        }
        if acc != 0 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute minimal linear complexity: try every feedback
    /// vector r ∈ GF(p)^L, L ascending — the recurrence
    /// s[n] = Σ r[j]·s[n−1−j] must hold at every position.
    fn min_complexity(s: &[u64], p: u64, l_cap: usize) -> usize {
        let s: Vec<u64> = s.iter().map(|&x| x % p).collect();
        for l in 0..=l_cap.min(s.len() / 2) {
            let mut r = vec![0u64; l];
            loop {
                let ok = (l..s.len()).all(|i| {
                    let mut acc = 0u64;
                    for j in 0..l {
                        acc = (acc + mul(r[j], s[i - 1 - j], p)) % p;
                    }
                    acc == s[i]
                });
                if ok {
                    return l;
                }
                // increment r as a p-ary counter; overflow
                // at digit l means every r for this L failed
                let mut k = 0;
                loop {
                    if k == l {
                        break;
                    }
                    r[k] = (r[k] + 1) % p;
                    if r[k] != 0 {
                        break;
                    }
                    k += 1;
                }
                if k == l {
                    break;
                }
            }
        }
        // nothing fit within cap — complexity exceeds it
        l_cap.min(s.len() / 2) + 1
    }

    /// Feed-forward oracle: generate terms n.. via C, check
    /// against the tail of s — independent of `holds`.
    fn predicts(s: &[u64], c: &[u64], p: u64) -> bool {
        let l = c.len() - 1;
        for i in l..s.len() {
            let mut acc = 0u64;
            for j in 1..=l {
                acc = (acc + mul(c[j], s[i - j], p)) % p;
            }
            acc = (p - acc) % p;
            if acc != s[i] % p {
                return false;
            }
        }
        true
    }

    #[test]
    fn known_sequences() {
        // Fibonacci mod 7
        let s = [0u64, 1, 1, 2, 3, 5, 8 % 7, 13 % 7, 21 % 7, 34 % 7];
        assert_eq!(massey(&s, 7), vec![1, 6, 6]);
        // constant sequence → L=1
        assert_eq!(massey(&[3, 3, 3, 3], 7), vec![1, 6]);
        // all zeros → L=0 (empty recurrence C=[1])
        assert_eq!(massey(&[0, 0, 0], 7), vec![1]);
        // empty input
        assert_eq!(massey(&[], 7), vec![1]);
        // geometric s[n] = 2^n mod 7 → L=1, C=[1,−2]=[1,5]
        let s: Vec<u64> = (0..8).map(|i| pow_mod(2, i, 7)).collect();
        assert_eq!(massey(&s, 7), vec![1, 5]);
        // quadratic non-recurring-ish data still yields *a*
        // recurrence that holds — valid by construction
        let s = [1u64, 4, 9, 16 % 7, 25 % 7, 36 % 7, 49 % 7];
        let c = massey(&s, 7);
        assert!(holds(&s, &c, 7) && predicts(&s, &c, 7));
    }

    #[test]
    fn oracle_brute_minimal() {
        let mut rng = SplitMix64::new(71);
        for _round in 0..400 {
            let p = [3u64, 5, 7][rng.below(3) as usize];
            let n = (rng.below(8) + 4) as usize;
            // build a sequence FROM a random small LFSR so the
            // true complexity is bounded
            let l_true = (rng.below(2) + 1) as usize;
            let r: Vec<u64> = (0..l_true).map(|_| rng.below(p as u32) as u64).collect();
            let mut s: Vec<u64> = (0..l_true).map(|_| rng.below(p as u32) as u64).collect();
            for _ in l_true..n {
                let mut acc = 0u64;
                for j in 0..l_true {
                    acc = (acc + mul(r[j], s[s.len() - 1 - j], p)) % p;
                }
                s.push(acc);
            }
            let c = massey(&s, p);
            assert_eq!(c[0], 1, "round {_round} leading coeff");
            assert!(
                holds(&s, &c, p),
                "round {_round} holds s={s:?} p={p} c={c:?}"
            );
            assert!(predicts(&s, &c, p), "round {_round} predicts");
            let want = min_complexity(&s, p, 3);
            assert_eq!(
                c.len() - 1,
                want,
                "round {_round} complexity s={s:?} c={c:?} want={want}"
            );
        }
    }

    #[test]
    fn large_prime() {
        // 998244353 — generated LFSR of length 4 recovered
        let p = 998_244_353u64;
        let r = [7u64, 11, 13, 17];
        let mut s = vec![1u64, 2, 3, 4];
        for _ in 0..12 {
            let mut acc = 0u64;
            for (j, &rj) in r.iter().enumerate() {
                acc = (acc + mul(rj, s[s.len() - 1 - j], p)) % p;
            }
            s.push(acc);
        }
        let c = massey(&s, p);
        assert_eq!(c.len(), 5);
        // implied feedback r' must equal r: s[n] = −Σ C[j]s[n−j]
        for (j, &rj) in r.iter().enumerate() {
            assert_eq!((p - c[j + 1]) % p, rj, "coef {j}");
        }
    }
}
