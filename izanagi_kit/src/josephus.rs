//! The Josephus problem — `n` people in a circle, every
//! `k`-th is eliminated until one survivor remains.
//! `J(n,k)` is computed by the classic `O(n)` recurrence
//! `J₁=0, Jₙ = (Jₙ₋₁ + k) mod n` (survivor index, 0-based);
//! `k = 2` has the closed form `2l` for `n = 2ᵐ + l`,
//! and the full elimination order is produced by an
//! order-statistic tree (select the `k`-th living person,
//! erase, repeat) in `O(n log n)`.
//!
//! ```
//! use izanagi_kit::josephus::{survivor, survivor2, order};
//! // Josephus's own case: n=41, k=3 → survivor 30
//! assert_eq!(survivor(41, 3), Some(30));
//! // n=7, k=2: 2·(7−4) = 6
//! assert_eq!(survivor2(7), Some(6));
//! // elimination order for n=5, k=2: 1,3,0,4 → 2 survives
//! assert_eq!(order(5, 2, 7), Some(vec![1, 3, 0, 4, 2]));
//! ```
//!
//! References: the `J(n,k)` recurrence is classical
//! (Concrete Mathematics §1.3, Graham–Knuth–Patashnik);
//! the `2l` closed form is ibid.; simulating the order via
//! an order-statistic structure is the standard
//! deterministic approach (Library Checker
//! `josephus_problem` write-ups).

/// Survivor index `J(n,k)` (0-based) — `None` for
/// `n == 0 || k == 0`. `O(n)`.
pub fn survivor(n: u32, k: u32) -> Option<u32> {
    if n == 0 || k == 0 {
        return None;
    }
    let mut j = 0u32;
    for i in 2..=n {
        j = (j + k) % i;
    }
    Some(j)
}

/// Survivor for `k = 2` via the closed form
/// `2l` where `n = 2ᵐ + l`, `0 ≤ l < 2ᵐ` — `O(1)`,
/// `None` for `n == 0`.
pub fn survivor2(n: u32) -> Option<u32> {
    if n == 0 {
        return None;
    }
    let pow2 = 1u32 << (31 - n.leading_zeros());
    Some(2 * (n - pow2))
}

/// Full elimination order: the `Vec` returned lists the
/// indices killed at each step in order, so its last
/// element is the survivor. `None` for `n == 0 || k == 0`.
/// `O(n log n)` via [`crate::ost`]; `seed` only shapes
/// the tree, not the result.
pub fn order(n: u32, k: u32, seed: u64) -> Option<Vec<u32>> {
    if n == 0 || k == 0 {
        return None;
    }
    let mut t = crate::ost::Ost::new(seed);
    for i in 0..n {
        t.insert(u64::from(i));
    }
    let mut res = Vec::with_capacity(n as usize);
    let mut pos = 0usize; // rank position of the cursor
    let mut alive = n as usize;
    while alive > 0 {
        pos = (pos + (k as usize - 1)) % alive;
        let victim = t.select(pos)? as u32;
        res.push(victim);
        t.erase(u64::from(victim));
        alive -= 1;
        // cursor stays at `pos`: the next living person
        // after the victim now occupies that rank
    }
    Some(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(survivor(41, 3), Some(30));
        assert_eq!(survivor(1, 7), Some(0));
        assert_eq!(survivor(0, 2), None);
        assert_eq!(survivor(5, 0), None);
        assert_eq!(survivor2(7), Some(6));
        assert_eq!(survivor2(1), Some(0));
        assert_eq!(survivor2(8), Some(0));
        assert_eq!(order(5, 2, 7), Some(vec![1, 3, 0, 4, 2]));
        assert_eq!(order(1, 5, 7), Some(vec![0]));
    }

    /// Brute-force oracle: simulate the circle with a
    /// `Vec` + index stepping — order and survivor must
    /// match for all small (n, k).
    #[test]
    fn brute_oracle() {
        let mut rng = SplitMix64::new(0xB10B);
        for _ in 0..400 {
            let n = 1 + rng.below(40);
            let k = 1 + rng.below(15);
            // brute: keep a list, cursor on next living
            let mut people: Vec<u32> = (0..n).collect();
            let mut ord = Vec::new();
            let mut i = 0usize;
            while !people.is_empty() {
                i = (i + k as usize - 1) % people.len();
                ord.push(people.remove(i));
            }
            let expect_surv = *ord.last().unwrap();
            assert_eq!(survivor(n, k), Some(expect_surv));
            let got = order(n, k, 17).unwrap();
            assert_eq!(got, ord, "order mismatch n={n} k={k}");
            if k == 2 {
                assert_eq!(survivor2(n), Some(expect_surv));
            }
        }
    }

    /// The k=1 edge: nobody is ever skipped — order is
    /// 0,1,…,n−1 and survivor is n−1.
    #[test]
    fn k_is_one() {
        let n = 25u32;
        assert_eq!(survivor(n, 1), Some(n - 1));
        let ord = order(n, 1, 3).unwrap();
        assert_eq!(ord, (0..n).collect::<Vec<_>>());
    }
}
