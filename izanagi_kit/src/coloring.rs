//! DSATUR graph coloring — fewest colors with a deterministic choice at
//! every step. Map regions, channel assignment, register-slot-like
//! allocation: any "things that must differ" problem on an integer graph.
//!
//! DSATUR (Brélaz 1979) colors the vertex with the most distinct
//! neighbour colors first — saturation, then degree, then lowest index
//! as tie-breaks — and gives it the smallest free color. Exact on
//! bipartite graphs and cycles; a strong heuristic elsewhere.
//!
//! ```
//! use izanagi_kit::coloring::dsatur;
//! // Path of 4 vertices: bipartite → 2 colors (DSATUR starts at a
//! // highest-degree vertex, so the phase lands on the other half).
//! let adj = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
//! assert_eq!(dsatur(&adj), vec![1, 0, 1, 0]);
//! ```

/// DSATUR coloring of an undirected graph given as `adj[v]` neighbour
/// lists. Returns one color id per vertex (`0..k`), `k ≤ n`. Parallel
/// edges and self-loops are tolerated (a self-loop forces no constraint
/// that breaks validity — it cannot be violated by a single vertex).
pub fn dsatur(adj: &[Vec<u32>]) -> Vec<u32> {
    let n = adj.len();
    let deg: Vec<u32> = adj.iter().map(|a| a.len() as u32).collect();
    let mut color = vec![u32::MAX; n];
    // sat[v] = sorted set of colors adjacent to v (kept as a bitset).
    let mut sat = vec![0u128; n];
    let mut left = n;
    while left > 0 {
        // Pick max saturation; ties: max degree, then lowest index.
        let mut pick = u32::MAX as usize;
        for v in 0..n {
            if color[v] != u32::MAX {
                continue;
            }
            let better = if pick == u32::MAX as usize {
                true
            } else {
                let (sv, sp) = (sat[v].count_ones(), sat[pick].count_ones());
                sv > sp || (sv == sp && (deg[v] > deg[pick] || (deg[v] == deg[pick] && v < pick)))
            };
            if better {
                pick = v;
            }
        }
        // Smallest color not in the saturation set.
        let mut c = 0u32;
        while sat[pick] >> c & 1 == 1 {
            c += 1;
        }
        color[pick] = c;
        left -= 1;
        for &w in &adj[pick] {
            let w = w as usize;
            if color[w] == u32::MAX && w < n {
                sat[w] |= 1u128 << c;
            }
        }
    }
    color
}

/// Check a coloring is proper: every undirected edge's endpoints differ.
pub fn is_proper(adj: &[Vec<u32>], colors: &[u32]) -> bool {
    for (u, nbrs) in adj.iter().enumerate() {
        for &v in nbrs {
            let v = v as usize;
            if v < colors.len() && u != v && colors[u] == colors[v] {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force chromatic number on n ≤ 7: try k colors 1..n.
    fn chromatic(adj: &[Vec<u32>]) -> u32 {
        let n = adj.len();
        for k in 1..=n.max(1) as u32 {
            let mut c = vec![0u32; n];
            if search(adj, &mut c, 0, k) {
                return k;
            }
        }
        n as u32
    }

    fn search(adj: &[Vec<u32>], c: &mut [u32], v: usize, k: u32) -> bool {
        if v == c.len() {
            return is_proper(adj, c);
        }
        for col in 0..k {
            c[v] = col;
            // Prune: check only edges to already-colored vertices.
            let ok = adj[v].iter().all(|&w| {
                let w = w as usize;
                w >= v || w >= c.len() || c[w] != col || w == v
            });
            if ok && search(adj, c, v + 1, k) {
                return true;
            }
        }
        c[v] = 0;
        false
    }

    #[test]
    fn proper_and_within_brute_force_bound() {
        let mut rng = SplitMix64::new(0xC010_0123);
        for _ in 0..150 {
            let n = (rng.below(6) + 2) as usize;
            let mut adj = vec![Vec::new(); n];
            for i in 0..n {
                for j in i + 1..n {
                    if rng.below(2) == 0 {
                        adj[i].push(j as u32);
                        adj[j].push(i as u32);
                    }
                }
            }
            let c = dsatur(&adj);
            assert!(is_proper(&adj, &c), "{adj:?} -> {c:?}");
            let used = c.iter().copied().max().map_or(0, |m| m + 1);
            let opt = chromatic(&adj);
            // DSATUR is a heuristic: assert only proper + not absurdly
            // far off (≤ optimum is wrong in general, ≤ n is trivial;
            // check the known-exact families below instead).
            assert!(used <= n as u32);
            // Heuristic bound: never worse than 2×opt in this regime.
            assert!(used <= 2 * opt.max(1), "{adj:?} -> {used} vs opt {opt}");
        }
        // Known-exact families.
        for n in 3..=7usize {
            // Clique → n colors.
            let k: Vec<Vec<u32>> = (0..n)
                .map(|i| (0..n).filter(|&j| j != i).map(|j| j as u32).collect())
                .collect();
            let c = dsatur(&k);
            assert_eq!(c.iter().copied().max().unwrap() + 1, n as u32);
            assert!(is_proper(&k, &c));
        }
        // Odd cycle → 3, even cycle → 2.
        for n in [5usize, 7] {
            let cyc: Vec<Vec<u32>> = (0..n)
                .map(|i| vec![((i + n - 1) % n) as u32, ((i + 1) % n) as u32])
                .collect();
            assert_eq!(dsatur(&cyc).iter().copied().max().unwrap() + 1, 3);
        }
        for n in [4usize, 6] {
            let cyc: Vec<Vec<u32>> = (0..n)
                .map(|i| vec![((i + n - 1) % n) as u32, ((i + 1) % n) as u32])
                .collect();
            assert_eq!(dsatur(&cyc).iter().copied().max().unwrap() + 1, 2);
        }
        // Complete bipartite K_{2,3} → 2.
        let bi = vec![
            vec![2, 3, 4],
            vec![2, 3, 4],
            vec![0, 1],
            vec![0, 1],
            vec![0, 1],
        ];
        let c = dsatur(&bi);
        assert_eq!(c.iter().copied().max().unwrap() + 1, 2);
        assert!(is_proper(&bi, &c));
        // Empty graph → all color 0.
        assert_eq!(dsatur(&[vec![], vec![]]), vec![0, 0]);
    }
}
