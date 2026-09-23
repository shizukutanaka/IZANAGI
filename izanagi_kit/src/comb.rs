//! Combinatorial number system: exact binomials plus rank/unrank of
//! `k`-subsets in lexicographic order.
//!
//! `choose` counts `C(n, k)` exactly; `rank`/`unrank` map a sorted
//! `k`-subset of `{0..n}` bijectively to `[0, C(n,k))` — the
//! "combinadic" encoding. `next_comb` iterates in place without
//! allocation, `all` materializes the full lexicographic list.
//!
//! All routines are pure functions of `(n, k)` / the subset; overflow
//! is reported (`None`) rather than wrapped.
//!
//! ```
//! use izanagi_kit::comb;
//! assert_eq!(comb::choose(5, 2), Some(10));
//! assert_eq!(comb::unrank(3, 5, 2), Some(vec![0, 4]));
//! assert_eq!(comb::rank(&[0, 4], 5), Some(3));
//! ```

/// `C(n, k)` computed exactly; `None` when the value exceeds `u64` or
/// an intermediate product overflows `u128`.
///
/// Iterates `acc = acc·(n−k+i)/i` for `i = 1..=k`, which is exact at
/// every step because `acc·(n−k+i) = i·C(n−k+i, i)` — the running
/// value equals `C(n−k+i, i)` throughout.
pub fn choose(n: u64, k: u64) -> Option<u64> {
    u64::try_from(choose128(n, k)?).ok()
}

/// `C(n, k)` in `u128`; `Some(0)` for `k > n`, `None` on product
/// overflow (`n` up to 134 is safe in practice).
pub fn choose128(n: u64, k: u64) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut acc: u128 = 1;
    for i in 1..=k {
        acc = acc.checked_mul((n - k + i) as u128)? / i as u128;
    }
    Some(acc)
}

/// Lexicographic rank of the sorted `k`-subset `comb` of `{0..n}`.
///
/// `Some` iff `comb` is strictly increasing with every entry `< n`;
/// satisfies `unrank(rank(c, n), n, c.len()) == c`.
///
/// ```
/// use izanagi_kit::comb;
/// assert_eq!(comb::rank(&[0, 1, 2], 5), Some(0));
/// assert_eq!(comb::rank(&[2, 3, 4], 5), Some(9));
/// ```
pub fn rank(comb: &[u64], n: u64) -> Option<u128> {
    if comb.is_empty() {
        return Some(0);
    }
    let k = comb.len() as u64;
    if k > n || comb[comb.len() - 1] >= n {
        return None;
    }
    let mut r: u128 = 0;
    let mut prev: u64 = 0;
    for (i, &c) in comb.iter().enumerate() {
        if i > 0 && c <= prev {
            return None;
        }
        let lo = if i == 0 { 0 } else { prev + 1 };
        // Every subset whose i-th element is v < c (same prefix) is earlier.
        for v in lo..c {
            r = r.checked_add(choose128(n - v - 1, k - 1 - i as u64)?)?;
        }
        prev = c;
    }
    Some(r)
}

/// The `r`-th `k`-subset of `{0..n}` in lexicographic order; `None`
/// when `r >= C(n, k)` or `k > n`.
///
/// ```
/// use izanagi_kit::comb;
/// assert_eq!(comb::unrank(0, 5, 2), Some(vec![0, 1]));
/// assert_eq!(comb::unrank(9, 5, 2), Some(vec![3, 4]));
/// ```
pub fn unrank(mut r: u128, n: u64, k: u64) -> Option<Vec<u64>> {
    if k > n {
        return None;
    }
    let mut out = Vec::with_capacity(k as usize);
    let mut start: u64 = 0;
    for i in 0..k {
        let rem = k - 1 - i;
        let mut v = start;
        loop {
            // Block of subsets sharing the current prefix with v next.
            let block = choose128(n.checked_sub(v)?.checked_sub(1)?, rem)?;
            if block == 0 || v >= n {
                return None;
            }
            if r >= block {
                r -= block;
                v += 1;
            } else {
                break;
            }
        }
        out.push(v);
        start = v + 1;
    }
    Some(out)
}

/// Advance `comb` to the next `k`-subset of `{0..n}` lexicographically.
/// Returns `false` when `comb` was already the last subset (contents
/// stay valid and sorted).
///
/// ```
/// use izanagi_kit::comb;
/// let mut c = vec![0, 1];
/// assert!(comb::next_comb(&mut c, 4));
/// assert_eq!(c, vec![0, 2]);
/// ```
pub fn next_comb(comb: &mut [u64], n: u64) -> bool {
    let k = comb.len() as u64;
    if k == 0 || k > n {
        return false;
    }
    // Rightmost position that can still grow: comb[idx] < n − (k − idx).
    let mut i = k;
    while i > 0 {
        let idx = (i - 1) as usize;
        if comb[idx] < n - k + idx as u64 {
            comb[idx] += 1;
            for j in idx + 1..comb.len() {
                comb[j] = comb[j - 1] + 1;
            }
            return true;
        }
        i -= 1;
    }
    false
}

/// All `k`-subsets of `{0..n}` in lexicographic order; `None` for
/// `k > n` or when `C(n,k)` overflows `u64`.
pub fn all(n: u64, k: u64) -> Option<Vec<Vec<u64>>> {
    let total = choose(n, k)? as usize;
    if k > n {
        return None;
    }
    let mut out = Vec::with_capacity(total.min(1 << 22));
    if k == 0 {
        out.push(Vec::new());
        return Some(out);
    }
    let mut c: Vec<u64> = (0..k).collect();
    loop {
        out.push(c.clone());
        if !next_comb(&mut c, n) {
            break;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_basics() {
        assert_eq!(choose(0, 0), Some(1));
        assert_eq!(choose(5, 0), Some(1));
        assert_eq!(choose(5, 5), Some(1));
        assert_eq!(choose(5, 2), Some(10));
        assert_eq!(choose(10, 3), Some(120));
        assert_eq!(choose(67, 33), Some(14226520737620288370));
        assert_eq!(choose(6, 7), Some(0));
        assert_eq!(choose(100, 50), None); // > u64::MAX
                                           // Pascal identity across a sweep of the triangle.
        for n in 1..20 {
            for k in 0..=n {
                let lhs = choose(n, k).unwrap() as u128;
                let rhs = choose128(n - 1, k).unwrap()
                    + if k == 0 {
                        0
                    } else {
                        choose128(n - 1, k - 1).unwrap()
                    };
                assert_eq!(lhs, rhs, "n={n} k={k}");
            }
        }
    }

    #[test]
    fn rank_unrank_round_trip_exhaustive() {
        for n in 0..=8u64 {
            for k in 0..=n {
                let subs = all(n, k).unwrap();
                assert_eq!(subs.len() as u64, choose(n, k).unwrap());
                for (r, c) in subs.iter().enumerate() {
                    assert_eq!(rank(c, n), Some(r as u128), "n={n} k={k} c={c:?}");
                    assert_eq!(unrank(r as u128, n, k).as_deref(), Some(&c[..]));
                }
            }
        }
    }

    #[test]
    fn unrank_rejects_out_of_range() {
        assert_eq!(unrank(10, 5, 2), None);
        assert_eq!(unrank(0, 4, 5), None);
        assert_eq!(unrank(0, 0, 0), Some(vec![]));
    }

    #[test]
    fn rank_rejects_malformed() {
        assert_eq!(rank(&[2, 1], 5), None);
        assert_eq!(rank(&[1, 1], 5), None);
        assert_eq!(rank(&[5], 5), None);
        assert_eq!(rank(&[1, 2, 3, 4, 5, 6], 5), None);
        assert_eq!(rank(&[], 5), Some(0));
    }

    #[test]
    fn next_comb_walks_all_then_stops() {
        let mut c = vec![0, 1, 2];
        let mut count = 1u64;
        while next_comb(&mut c, 6) {
            count += 1;
        }
        assert_eq!(count, 20);
        assert_eq!(c, vec![3, 4, 5]);
        assert!(!next_comb(&mut c, 6));
    }

    #[test]
    fn all_matches_lexicographic_oracle() {
        let subs = all(6, 3).unwrap();
        assert_eq!(subs.len(), 20);
        assert_eq!(subs[0], vec![0, 1, 2]);
        assert_eq!(subs[19], vec![3, 4, 5]);
        for w in subs.windows(2) {
            assert!(w[0] < w[1]);
        }
        // Deterministic.
        assert_eq!(subs, all(6, 3).unwrap());
    }
}
