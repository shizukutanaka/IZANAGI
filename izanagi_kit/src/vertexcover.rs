//! Minimum vertex cover on bipartite graphs via Kőnig's theorem
//! (1931): a minimum cover has exactly `max_matching` vertices, and
//! one is recovered by alternating reachability — let `Z` be the
//! vertices reachable from unmatched left vertices along unmatched
//! `L→R` edges and matching `R→L` edges; then
//! `C = (L ∖ Z) ∪ (R ∩ Z)` is a minimum vertex cover. Reuses
//! [`crate::bipartite::hopcroft_karp`] for the matching, so the
//! whole computation stays `O(E·√V)`.
//!
//! ```
//! use izanagi_kit::vertexcover::min_vertex_cover;
//! // Path 0—0—1—1 on a 2×2 bipartition: matching = 2, cover = 2.
//! let (lc, rc) = min_vertex_cover(&[vec![0], vec![1]], 2);
//! assert_eq!(lc.len() + rc.len(), 2);
//! ```

use crate::bipartite::hopcroft_karp;

/// Minimum vertex cover of the bipartite graph `adj` (left side
/// `0..adj.len()`, right side `0..m`). Returns the two sides of the
/// cover `(left_cover, right_cover)`, both sorted — a pure function
/// of the edge set.
pub fn min_vertex_cover(adj: &[Vec<usize>], m: usize) -> (Vec<usize>, Vec<usize>) {
    let n = adj.len();
    let (_size, ml) = hopcroft_karp(adj, m);
    // mr[r] = matched left vertex (if any).
    let mut mr = vec![None; m];
    for (u, &v) in ml.iter().enumerate() {
        if let Some(v) = v {
            mr[v] = Some(u);
        }
    }
    // Alternating BFS from free left vertices: unmatched edges go
    // L→R, matching edges go R→L.
    let mut lz = vec![false; n];
    let mut rz = vec![false; m];
    let mut queue: Vec<(bool, usize)> = Vec::new();
    for (u, &v) in ml.iter().enumerate() {
        if v.is_none() {
            lz[u] = true;
            queue.push((true, u));
        }
    }
    let mut head = 0;
    while head < queue.len() {
        let (side, x) = queue[head];
        head += 1;
        if side {
            for &v in &adj[x] {
                if ml[x] != Some(v) && !rz[v] {
                    rz[v] = true;
                    queue.push((false, v));
                }
            }
        } else if let Some(u) = mr[x] {
            if !lz[u] {
                lz[u] = true;
                queue.push((true, u));
            }
        }
    }
    let left_cover: Vec<usize> = (0..n).filter(|&u| !lz[u]).collect();
    let right_cover: Vec<usize> = (0..m).filter(|&v| rz[v]).collect();
    (left_cover, right_cover)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn covers_all(adj: &[Vec<usize>], m: usize, lc: &[usize], rc: &[usize]) -> bool {
        for (u, es) in adj.iter().enumerate() {
            for &v in es {
                if !lc.contains(&u) && !rc.contains(&v) {
                    return false;
                }
            }
        }
        let _ = m;
        true
    }

    #[test]
    fn path_and_star() {
        // Single edge: cover size 1.
        let (lc, rc) = min_vertex_cover(&[vec![0]], 1);
        assert_eq!(lc.len() + rc.len(), 1);
        assert!(covers_all(&[vec![0]], 1, &lc, &rc));
        // Star K_{1,3}: center-side cover of size 1.
        let (lc, rc) = min_vertex_cover(&[vec![0, 1, 2]], 3);
        assert_eq!(lc.len() + rc.len(), 1);
        assert!(covers_all(&[vec![0, 1, 2]], 3, &lc, &rc));
        // Complete K_{3,3}: cover = 3.
        let k33 = vec![vec![0, 1, 2], vec![0, 1, 2], vec![0, 1, 2]];
        let (lc, rc) = min_vertex_cover(&k33, 3);
        assert_eq!(lc.len() + rc.len(), 3);
        assert!(covers_all(&k33, 3, &lc, &rc));
    }

    #[test]
    fn equals_matching_size_and_covers() {
        let mut rng = SplitMix64::new(67);
        for _ in 0..50 {
            let n = 1 + rng.below(8) as usize;
            let m = 1 + rng.below(8) as usize;
            let adj: Vec<Vec<usize>> = (0..n)
                .map(|_| (0..m).filter(|_| rng.below(2) == 1).collect())
                .collect();
            let (size, _ml) = hopcroft_karp(&adj, m);
            let (lc, rc) = min_vertex_cover(&adj, m);
            assert_eq!(lc.len() + rc.len(), size);
            assert!(covers_all(&adj, m, &lc, &rc));
        }
    }

    #[test]
    fn brute_force_minimality() {
        // Exhaustive cover search on tiny graphs.
        let mut rng = SplitMix64::new(71);
        for _ in 0..40 {
            let n = 1 + rng.below(4) as usize;
            let m = 1 + rng.below(4) as usize;
            let adj: Vec<Vec<usize>> = (0..n)
                .map(|_| (0..m).filter(|_| rng.below(3) == 0).collect())
                .collect();
            let (lc, rc) = min_vertex_cover(&adj, m);
            let got = lc.len() + rc.len();
            // Enumerate all 2^(n+m) vertex sets.
            let mut best = n + m + 1; // sentinel: no cover found yet
            for mask in 0..(1usize << (n + m)) {
                let mut ok = true;
                'edges: for (u, es) in adj.iter().enumerate() {
                    for &v in es {
                        let l = (mask >> u) & 1 == 1;
                        let r = (mask >> (n + v)) & 1 == 1;
                        if !l && !r {
                            ok = false;
                            break 'edges;
                        }
                    }
                }
                if ok {
                    best = best.min(mask.count_ones() as usize);
                }
            }
            assert_eq!(got, best);
        }
    }

    #[test]
    fn empty_and_disconnected() {
        let (lc, rc) = min_vertex_cover(&[Vec::new(), Vec::new()], 3);
        assert!(lc.is_empty() && rc.is_empty());
        // Isolated right vertices never enter the cover.
        let (lc, rc) = min_vertex_cover(&[vec![0]], 5);
        assert_eq!(lc.len() + rc.len(), 1);
    }
}
