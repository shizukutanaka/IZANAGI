//! Held–Karp dynamic-programming TSP — the *exact* counterpart to
//! [`crate::tsp`]'s heuristics.
//!
//! `dp[mask][v]` = cheapest path that starts at city `0`, visits exactly
//! `mask` (bit 0 excluded by convention), and ends at `v ∈ mask`.
//! `O(n²·2ⁿ)` time, `O(n·2ⁿ)` space — the memory bound is the reason the
//! city count is capped at [`MAX_CITIES`].
//!
//! Determinism: subsets are enumerated in increasing mask order and
//! transitions scan cities in increasing index order, so the witness
//! tour is the lexicographically smallest optimal one.
//!
//! ```
//! use izanagi_kit::hamdp::tsp_exact;
//! // A 3-city triangle: 0→1→2→0 costs 10 no matter which direction.
//! let dist = vec![
//!     vec![0, 3, 4],
//!     vec![3, 0, 5],
//!     vec![4, 5, 0],
//! ];
//! let (cost, tour) = tsp_exact(&dist).unwrap();
//! assert_eq!(cost, 12);
//! assert_eq!(tour.len(), 4); // start city repeated at the end
//! ```

/// Maximum city count — `2^n · n` `u64` cells must stay reasonable.
pub const MAX_CITIES: usize = 16;

/// Solves the traveling-salesman problem exactly on the full
/// `n × n` distance matrix. `dist[u][v]` is the cost of the edge `u→v`;
/// asymmetry is fine (directed TSP). `dist[v][v]` is ignored.
///
/// `u32::MAX` in an entry means "no edge" — the tour may be absent.
///
/// Returns `(minimum tour cost, visiting order)` where the order starts
/// and ends at city 0 — the lexicographically smallest optimal tour.
/// `None` when `n < 2`, `n > MAX_CITIES`, or the matrix is ragged.
pub fn tsp_exact(dist: &[Vec<u32>]) -> Option<(u64, Vec<u32>)> {
    let n = dist.len();
    if !(2..=MAX_CITIES).contains(&n) || dist.iter().any(|r| r.len() != n) {
        return None;
    }
    let m = n - 1; // cities 1..n in the DP
    let full = (1usize << m) - 1;
    let idx = |mask: usize, j: usize| mask * m + j;
    // dp[mask][j]: min cost 0 → {mask} → city j+1.
    let mut dp = vec![u64::MAX; (full + 1) * m];
    for j in 0..m {
        if dist[0][j + 1] != u32::MAX {
            let c = 1usize << j;
            dp[idx(c, j)] = dist[0][j + 1] as u64;
        }
    }
    for mask in 1..=full {
        for j in 0..m {
            if mask & (1 << j) == 0 {
                continue;
            }
            let prev_mask = mask ^ (1 << j);
            let mut best = u64::MAX;
            for k in 0..m {
                if prev_mask & (1 << k) == 0 {
                    continue;
                }
                let prev = dp[idx(prev_mask, k)];
                if prev != u64::MAX && dist[k + 1][j + 1] != u32::MAX {
                    let cand = prev + dist[k + 1][j + 1] as u64;
                    best = best.min(cand);
                }
            }
            let cur = idx(mask, j);
            dp[cur] = dp[cur].min(best);
        }
    }
    // Close the tour: mask = full, end j, then edge j+1 → 0.
    let mut best = u64::MAX;
    for j in 0..m {
        let d = dp[idx(full, j)];
        if d != u64::MAX && dist[j + 1][0] != u32::MAX {
            best = best.min(d + dist[j + 1][0] as u64);
        }
    }
    if best == u64::MAX {
        return None; // unreachable tour (all dist u32::MAX, etc.)
    }
    // Reconstruct the lexicographically smallest optimal tour: at each
    // step choose the smallest next-city index that can still close
    // optimally. Greedy choice is safe because dp is already computed.
    let mut tour = vec![0u32];
    let mut mask = 0usize;
    let mut cur_cost = 0u64; // cost of tour[0..] so far
    let mut at = 0usize; // current city
    for step in 0..m {
        let rem = m - step - 1;
        let mut chosen = None;
        for j in 0..m {
            if mask & (1 << j) != 0 {
                continue;
            }
            // Feasible: after taking j, dp over the rest must equal best.
            if dist[at][j + 1] == u32::MAX {
                continue;
            }
            let take = cur_cost + dist[at][j + 1] as u64;
            let nmask = mask | (1 << j);
            let tail = if rem == 0 {
                if dist[j + 1][0] == u32::MAX {
                    u64::MAX
                } else {
                    dist[j + 1][0] as u64
                }
            } else {
                // Exact optimal tail: cheapest path from j+1 through every
                // still-unvisited city, closing to 0.
                tail_cost(j, nmask, dist, m)
            };
            if tail == u64::MAX {
                continue;
            }
            if take + tail == best {
                chosen = Some(j);
                break;
            }
        }
        let j = chosen?; // some city must be feasible when best is finite
        tour.push(j as u32 + 1);
        cur_cost += dist[at][j + 1] as u64;
        mask |= 1 << j;
        at = j + 1;
    }
    tour.push(0);
    Some((best, tour))
}

/// Minimum cost to finish a tour standing at city `j+1` with `mask`
/// already visited (bit i ↔ city i+1): visit every city not in `mask`,
/// then return to 0. Computed as its own Held–Karp subproblem over the
/// remaining cities — small enough that the reconstruction stays cheap.
fn tail_cost(j: usize, mask: usize, dist: &[Vec<u32>], m: usize) -> u64 {
    let rem: Vec<usize> = (0..m).filter(|&c| mask & (1 << c) == 0).collect();
    if rem.is_empty() {
        return if dist[j + 1][0] == u32::MAX {
            u64::MAX
        } else {
            dist[j + 1][0] as u64
        };
    }
    let r = rem.len();
    let rfull = (1usize << r) - 1;
    // f[s][i] = min cost: start at j+1, visit s ⊆ rem, end at rem[i]+1.
    let mut f = vec![u64::MAX; (rfull + 1) * r];
    for (i, &c) in rem.iter().enumerate() {
        f[(1 << i) * r + i] = if dist[j + 1][c + 1] == u32::MAX {
            u64::MAX
        } else {
            dist[j + 1][c + 1] as u64
        };
    }
    for s in 1..=rfull {
        for i in 0..r {
            if s & (1 << i) == 0 {
                continue;
            }
            let ps = s ^ (1 << i);
            let mut b = u64::MAX;
            for k in 0..r {
                if ps & (1 << k) == 0 {
                    continue;
                }
                let pv = f[ps * r + k];
                if pv != u64::MAX && dist[rem[k] + 1][rem[i] + 1] != u32::MAX {
                    b = b.min(pv + dist[rem[k] + 1][rem[i] + 1] as u64);
                }
            }
            let cur = s * r + i;
            f[cur] = f[cur].min(b);
        }
    }
    let mut t = u64::MAX;
    for i in 0..r {
        let v = f[rfull * r + i];
        if v != u64::MAX && dist[rem[i] + 1][0] != u32::MAX {
            t = t.min(v + dist[rem[i] + 1][0] as u64);
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: brute-force all permutations of 1..n.
    fn oracle(dist: &[Vec<u32>]) -> u64 {
        let n = dist.len();
        let mut perm: Vec<usize> = (1..n).collect();
        let mut best = u64::MAX;
        loop {
            let mut c = 0u64;
            let mut prev = 0;
            for &v in &perm {
                c += dist[prev][v] as u64;
                prev = v;
            }
            c += dist[prev][0] as u64;
            best = best.min(c);
            // next lexicographic permutation (perm has n-1 elements)
            let pl = n - 1;
            let mut i = pl - 2;
            while perm[i] > perm[i + 1] {
                if i == 0 {
                    return best;
                }
                i -= 1;
            }
            let mut j = pl - 1;
            while perm[j] < perm[i] {
                j -= 1;
            }
            perm.swap(i, j);
            perm[i + 1..].reverse();
        }
    }

    #[test]
    fn matches_permutation_oracle() {
        let mut rng = SplitMix64::new(0x4A17);
        for _ in 0..120 {
            let n = (rng.below(6) + 3) as usize;
            let mut dist = vec![vec![0u32; n]; n];
            for (u, row) in dist.iter_mut().enumerate() {
                for (v, cell) in row.iter_mut().enumerate() {
                    if u != v {
                        *cell = rng.below(97) + 1;
                    }
                }
            }
            let (cost, tour) = tsp_exact(&dist).unwrap();
            assert_eq!(cost, oracle(&dist), "dist={dist:?}");
            // Witness validity: visits all cities once and closes.
            assert_eq!(tour.len(), n + 1);
            assert_eq!(tour[0], 0);
            assert_eq!(tour[n], 0);
            let mut seen = vec![false; n];
            for &v in &tour[1..n] {
                assert!(!seen[v as usize]);
                seen[v as usize] = true;
            }
            // Witness cost equals the claimed optimum.
            let mut c = 0u64;
            for w in 0..n {
                c += dist[tour[w] as usize][tour[w + 1] as usize] as u64;
            }
            assert_eq!(c, cost);
        }
    }

    #[test]
    fn lexicographically_smallest_optimum() {
        // Square: 0-1-2-3-0 and 0-3-2-1-0 both cost 4. The tour that
        // visits city 1 second is lexicographically smaller.
        let dist = vec![
            vec![0, 1, 2, 1],
            vec![1, 0, 1, 2],
            vec![2, 1, 0, 1],
            vec![1, 2, 1, 0],
        ];
        let (cost, tour) = tsp_exact(&dist).unwrap();
        assert_eq!(cost, 4);
        assert_eq!(tour, vec![0, 1, 2, 3, 0]);
    }

    #[test]
    fn edge_cases() {
        assert!(tsp_exact(&[vec![0]]).is_none());
        assert!(tsp_exact(&Vec::new()).is_none());
        let big = vec![vec![1u32; MAX_CITIES + 1]; MAX_CITIES + 1];
        assert!(tsp_exact(&big).is_none());
        // Ragged.
        assert!(tsp_exact(&[vec![0, 1], vec![1]]).is_none());
        // Unreachable: all off-diagonal edges absent (u32::MAX).
        let mut d = vec![vec![u32::MAX; 3]; 3];
        for (v, row) in d.iter_mut().enumerate() {
            row[v] = 0;
        }
        assert!(tsp_exact(&d).is_none());
    }
}
