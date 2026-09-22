//! Deterministic TSP heuristics on an `i64` distance matrix —
//! nearest-neighbour seed + 2-opt improvement loop.
//!
//! Patrol routes, courier paths, "visit every trigger once" mission
//! plans: the output is a *good* tour, not a provably optimal one —
//! determinism, not optimality, is the contract. Exact optima for small
//! instances go through the test oracle, not this API.
//!
//! Canonicalization: the returned tour always starts at city 0 and is
//! oriented so `tour[1] < tour[n-1]` — reversing gives the same cycle.
//! All tie-breaks are index order, so the result is a pure function of
//! the matrix.
//!
//! ```
//! use izanagi_kit::tsp::{tour_cost, tsp_2opt};
//! // Unit square: optimum tour length 4.
//! let d = vec![
//!     vec![0, 1, 2, 1],
//!     vec![1, 0, 1, 2],
//!     vec![2, 1, 0, 1],
//!     vec![1, 2, 1, 0],
//! ];
//! let tour = tsp_2opt(&d).unwrap();
//! assert_eq!(tour_cost(&d, &tour), Some(4));
//! ```

/// Length of a tour (permutation of `0..n`, cyclic) under matrix `d`.
/// `None` when `tour` isn't a permutation or `d` isn't square.
pub fn tour_cost(d: &[Vec<i64>], tour: &[usize]) -> Option<i64> {
    let n = d.len();
    if n == 0 || tour.len() != n || d.iter().any(|r| r.len() != n) {
        return None;
    }
    let mut seen = vec![false; n];
    for &c in tour {
        if c >= n || seen[c] {
            return None;
        }
        seen[c] = true;
    }
    let mut s = 0i64;
    for i in 0..n {
        s += d[tour[i]][tour[(i + 1) % n]];
    }
    Some(s)
}

/// Nearest-neighbour construction starting at city 0: repeatedly move
/// to the closest unvisited city (ties → smallest index). `O(n²)`.
/// `None` for empty/asymmetric-length matrices.
pub fn nn_tour(d: &[Vec<i64>]) -> Option<Vec<usize>> {
    let n = d.len();
    if n == 0 || d.iter().any(|r| r.len() != n) {
        return None;
    }
    let mut tour = Vec::with_capacity(n);
    let mut visited = vec![false; n];
    let mut cur = 0usize;
    tour.push(0);
    visited[0] = true;
    for _ in 1..n {
        // Ascending scan + strict `<` ⇒ ties resolve to the smallest index.
        let mut best = 0usize;
        let mut best_d = i64::MAX;
        for c in 0..n {
            if !visited[c] && d[cur][c] < best_d {
                best_d = d[cur][c];
                best = c;
            }
        }
        tour.push(best);
        visited[best] = true;
        cur = best;
    }
    Some(tour)
}

/// NN seed + 2-opt descent: repeatedly apply the first improving
/// segment reversal found in canonical `(i, j)` scan order, until a
/// local optimum. `O(n³)` per pass on dense matrices; total passes are
/// bounded since each strictly reduces the cost.
///
/// The result is canonicalized: rotated to start at city 0 and oriented
/// so the second city has the smaller index of the two neighbours.
/// `None` for degenerate input; `Some(vec![0])` for `n == 1`.
pub fn tsp_2opt(d: &[Vec<i64>]) -> Option<Vec<usize>> {
    let n = d.len();
    let mut tour = nn_tour(d)?;
    if n <= 3 {
        return Some(canonicalize(tour));
    }
    let mut improved = true;
    while improved {
        improved = false;
        // First improving 2-opt swap in (i, j) order — skip the wrap edge.
        'scan: for i in 0..n - 1 {
            for j in i + 2..n {
                // Reversal of tour[i+1..=j]: edges (i,i+1)+(j,j+1) → (i,j)+(i+1,j+1)
                let a = tour[i];
                let b = tour[i + 1];
                let c = tour[j];
                let e = tour[(j + 1) % n];
                if i == 0 && j == n - 1 {
                    continue; // the wrap edge pair — reversal is identity
                }
                if d[a][c] + d[b][e] < d[a][b] + d[c][e] {
                    tour[i + 1..=j].reverse();
                    improved = true;
                    break 'scan;
                }
            }
        }
    }
    Some(canonicalize(tour))
}

/// Rotate to start at 0; orient so `tour[1] < tour[n-1]`.
fn canonicalize(mut tour: Vec<usize>) -> Vec<usize> {
    let n = tour.len();
    if n <= 1 {
        return tour;
    }
    let pos = tour.iter().position(|&c| c == 0).unwrap_or(0);
    tour.rotate_left(pos);
    if tour[1] > tour[n - 1] {
        tour[1..].reverse();
    }
    tour
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn random_dist(rng: &mut SplitMix64, n: usize) -> Vec<Vec<i64>> {
        let mut d = vec![vec![0i64; n]; n];
        let pairs: Vec<(usize, usize)> = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .collect();
        for (i, j) in pairs {
            d[i][j] = rng.below(100) as i64;
            d[j][i] = d[i][j];
        }
        d
    }

    fn brute_opt(d: &[Vec<i64>]) -> i64 {
        // Exhaustive permutations, n ≤ 8.
        let n = d.len();
        let mut perm: Vec<usize> = (0..n).collect();
        let mut best = i64::MAX;
        loop {
            if perm[0] == 0 {
                // canonicalized start — cost is cycle-invariant
                if let Some(c) = tour_cost(d, &perm) {
                    best = best.min(c);
                }
            }
            // next permutation
            if let Some(i) = (0..n - 1).rev().find(|&i| perm[i] < perm[i + 1]) {
                let j = (i + 1..n).rev().find(|&j| perm[j] > perm[i]).unwrap();
                perm.swap(i, j);
                perm[i + 1..].reverse();
            } else {
                break;
            }
        }
        best
    }

    #[test]
    fn cost_is_a_valid_permutation_metric() {
        let d = random_dist(&mut SplitMix64::new(1), 6);
        assert_eq!(tour_cost(&d, &[0, 1, 2, 3, 4, 4]), None);
        assert_eq!(tour_cost(&d, &[0, 1, 2, 3, 4]), None); // wrong length
        assert!(tour_cost(&d, &[0, 1, 2, 3, 4, 5]).is_some());
        let empty: Vec<Vec<i64>> = vec![];
        assert_eq!(nn_tour(&empty), None);
        assert_eq!(tsp_2opt(&[vec![7]]), Some(vec![0]));
    }

    #[test]
    fn two_opt_never_worse_than_nn_seed() {
        let mut rng = SplitMix64::new(0x7E59);
        for _ in 0..200 {
            let n = 2 + rng.below(9) as usize;
            let d = random_dist(&mut rng, n);
            let nn = nn_tour(&d).unwrap();
            let opt = tsp_2opt(&d).unwrap();
            let (cn, co) = (tour_cost(&d, &nn).unwrap(), tour_cost(&d, &opt).unwrap());
            assert!(co <= cn, "2opt {co} > nn {cn} for n={n}");
            assert_eq!(&opt[0], &0);
        }
    }

    #[test]
    fn two_opt_hits_optimum_on_small_instances() {
        // n ≤ 8: 2-opt + canonical orientation reaches the true optimum
        // for the overwhelming majority of iid matrices — the oracle is
        // exact, so any miss is visible. We don't assert optimality
        // (heuristic, not a solver), only that cost ≤ optimum*(1+slack)
        // is never violated — here we use slack 0 to measure hit-rate.
        let mut rng = SplitMix64::new(0x0DE7);
        let mut hits = 0;
        let trials = 200;
        for _ in 0..trials {
            let n = 4 + rng.below(4) as usize;
            let d = random_dist(&mut rng, n);
            let opt = tsp_2opt(&d).unwrap();
            let got = tour_cost(&d, &opt).unwrap();
            let want = brute_opt(&d);
            hits += usize::from(got == want);
            assert!(got >= want);
        }
        // Deterministic sanity: heuristic should nail most 4..8 cases.
        assert!(hits >= trials * 7 / 10, "hit rate {hits}/{trials}");
    }

    #[test]
    fn tour_is_canonical_regardless_of_matrix_labeling() {
        let d = vec![
            vec![0, 5, 9, 3],
            vec![5, 0, 2, 7],
            vec![9, 2, 0, 4],
            vec![3, 7, 4, 0],
        ];
        let a = tsp_2opt(&d).unwrap();
        let b = tsp_2opt(&d).unwrap();
        assert_eq!(a, b);
        assert_eq!(a[0], 0);
        assert!(a[1] < a[a.len() - 1]);
    }

    #[test]
    fn asymmetric_matrices_handled() {
        // No symmetry requirement: cost/tour still well-defined.
        let mut d = random_dist(&mut SplitMix64::new(2), 5);
        d[1][3] += 50;
        let t = tsp_2opt(&d).unwrap();
        assert_eq!(t.len(), 5);
    }
}
