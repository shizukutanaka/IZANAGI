//! Maximum bipartite matching — Hopcroft–Karp, `O(E·√V)`.
//!
//! Unweighted pairing on a two-sided adjacency list: unit↔job
//! assignment, team formation, glyph↔cell placement. For *weighted*
//! assignment use [`crate::hungarian`]; when the graph isn't two-sided
//! you don't have a bipartite instance.
//!
//! Determinism: BFS levels and DFS augment walks consume adjacency
//! lists in caller order, so the *matching edges returned* depend only
//! on the input, never on hashing.
//!
//! ```
//! use izanagi_kit::bipartite::hopcroft_karp;
//! // Left {0,1,2} → right {0,1,2}; a triangle-free cover of size 2.
//! let adj = vec![vec![0], vec![0, 1], vec![1, 2]];
//! let (size, mate) = hopcroft_karp(&adj, 3);
//! assert_eq!(size, 3);
//! // mate[l] = Some(r) for every matched left node.
//! assert!(mate.iter().all(|m| m.is_some()));
//! ```

/// Hopcroft–Karp on `adj: adj[l] = right nodes adjacent to left l`,
/// `m` = right-side size. Returns `(matching_size, mate)` where
/// `mate[l] = Some(r)` iff left `l` is matched. The specific maximum
/// matching found is deterministic for a given input.
pub fn hopcroft_karp(adj: &[Vec<usize>], m: usize) -> (usize, Vec<Option<usize>>) {
    let n = adj.len();
    let mut pair_l: Vec<Option<usize>> = vec![None; n];
    let mut pair_r: Vec<Option<usize>> = vec![None; m];
    let mut dist: Vec<Option<u32>> = vec![None; n];
    let mut size = 0usize;

    // BFS builds the layered graph over free left nodes; a right node
    // with no mate is the frontier marker that another phase is due.
    let bfs = |pair_l: &Vec<Option<usize>>,
               pair_r: &Vec<Option<usize>>,
               dist: &mut Vec<Option<u32>>|
     -> bool {
        let mut q = std::collections::VecDeque::with_capacity(n);
        for (l, &p) in pair_l.iter().enumerate() {
            if p.is_none() {
                dist[l] = Some(0);
                q.push_back(l);
            } else {
                dist[l] = None;
            }
        }
        let mut found_free = false;
        while let Some(l) = q.pop_front() {
            for &r in &adj[l] {
                if r >= m {
                    continue;
                }
                match pair_r[r] {
                    None => found_free = true,
                    Some(pl) if dist[pl].is_none() => {
                        dist[pl] = dist[l].map(|d| d + 1);
                        q.push_back(pl);
                    }
                    _ => {}
                }
            }
        }
        found_free
    };

    fn dfs(
        l: usize,
        adj: &[Vec<usize>],
        pair_l: &mut Vec<Option<usize>>,
        pair_r: &mut Vec<Option<usize>>,
        dist: &mut Vec<Option<u32>>,
    ) -> bool {
        let d_l = dist[l].unwrap_or(0);
        for &r in &adj[l] {
            if r >= pair_r.len() {
                continue;
            }
            let ok = match pair_r[r] {
                None => true,
                Some(pl) => dist[pl] == Some(d_l + 1) && dfs(pl, adj, pair_l, pair_r, dist),
            };
            if ok {
                pair_l[l] = Some(r);
                pair_r[r] = Some(l);
                return true;
            }
        }
        dist[l] = None;
        false
    }

    while bfs(&pair_l, &pair_r, &mut dist) {
        for l in 0..n {
            if pair_l[l].is_none() && dfs(l, adj, &mut pair_l, &mut pair_r, &mut dist) {
                size += 1;
            }
        }
    }

    (size, pair_l)
}

/// Naive augmenting-path matching (Kuhn) — `O(V·E)`; exposed for parity
/// checking and for the tiny instances where Hopcroft–Karp's layering
/// is overhead. Same determinism contract.
pub fn kuhn_match(adj: &[Vec<usize>], m: usize) -> (usize, Vec<Option<usize>>) {
    let n = adj.len();
    let mut pair_r: Vec<Option<usize>> = vec![None; m];
    let mut pair_l: Vec<Option<usize>> = vec![None; n];

    fn try_kuhn(
        l: usize,
        adj: &[Vec<usize>],
        pair_r: &mut Vec<Option<usize>>,
        seen: &mut [bool],
    ) -> bool {
        for &r in &adj[l] {
            if r >= pair_r.len() || seen[r] {
                continue;
            }
            seen[r] = true;
            match pair_r[r] {
                None => {
                    pair_r[r] = Some(l);
                    return true;
                }
                Some(pl) => {
                    if try_kuhn(pl, adj, pair_r, seen) {
                        pair_r[r] = Some(l);
                        return true;
                    }
                }
            }
        }
        false
    }

    let mut size = 0usize;
    for l in 0..n {
        let mut seen = vec![false; m];
        if try_kuhn(l, adj, &mut pair_r, &mut seen) {
            size += 1;
        }
    }
    for (r, &pl) in pair_r.iter().enumerate() {
        if let Some(l) = pl {
            pair_l[l] = Some(r);
        }
    }
    (size, pair_l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn random_adj(rng: &mut SplitMix64, n: usize, m: usize) -> Vec<Vec<usize>> {
        (0..n)
            .map(|_| (0..m).filter(|_| rng.below(3) == 0).collect::<Vec<_>>())
            .collect()
    }

    #[test]
    fn hopcroft_karp_matches_kuhn_size() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..300 {
            let n = 1 + rng.below(8) as usize;
            let m = 1 + rng.below(8) as usize;
            let adj = random_adj(&mut rng, n, m);
            let (hk, _) = hopcroft_karp(&adj, m);
            let (ku, _) = kuhn_match(&adj, m);
            assert_eq!(hk, ku, "sizes differ on n={n} m={m}");
        }
    }

    #[test]
    fn matching_is_valid_and_size_bounded() {
        let mut rng = SplitMix64::new(0xCAFE);
        for _ in 0..200 {
            let n = 1 + rng.below(10) as usize;
            let m = 1 + rng.below(10) as usize;
            let adj = random_adj(&mut rng, n, m);
            let (size, mate) = hopcroft_karp(&adj, m);
            assert!(size <= n.min(m));
            let mut right_used = vec![false; m];
            let mut counted = 0;
            for (l, &mr) in mate.iter().enumerate() {
                if let Some(r) = mr {
                    assert!(adj[l].contains(&r), "matched edge not in adj");
                    assert!(!right_used[r], "right node reused");
                    right_used[r] = true;
                    counted += 1;
                }
            }
            assert_eq!(counted, size);
        }
    }

    #[test]
    fn perfect_matching_found_when_present() {
        // Identity permutation graph — n == m, adj[l] = {l}.
        let adj: Vec<Vec<usize>> = (0..6).map(|l| vec![l]).collect();
        let (size, mate) = hopcroft_karp(&adj, 6);
        assert_eq!(size, 6);
        assert!(mate.iter().all(|m| m.is_some()));
    }

    #[test]
    fn empty_and_degenerate_inputs() {
        let (s0, m0) = hopcroft_karp(&[], 3);
        assert_eq!(s0, 0);
        assert!(m0.is_empty());
        let (s1, _) = hopcroft_karp(&[vec![]], 3); // left node, no edges
        assert_eq!(s1, 0);
        let (s2, _) = hopcroft_karp(&[vec![0, 5, 7]], 3); // out-of-range filtered
        assert_eq!(s2, 1);
    }
}
