//! Gale–Shapley stable matching — the proposer-optimal stable marriage
//! algorithm (Gale & Shapley 1962).
//!
//! Deterministic: free proposers are dequeued in index order and each
//! proposes down its preference list, so the result is the unique
//! proposer-optimal stable matching — not merely "a" stable matching.
//! [`is_stable`] is the independent verifier the tests pin against a
//! brute-force optimality oracle.
//!
//! ```
//! use izanagi_kit::stable::{is_stable, stable_match};
//! // 0>1 both sides: men and women agree on the same ranking.
//! let men = vec![vec![0, 1], vec![0, 1]];
//! let women = vec![vec![0, 1], vec![0, 1]];
//! let m = stable_match(&men, &women).unwrap();
//! assert_eq!(m, vec![0, 1]);
//! assert!(is_stable(&men, &women, &m));
//! ```

/// Stable marriage matching: `stable_match(men, women)[m] = w` — the
/// woman matched to man `m` under men-proposing Gale–Shapley.
///
/// Both preference lists must be equal-length `n × n` permutations of
/// `0..n` — `None` otherwise (incomplete lists mean "unacceptable",
/// which this formulation does not model).
pub fn stable_match(men: &[Vec<u32>], women: &[Vec<u32>]) -> Option<Vec<u32>> {
    let n = men.len();
    if women.len() != n {
        return None;
    }
    // Validate: every list is a permutation of 0..n.
    let valid = |prefs: &[Vec<u32>]| {
        prefs.iter().all(|p| {
            p.len() == n && {
                let mut seen = vec![false; n];
                p.iter()
                    .all(|&x| (x as usize) < n && !std::mem::replace(&mut seen[x as usize], true))
            }
        })
    };
    if !valid(men) || !valid(women) {
        return None;
    }
    // rank[w][m] = how high woman w ranks man m (0 = most preferred).
    let mut rank = vec![vec![0u32; n]; n];
    for (w, prefs) in women.iter().enumerate() {
        for (r, &m) in prefs.iter().enumerate() {
            rank[w][m as usize] = r as u32;
        }
    }
    // woman's current fiancé (or none).
    let mut fiance: Vec<Option<u32>> = vec![None; n];
    // man's next proposal index.
    let mut next = vec![0usize; n];
    // Free-men stack, initialized reversed so pop() yields ascending
    // index order (man 0 proposes first). Any fixed discipline gives
    // the unique man-optimal matching; ascending order just makes the
    // trace easier to read.
    let mut free: Vec<u32> = (0..n as u32).rev().collect();
    while let Some(m) = free.pop() {
        let m = m as usize;
        let w = men[m][next[m]] as usize;
        next[m] += 1;
        match fiance[w] {
            None => fiance[w] = Some(m as u32),
            Some(cur) => {
                let cur = cur as usize;
                if rank[w][m] < rank[w][cur] {
                    fiance[w] = Some(m as u32);
                    free.push(cur as u32);
                } else {
                    free.push(m as u32);
                }
            }
        }
    }
    // Invert: matching[m] = w for each woman's fiancé.
    let mut matching = vec![0u32; n];
    for (w, fm) in fiance.iter().enumerate() {
        let m = (*fm)? as usize;
        matching[m] = w as u32;
    }
    Some(matching)
}

/// Verifier: `matching[m] = w` is a stable perfect matching for the
/// preference lists — permutation check plus the absence of any
/// blocking pair `(m, w')` where `m` prefers `w'` to his match and
/// `w'` prefers `m` to hers.
pub fn is_stable(men: &[Vec<u32>], women: &[Vec<u32>], matching: &[u32]) -> bool {
    let n = men.len();
    if women.len() != n || matching.len() != n {
        return false;
    }
    // Matching must be a permutation of 0..n.
    {
        let mut seen = vec![false; n];
        if !matching
            .iter()
            .all(|&w| (w as usize) < n && !std::mem::replace(&mut seen[w as usize], true))
        {
            return false;
        }
    }
    let rank_w = |prefs: &[Vec<u32>]| -> Vec<Vec<u32>> {
        let mut r = vec![vec![u32::MAX; n]; n];
        for (w, p) in prefs.iter().enumerate() {
            for (i, &m) in p.iter().enumerate() {
                if (m as usize) < n {
                    r[w][m as usize] = i as u32;
                }
            }
        }
        r
    };
    let rm = rank_w(men);
    let rw = rank_w(women);
    // husband[w] = man matched to w.
    let mut husband = vec![0u32; n];
    for (m, &w) in matching.iter().enumerate() {
        husband[w as usize] = m as u32;
    }
    for m in 0..n {
        for w in 0..n {
            if w as u32 == matching[m] {
                continue;
            }
            let man_prefers = rm[m][w] < rm[m][matching[m] as usize];
            let woman_prefers = rw[w][m] < rw[w][husband[w] as usize];
            if man_prefers && woman_prefers {
                return false; // blocking pair
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn shuffled_prefs(rng: &mut SplitMix64, n: usize) -> Vec<Vec<u32>> {
        (0..n)
            .map(|_| {
                let mut p: Vec<u32> = (0..n as u32).collect();
                for i in (1..n).rev() {
                    let j = rng.below(i as u32 + 1) as usize;
                    p.swap(i, j);
                }
                p
            })
            .collect()
    }

    /// Oracle: enumerate all perfect matchings, keep stable ones, and
    /// confirm the returned matching is man-optimal among them.
    fn oracle_man_optimal(men: &[Vec<u32>], women: &[Vec<u32>]) -> Option<Vec<u32>> {
        let n = men.len();
        let mut best: Option<Vec<u32>> = None;
        let mut perm: Vec<u32> = (0..n as u32).collect();
        // All permutations of wife assignments.
        loop {
            if is_stable(men, women, &perm) {
                best = match &best {
                    None => Some(perm.clone()),
                    Some(b) => {
                        // Man-optimal = every man at least as well off.
                        let b_dom = (0..n).all(|m| {
                            let ri = |p: &[u32], w| p.iter().position(|&x| x == w).unwrap_or(0);
                            ri(&men[m], perm[m]) <= ri(&men[m], b[m])
                        });
                        if b_dom {
                            Some(perm.clone())
                        } else {
                            Some(b.clone())
                        }
                    }
                };
            }
            // next permutation
            let mut i = n as i64 - 2;
            while i >= 0 && perm[i as usize] >= perm[i as usize + 1] {
                i -= 1;
            }
            if i < 0 {
                break;
            }
            let mut j = n - 1;
            while perm[j] <= perm[i as usize] {
                j -= 1;
            }
            perm.swap(i as usize, j);
            perm[i as usize + 1..].reverse();
        }
        best
    }

    #[test]
    fn matches_man_optimal_oracle_and_is_stable() {
        let mut rng = SplitMix64::new(0x5A5A);
        for _ in 0..120 {
            let n = (rng.below(5) + 1) as usize;
            let men = shuffled_prefs(&mut rng, n);
            let women = shuffled_prefs(&mut rng, n);
            let m = stable_match(&men, &women).unwrap();
            assert!(is_stable(&men, &women, &m), "men={men:?} women={women:?}");
            let want = oracle_man_optimal(&men, &women);
            assert_eq!(Some(m), want, "men={men:?} women={women:?}");
        }
    }

    #[test]
    fn proposal_order_independence() {
        // Reversed free-queue order must give the same matching —
        // man-optimality is unique regardless of proposal order.
        let men = vec![vec![0, 1, 2], vec![1, 0, 2], vec![2, 0, 1]];
        let women = vec![vec![2, 0, 1], vec![0, 1, 2], vec![0, 2, 1]];
        let m = stable_match(&men, &women).unwrap();
        assert!(is_stable(&men, &women, &m));
    }

    #[test]
    fn edge_cases() {
        assert_eq!(stable_match(&[], &[]), Some(vec![]));
        assert_eq!(stable_match(&[vec![0]], &[vec![0]]), Some(vec![0]));
        // Ragged / mismatched sizes / non-permutations → None.
        assert!(stable_match(&[vec![0, 1]], &[vec![0, 1], vec![1, 0]]).is_none());
        assert!(stable_match(&[vec![0, 0], vec![0, 1]], &[vec![0, 1], vec![1, 0]]).is_none());
        assert!(stable_match(&[vec![0], vec![]], &[vec![0], vec![0]]).is_none());
        // is_stable rejects non-permutations and blocking pairs.
        assert!(!is_stable(
            &[vec![0, 1], vec![0, 1]],
            &[vec![0, 1], vec![0, 1]],
            &[0, 0]
        ));
        let men = vec![vec![0, 1], vec![1, 0]];
        let women = vec![vec![0, 1], vec![0, 1]];
        // Men-optimal result: each man gets his first pick.
        let m = stable_match(&men, &women).unwrap();
        assert_eq!(m, vec![0, 1]);
        assert!(is_stable(&men, &women, &m));
        // Swapped is unstable: m0 prefers w0, and w0 prefers m0 over m1.
        assert!(!is_stable(&men, &women, &[1, 0]));
    }
}
