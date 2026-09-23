//! Batcher's bitonic sorting network — the data-oblivious sort
//! whose comparator sequence depends only on `n`, never on the
//! input. That makes it the right sort for lockstep-critical
//! paths: every peer replays the *identical* compare-exchange
//! trace, and the network itself (`network`) can be shipped as
//! data for a verifier or a sorting gadget. `O(n log² n)`
//! comparators, all integer.
//!
//! `sort` handles arbitrary `n` by conceptually padding to the
//! next power of two with `!0` sentinels (real `!0` values are
//! fine: the first `n` outputs are the `n` smallest either way).
//!
//! ```
//! use izanagi_kit::bitonic::{sort, network};
//! let mut a = [9u64, 1, 7, 3, 5];
//! sort(&mut a);
//! assert_eq!(a, [1, 3, 5, 7, 9]);
//! // The network for n=8 is a fixed comparator list.
//! let net = network(8);
//! assert_eq!(net.len(), 24);
//! ```

/// `(i, j, ascending)` comparator: after applying it, position
/// `i` holds the smaller element when `ascending`, the larger
/// otherwise. `i < j` always.
pub type Comparator = (usize, usize, bool);

/// The full comparator sequence that bitonic-sorts `n` items —
/// a pure function of `n`. Apply in order: `if cmp(a[i],a[j])`
/// fails the direction, swap.
pub fn network(n: usize) -> Vec<Comparator> {
    let mut out = Vec::new();
    let mut k = 2;
    while k <= n && k != 0 {
        let mut j = k >> 1;
        while j > 0 {
            for i in 0..n {
                let p = i ^ j;
                if p > i {
                    out.push((i, p, (i & k) == 0));
                }
            }
            j >>= 1;
        }
        k <<= 1;
    }
    out
}

/// Sort `a` ascending in place via the bitonic network, padded
/// to the next power of two with `!0` sentinels.
pub fn sort(a: &mut [u64]) {
    let n = a.len();
    if n < 2 {
        return;
    }
    let m = n.next_power_of_two();
    let mut v = a.to_vec();
    v.resize(m, !0u64);
    for (i, j, asc) in network(m) {
        let swap = if asc { v[i] > v[j] } else { v[i] < v[j] };
        if swap {
            v.swap(i, j);
        }
    }
    a.copy_from_slice(&v[..n]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn known_nets() {
        // n=2: single comparator; n=4: 6 comparators
        // (2 for the halves + 4 for the merge).
        assert_eq!(network(2), vec![(0, 1, true)]);
        let n4 = network(4);
        assert_eq!(n4.len(), 6);
        // n=8: 24 comparators (k·log² form checks in the oracle test).
        assert_eq!(network(8).len(), 24);
    }

    #[test]
    fn sort_matches_std() {
        let mut rng = SplitMix64::new(99);
        for _ in 0..200 {
            let n = 1 + rng.below(40) as usize;
            let mut a: Vec<u64> = (0..n).map(|_| rng.below(100) as u64).collect();
            // Sometimes include !0u64 so sentinels mix with real data.
            if rng.below(4) == 0 && n > 2 {
                a[n / 2] = !0u64;
            }
            let mut want = a.clone();
            want.sort();
            sort(&mut a);
            assert_eq!(a, want, "n={n}");
        }
    }

    #[test]
    fn network_is_input_independent() {
        // Same comparator list sorts every input of that size —
        // the data-oblivious property the module exists for.
        let net = network(16);
        let mut rng = SplitMix64::new(7);
        for _ in 0..50 {
            let mut a: Vec<u64> = (0..16).map(|_| rng.next_u64() % 1000).collect();
            let mut want = a.clone();
            want.sort();
            for &(i, j, asc) in &net {
                let swap = if asc { a[i] > a[j] } else { a[i] < a[j] };
                if swap {
                    a.swap(i, j);
                }
            }
            assert_eq!(a, want);
        }
    }

    #[test]
    fn multiset_preserved_and_pow2_sizes() {
        let mut rng = SplitMix64::new(5);
        for &n in &[2usize, 4, 8, 16, 32, 64, 128] {
            let mut a: Vec<u64> = (0..n).map(|_| rng.below(7) as u64).collect();
            let before: BTreeMap<u64, usize> = a.iter().fold(BTreeMap::new(), |mut m, &x| {
                *m.entry(x).or_insert(0) += 1;
                m
            });
            sort(&mut a);
            let after: BTreeMap<u64, usize> = a.iter().fold(BTreeMap::new(), |mut m, &x| {
                *m.entry(x).or_insert(0) += 1;
                m
            });
            assert_eq!(before, after);
            assert!(a.windows(2).all(|w| w[0] <= w[1]));
        }
    }
}
