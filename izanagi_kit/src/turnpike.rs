//! Turnpike reconstruction — recover point positions `0 = x₀ < … <
//! x_{n−1} = D` from the multiset of all pairwise distances
//! `{ |xᵢ − xⱼ| }`, by Skiena's backtracking: the largest remaining
//! distance must be `x_{n−1} − x₀`-anchored at one end — try placing
//! it on the left (at position `d`) or the right (at `D − d`), check
//! the implied distances exist in the multiset, recurse. Deterministic:
//! left placements are tried first, so the returned solution is a
//! pure function of the sorted input (up to the distance multiset —
//! the *point set* can be genuinely ambiguous: `{0,1,4,6}` and
//! `{0,2,3,6}` are homometric mates; this returns the DFS-first one).
//!
//! [`reconstruct`] also returns the canonical form `min(points,
//! reflect(points))` when `canonical` is set — see
//! [`canonical_turnpike`].
//!
//! ```
//! // {0,2,3,6} has distances {1,2,3,4,6} + {2,4,6}... classic example:
//! let pts = izanagi_kit::turnpike::reconstruct(&[1, 2, 3, 4, 5, 6]);
//! assert!(pts.is_some());
//! ```
//!
//! Reference: Skiena, Smith & Lemke (1990), "Reconstructing sets
//! from interpoint distances" (SoCG).

use std::collections::BTreeMap;

/// Canonical form of a solution: the lexicographically smaller of the
/// point set and its reflection `{D − xᵢ}`.
pub fn canonical_turnpike(points: &[u64], d: u64) -> Vec<u64> {
    let mut refl: Vec<u64> = points.iter().map(|&p| d - p).collect();
    refl.sort_unstable();
    if refl < points.to_vec() {
        refl
    } else {
        points.to_vec()
    }
}

/// Multiset of pairwise distances of `points` — the round-trip
/// oracle's own helper, also public for callers.
pub fn distances(points: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(points.len() * (points.len() + 1) / 2);
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            out.push(points[i].abs_diff(points[j]));
        }
    }
    out.sort_unstable();
    out
}

/// Reconstruct points from the pairwise-distance multiset
/// `distances` (must contain exactly `n(n−1)/2` entries — inferred
/// `n`; `None` when inconsistent, unsatisfiable, or `n < 2`).
///
/// Deterministic: returns the DFS-first solution (left placements
/// first). Reflected duplicates collapse to the canonical answer.
pub fn reconstruct(distances_in: &[u64]) -> Option<Vec<u64>> {
    // n from m = n(n−1)/2 — n=1 degenerates to {0} with no distances.
    let m = distances_in.len();
    if m == 0 {
        return Some(vec![0]);
    }
    let mut n = 1usize;
    while n * (n - 1) / 2 < m {
        n += 1;
    }
    if n * (n - 1) / 2 != m {
        return None;
    }
    let mut dist: BTreeMap<u64, u64> = BTreeMap::new();
    for &d in distances_in {
        *dist.entry(d).or_insert(0) += 1;
    }
    // D = largest distance.
    let d = *dist.keys().next_back()?;
    // place x0 = 0, x_{n-1} = D
    remove(&mut dist, d)?;
    let mut placed = vec![0u64, d];
    let mut slots = n - 2;
    let mut sol = Vec::with_capacity(n);
    solve(&mut dist, &mut placed, d, &mut slots, &mut sol).then(|| {
        sol.sort_unstable();
        sol
    })
}

fn remove(map: &mut BTreeMap<u64, u64>, d: u64) -> Option<()> {
    let e = map.get_mut(&d)?;
    if *e == 0 {
        return None;
    }
    *e -= 1;
    if *e == 0 {
        map.remove(&d);
    }
    Some(())
}

fn solve(
    dist: &mut BTreeMap<u64, u64>,
    placed: &mut Vec<u64>,
    d: u64,
    slots: &mut usize,
    sol: &mut Vec<u64>,
) -> bool {
    if *slots == 0 {
        if dist.is_empty() {
            sol.extend_from_slice(placed);
            return true;
        }
        return false;
    }
    // largest remaining distance d_max: candidate at left=d_max or
    // right=D-d_max.
    let &dmax = match dist.keys().next_back() {
        Some(v) => v,
        None => return *slots == 0,
    };
    for &x in &[dmax, d.saturating_sub(dmax)] {
        // implied distances |x - p| for all placed p
        let mut need: Vec<u64> = placed.iter().map(|&p| x.abs_diff(p)).collect();
        need.sort_unstable();
        // all needs present? and x not already placed (dup positions)
        let mut ok = !placed.contains(&x);
        if ok {
            // multiplicity matters: `need` may contain duplicates and
            // each must be backed by that many multiset entries.
            let mut i = 0;
            while i < need.len() {
                let v = need[i];
                let mut cnt = 0u64;
                while i < need.len() && need[i] == v {
                    cnt += 1;
                    i += 1;
                }
                if dist.get(&v).copied().unwrap_or(0) < cnt {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            // place
            for &nd in &need {
                let _ = remove(dist, nd);
            }
            placed.push(x);
            *slots -= 1;
            if solve(dist, placed, d, slots, sol) {
                return true;
            }
            placed.pop();
            *slots += 1;
            for &nd in &need {
                *dist.entry(nd).or_insert(0) += 1;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn classic_example() {
        // Points {0,1,4,7,9,10} (the standard textbook instance).
        let pts = vec![0u64, 1, 4, 7, 9, 10];
        let d = distances(&pts);
        let got = reconstruct(&d).expect("solvable");
        assert_eq!(distances(&got), d);
        assert_eq!(canonical_turnpike(&got, 10), vec![0, 1, 3, 6, 9, 10]);
    }

    #[test]
    fn roundtrip_oracle() {
        // Random point sets → distance multiset → reconstruct →
        // recompute distances: must equal the input multiset.
        let mut rng = SplitMix64::new(0x7a91_5eed_c0de_beef);
        for _ in 0..200 {
            let n = 2 + rng.below(6) as usize;
            let mut pts: Vec<u64> = (0..n).map(|_| rng.below(40) as u64).collect();
            pts.sort_unstable();
            pts.dedup();
            while pts.len() < n {
                pts.push(rng.below(60) as u64);
                pts.sort_unstable();
                pts.dedup();
            }
            let d = distances(&pts);
            let got = match reconstruct(&d) {
                Some(g) => g,
                None => panic!("unsolved for {pts:?}"),
            };
            assert_eq!(got.len(), n);
            assert_eq!(distances(&got), d);
            assert_eq!(got[0], 0);
        }
    }

    #[test]
    fn homometric_mates() {
        // {0,1,5,7,8} vs {0,1,3,7,8} share the distance multiset
        // {1,1,2,3,4,5,6,7,7,8} — reconstruction returns the
        // DFS-first one; both are valid answers.
        let a = vec![0u64, 1, 5, 7, 8];
        let b = vec![0u64, 1, 3, 7, 8];
        assert_eq!(distances(&a), distances(&b));
        let got = reconstruct(&distances(&a)).expect("solvable");
        assert_eq!(distances(&got), distances(&a));
    }

    #[test]
    fn rejects_inconsistent() {
        assert_eq!(reconstruct(&[]), Some(vec![0])); // n=1 trivially
        assert!(reconstruct(&[1]).is_some()); // n=2: {0,1}
        assert_eq!(reconstruct(&[1]), Some(vec![0, 1]));
        assert!(reconstruct(&[1, 2, 4]).is_none()); // can't place 1,2,4 pairwise
        assert!(reconstruct(&[1, 2, 3, 4]).is_none()); // wrong count
                                                       // duplicates in multiset respected:
        let pts = vec![0u64, 2, 4];
        assert_eq!(reconstruct(&distances(&pts)), Some(pts));
    }

    #[test]
    fn canonical_reflect() {
        // {0,2,3,6} reflected in D=6 is {0,3,4,6}: canonical = lex-min.
        assert_eq!(canonical_turnpike(&[0, 2, 3, 6], 6), vec![0, 2, 3, 6]);
        assert_eq!(canonical_turnpike(&[0, 3, 4, 6], 6), vec![0, 2, 3, 6]);
    }
}
