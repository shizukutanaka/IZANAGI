//! The coin (Frobenius) problem — given coin denominations
//! `c₁,…,cₖ` with `gcd = 1`, which totals are
//! representable as `Σ aᵢ·cᵢ` with `aᵢ ≥ 0`? The largest
//! unrepresentable total is the *Frobenius number*
//! `g(c)`: `a·b − a − b` for two coins; for more it needs
//! the residue-class Dijkstra over `min(coins)`.
//!
//! `dist[r]` = the smallest representable value
//! `≡ r (mod m)` — then `x` is representable iff
//! `dist[x mod m] ≤ x`, and `g = max_r dist[r] − m`.
//!
//! ```
//! use izanagi_kit::frobenius::{frobenius, representable};
//! // coins 3, 5: g = 3·5 − 3 − 5 = 7
//! assert_eq!(frobenius(&[3, 5]), Some(7));
//! assert!(representable(&[3, 5], 8).unwrap());
//! assert!(!representable(&[3, 5], 7).unwrap());
//! // McNugget numbers: g(6,9,20) = 43
//! assert_eq!(frobenius(&[6, 9, 20]), Some(43));
//! ```
//!
//! References: the min-residue Dijkstra is the standard
//! competitive-programming construction (cf. Qiita/Zenn
//! coin-problem write-ups); `g(a,b) = ab − a − b` is
//! Sylvester (1882); the `dist[x mod m] ≤ x`
//! representability test is the classical
//! Brauer/Davison criterion.

/// Smallest representable value for each residue mod
/// `m = min(coins)` — `None` when the input is empty,
/// contains a `0`, or has `gcd > 1` (the Frobenius number
/// doesn't exist). `dist[0] = 0`.
fn residue_dists(coins: &[u64]) -> Option<Vec<u64>> {
    let m = *coins.iter().min()?;
    if m == 0
        || coins
            .iter()
            .fold(0u64, |g, &c| crate::ntheory::gcd(g as i64, c as i64) as u64)
            != 1
    {
        return None;
    }
    let m = m as usize;
    let mut dist = vec![u64::MAX; m];
    dist[0] = 0;
    // Dijkstra over m residue classes, edge weight c
    let mut heap = std::collections::BinaryHeap::new();
    heap.push(std::cmp::Reverse((0u64, 0usize)));
    while let Some(std::cmp::Reverse((d, r))) = heap.pop() {
        if d > dist[r] {
            continue;
        }
        for &c in coins {
            let nr = ((d + c) % m as u64) as usize;
            let nd = d + c;
            if nd < dist[nr] {
                dist[nr] = nd;
                heap.push(std::cmp::Reverse((nd, nr)));
            }
        }
    }
    Some(dist)
}

/// The Frobenius number `g(c)` — the largest
/// unrepresentable total, `Some(-1 as u64)`-style: for
/// `coins = [1]` every total is representable so we
/// return `Some(u64::MAX)` as "none exists". `None` for
/// invalid input (empty, zero, `gcd > 1`).
pub fn frobenius(coins: &[u64]) -> Option<u64> {
    let dist = residue_dists(coins)?;
    let m = *coins.iter().min()? as usize;
    if m == 1 {
        return Some(u64::MAX); // sentinel: no non-representable total
    }
    let g = dist.iter().copied().max()? - m as u64;
    Some(g)
}

/// Is `x` representable as `Σ aᵢ·cᵢ` (`aᵢ ≥ 0`)?
/// `None` for invalid input — see [`frobenius`].
pub fn representable(coins: &[u64], x: u64) -> Option<bool> {
    let dist = residue_dists(coins)?;
    let m = *coins.iter().min()? as usize;
    Some(dist[(x % m as u64) as usize] <= x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(frobenius(&[3, 5]), Some(7));
        assert_eq!(frobenius(&[6, 9, 20]), Some(43)); // McNuggets
        assert_eq!(frobenius(&[4, 6]), None); // gcd 2 — unbounded gaps
        assert_eq!(frobenius(&[]), None);
        assert_eq!(frobenius(&[0, 5]), None);
        assert_eq!(representable(&[3, 5], 8), Some(true));
        assert_eq!(representable(&[3, 5], 7), Some(false));
        assert_eq!(representable(&[6, 9, 20], 43), Some(false));
        assert_eq!(representable(&[6, 9, 20], 44), Some(true));
    }

    /// Sylvester closed form for k=2 + brute-force
    /// representability oracle (DP up to bound) for
    /// small coprime coin sets.
    #[test]
    fn sylvester_and_oracle() {
        let mut rng = SplitMix64::new(0xF70B);
        // two coprime coins: g = ab − a − b
        for _ in 0..300 {
            let a = 2 + u64::from(rng.below(40));
            let b = 2 + u64::from(rng.below(40));
            if crate::ntheory::gcd(a as i64, b as i64) != 1 {
                continue;
            }
            assert_eq!(frobenius(&[a, b]), Some(a * b - a - b));
        }
        // DP representability oracle
        fn check(coins: &[u64], x: u64) -> bool {
            let mut dp = vec![false; x as usize + 1];
            dp[0] = true;
            for v in 0..=x as usize {
                if !dp[v] {
                    continue;
                }
                for &c in coins {
                    if v + c as usize <= x as usize {
                        dp[v + c as usize] = true;
                    }
                }
            }
            dp[x as usize]
        }
        for _ in 0..2000 {
            let n = 2 + rng.below(3) as usize;
            let mut coins: Vec<u64> = (0..n).map(|_| 1 + u64::from(rng.below(20))).collect();
            coins.sort_unstable();
            coins.dedup();
            if coins.len() < 2 {
                continue;
            }
            if coins
                .iter()
                .fold(0u64, |g, &c| crate::ntheory::gcd(g as i64, c as i64) as u64)
                != 1
            {
                continue;
            }
            let x = u64::from(rng.below(400));
            assert_eq!(representable(&coins, x), Some(check(&coins, x)));
        }
        // frobenius oracle: above g, everything is
        // representable; g itself is not
        let coins = [6u64, 10, 15];
        let g = frobenius(&coins).unwrap();
        assert_eq!(g, 29);
        for x in g + 1..g + 30 {
            assert!(check(&coins, x), "x = {x} must be representable");
            assert_eq!(representable(&coins, x), Some(true));
        }
        assert_eq!(representable(&coins, g), Some(false));
    }
}
