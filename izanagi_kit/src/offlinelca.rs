//! Tarjan's offline LCA — answers a whole batch of `(u, v)`
//! ancestor queries in `O((n + q) · α(n))` instead of paying
//! `O(log n)` per query against [`crate::lca`]. The disjoint-set
//! plus single-DFS trick: after vertex `u` finishes its subtree
//! it is *black*; for every query `(u, w)` with `w` already
//! black, the LCA is `ancestor[find(w)]`.
//!
//! Input is the same rooted-forest convention as `lca`:
//! `parent[v]` is the parent index or `-1` for a root. Vertices
//! unreachable from any root (cycles) are never visited, and
//! cross-tree queries answer `None` — both matching `lca`'s
//! `Option` semantics. Answers come back in query order.
//!
//! ```
//! use izanagi_kit::offlinelca::offline_lca;
//! //     0
//! //    / \
//! //   1   2
//! //  / \
//! // 3   4
//! let parent = [-1, 0, 0, 1, 1];
//! let ans = offline_lca(&parent, &[(3, 4), (3, 2), (1, 3)]);
//! assert_eq!(ans, vec![Some(1), Some(0), Some(1)]);
//! ```

struct Dsu {
    p: Vec<usize>,
}

impl Dsu {
    fn new(n: usize) -> Self {
        Dsu {
            p: (0..n).collect(),
        }
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.p[r] != r {
            r = self.p[r];
        }
        let mut c = x;
        while self.p[c] != r {
            let n = self.p[c];
            self.p[c] = r;
            c = n;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        self.p[rb] = ra;
    }
}

/// Offline least-common-ancestor over a rooted forest.
/// `parent[v] = -1` marks roots; answers are in query order.
pub fn offline_lca(parent: &[i32], queries: &[(usize, usize)]) -> Vec<Option<usize>> {
    let n = parent.len();
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for (v, &p) in parent.iter().enumerate() {
        if p < 0 {
            roots.push(v);
        } else if (p as usize) < n {
            kids[p as usize].push(v);
        }
        // p >= n: invalid parent — the vertex is unreachable,
        // never visited, and every query on it answers None
        // (matching `lca`'s invalid mark).
    }
    let mut by: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (qi, &(a, b)) in queries.iter().enumerate() {
        if a < n {
            by[a].push(qi);
        }
        if b < n {
            by[b].push(qi);
        }
    }
    let mut ans = vec![None; queries.len()];
    let mut dsu = Dsu::new(n);
    let mut ancestor: Vec<usize> = (0..n).collect();
    let mut black = vec![false; n];
    // Which tree a visited vertex belongs to (-1 = unvisited) —
    // DSU state persists across roots, so without this guard a
    // black vertex from an earlier tree would answer a
    // cross-tree query with a phantom ancestor.
    let mut tree_of = vec![-1i32; n];
    // Iterative DFS: stack of (vertex, next-child-cursor). A node
    // is grey on entry and black on exit.
    for (ti, &root) in roots.iter().enumerate() {
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        tree_of[root] = ti as i32;
        while let Some(&mut (u, ref mut ci)) = stack.last_mut() {
            if *ci < kids[u].len() {
                let c = kids[u][*ci];
                *ci += 1;
                stack.push((c, 0));
                dsu.p[c] = c;
                ancestor[c] = c;
                tree_of[c] = ti as i32;
            } else {
                // u is now black.
                black[u] = true;
                for &qi in &by[u] {
                    let (a, b) = queries[qi];
                    let w = if a == u { b } else { a };
                    if w < n && black[w] && tree_of[w] == tree_of[u] {
                        let r = dsu.find(w);
                        ans[qi] = Some(ancestor[r]);
                    }
                }
                stack.pop();
                if let Some(&(p, _)) = stack.last() {
                    dsu.union(p, u);
                    let r = dsu.find(p);
                    ancestor[r] = p;
                }
            }
        }
    }
    ans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lca::Lca;
    use crate::rng::SplitMix64;

    fn random_forest(rng: &mut SplitMix64, n: usize) -> Vec<i32> {
        // v's parent is a random earlier vertex or -1 (root).
        (0..n)
            .map(|v| {
                if v == 0 || rng.below(4) == 0 {
                    -1
                } else {
                    rng.below(v as u32) as i32
                }
            })
            .collect()
    }

    #[test]
    fn tree_examples() {
        let parent = [-1, 0, 0, 1, 1];
        assert_eq!(
            offline_lca(&parent, &[(3, 4), (3, 2), (1, 3), (0, 4), (4, 4)]),
            vec![Some(1), Some(0), Some(1), Some(0), Some(4)]
        );
    }

    #[test]
    fn matches_binary_lifting_oracle() {
        let mut rng = SplitMix64::new(1234);
        for _ in 0..100 {
            let n = 1 + rng.below(30) as usize;
            let parent = random_forest(&mut rng, n);
            let lca = Lca::new(&parent);
            let queries: Vec<(usize, usize)> = (0..40)
                .map(|_| (rng.below(n as u32) as usize, rng.below(n as u32) as usize))
                .collect();
            let got = offline_lca(&parent, &queries);
            let want: Vec<Option<usize>> = queries.iter().map(|&(a, b)| lca.lca(a, b)).collect();
            assert_eq!(got, want, "parent={parent:?} queries={queries:?}");
        }
    }

    #[test]
    fn cross_tree_and_cycles() {
        // Two roots: cross-tree queries are None.
        let parent = [-1, 0, -1, 2];
        assert_eq!(
            offline_lca(&parent, &[(1, 3), (1, 0), (3, 2)]),
            vec![None, Some(0), Some(2)]
        );
        // Cyclic vertices are unreachable: queries on them are None.
        let parent = [1, 0]; // 0->1->0 cycle, no root
        assert_eq!(offline_lca(&parent, &[(0, 1)]), vec![None]);
    }

    #[test]
    fn empty_queries_and_single() {
        assert_eq!(offline_lca(&[-1], &[]), Vec::<Option<usize>>::new());
        assert_eq!(offline_lca(&[-1], &[(0, 0)]), vec![Some(0)]);
    }

    #[test]
    fn deep_chain_no_recursion() {
        // A 5000-deep chain must not overflow the call stack.
        let n = 5000;
        let parent: Vec<i32> = (0..n as i32).map(|i| i - 1).collect();
        let ans = offline_lca(&parent, &[(0, n - 1), (n - 1, n - 2), (n - 1, n - 1)]);
        assert_eq!(ans, vec![Some(0), Some(n - 2), Some(n - 1)]);
    }
}
