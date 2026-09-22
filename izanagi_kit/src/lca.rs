//! Lowest common ancestor on a rooted forest — binary-lifting
//! `O(n log n)` preprocessing, `O(log n)` queries. Tree-shaped data is
//! everywhere in a game stack (quest prerequisite chains, skill trees,
//! tech trees, hierarchical world graphs, scenegraph ownership); this
//! is the query engine for "where do these two branches meet" and
//! "how far apart are these two nodes" — without floats, hashes, or
//! recursion.
//!
//! ```
//! use izanagi_kit::lca::Lca;
//! // Forest: 0 is a root; 1,2 under 0; 3 under 1; 4 under 2.
//! let lca = Lca::new(&[-1, 0, 0, 1, 2]);
//! assert_eq!(lca.lca(3, 4), Some(0));
//! assert_eq!(lca.lca(3, 1), Some(1));
//! assert_eq!(lca.dist(3, 4), Some(4));
//! assert_eq!(lca.ancestor(3, 2), Some(0));
//! assert_eq!(lca.depth(3), Some(2));
//! ```

/// Binary-lifting LCA table over a rooted forest.
///
/// `parent[v]` is `v`'s parent index, or `-1` for a root. Inputs are
/// validated: nodes unreachable from a root (cycles) are marked
/// invalid and every query involving them returns `None`.
pub struct Lca {
    /// `up[k][v]` = the `2^k`-th ancestor of `v` (`-1` = above root).
    up: Vec<Vec<i32>>,
    /// Depth of `v` from its root (0 for roots); `u32::MAX` = invalid.
    depth: Vec<u32>,
}

impl Lca {
    /// Build over `parent`. `O(n log n)` time and memory.
    pub fn new(parent: &[i32]) -> Self {
        let n = parent.len();
        let mut depth = vec![u32::MAX; n];
        let mut up0 = vec![-1i32; n];
        // Per-node walk to its root with a stamp; a node re-encountered
        // within the same walk is a cycle → the whole chain is invalid
        // unless it reaches an already-validated node.
        let mut stamp = vec![None; n];
        for start in 0..n {
            if depth[start] != u32::MAX {
                continue;
            }
            // Collect the path from `start` upwards until a validated
            // node, a root, or a repeat.
            let mut path = Vec::new();
            let mut cur = start;
            let root_ok = loop {
                if depth[cur] != u32::MAX {
                    // Chains onto a valid node: path's last element is
                    // its child, so base depth is +1.
                    break Some(depth[cur] + 1);
                }
                if stamp[cur] == Some(start) {
                    break None; // cycle
                }
                stamp[cur] = Some(start);
                path.push(cur);
                let p = parent[cur];
                if p < 0 {
                    break Some(0); // root
                }
                let p = p as usize;
                if p >= n {
                    break None; // out-of-range parent — treat as invalid
                }
                cur = p;
            };
            match root_ok {
                Some(base_depth) => {
                    for (i, &v) in path.iter().enumerate() {
                        // Last path element sits at `base_depth`; walk down.
                        depth[v] = base_depth + (path.len() - 1 - i) as u32;
                    }
                }
                None => {
                    // Whole path is cyclic or malformed → stays invalid.
                }
            }
        }
        for v in 0..n {
            if depth[v] != u32::MAX {
                up0[v] = parent[v];
            }
        }
        let mut levels = 1usize;
        while (1 << levels) <= n.max(1) {
            levels += 1;
        }
        let mut up = vec![up0];
        for k in 1..levels {
            let prev = &up[k - 1];
            let mut row = vec![-1i32; n];
            for v in 0..n {
                let mid = prev[v];
                if mid >= 0 {
                    row[v] = prev[mid as usize];
                }
            }
            up.push(row);
        }
        Self { up, depth }
    }

    /// `v`'s depth from its root — `None` for cyclic/invalid nodes.
    pub fn depth(&self, v: usize) -> Option<u32> {
        self.depth.get(v).copied().filter(|&d| d != u32::MAX)
    }

    /// The `k`-th ancestor of `v` — `None` if `k` exceeds `v`'s depth
    /// or `v` is invalid. `k == 0` → `Some(v)`.
    pub fn ancestor(&self, v: usize, k: usize) -> Option<usize> {
        if v >= self.depth.len() || self.depth[v] == u32::MAX {
            return None;
        }
        let mut v = v;
        let mut k = k;
        let mut bit = 0usize;
        while k > 0 {
            if k & 1 == 1 {
                let a = self.up.get(bit)?.get(v)?;
                if *a < 0 {
                    return None;
                }
                v = *a as usize;
            }
            k >>= 1;
            bit += 1;
        }
        Some(v)
    }

    /// Lowest common ancestor of `a` and `b` — `None` when either node
    /// is invalid or they live in different trees.
    pub fn lca(&self, a: usize, b: usize) -> Option<usize> {
        let da = self.depth(a)?;
        let db = self.depth(b)?;
        let (mut x, mut y) = (a, b);
        // Equalize depths.
        if da > db {
            x = self.ancestor(x, (da - db) as usize)?;
        } else if db > da {
            y = self.ancestor(y, (db - da) as usize)?;
        }
        if x == y {
            return Some(x);
        }
        // Climb while the next level up keeps them distinct.
        for k in (0..self.up.len()).rev() {
            let ax = self.up[k][x];
            let ay = self.up[k][y];
            if ax != ay {
                // Distinct rows — but both must be valid targets or -1.
                if ax >= 0 {
                    x = ax as usize;
                }
                if ay >= 0 {
                    y = ay as usize;
                }
            }
        }
        let px = self.up[0][x];
        let py = self.up[0][y];
        if px >= 0 && px == py {
            Some(px as usize)
        } else {
            None
        }
    }

    /// Path distance `depth(a) + depth(b) − 2·depth(lca)` — `None`
    /// whenever `lca` is.
    pub fn dist(&self, a: usize, b: usize) -> Option<u32> {
        let l = self.lca(a, b)?;
        let da = self.depth(a)?;
        let db = self.depth(b)?;
        let dl = self.depth(l)?;
        Some(da + db - 2 * dl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Naive oracle: walk ancestors of `a` into a set, walk `b` until
    /// a member appears.
    fn naive_lca(
        parent: &[i32],
        a: usize,
        b: usize,
        depth_of: &dyn Fn(usize) -> Option<u32>,
    ) -> Option<usize> {
        if depth_of(a).is_none() || depth_of(b).is_none() {
            return None;
        }
        let mut seen = BTreeSet::new();
        let mut cur = a;
        loop {
            seen.insert(cur);
            let p = parent[cur];
            if p < 0 || depth_of(p as usize).is_none() {
                break;
            }
            cur = p as usize;
        }
        let mut cur = b;
        loop {
            if seen.contains(&cur) {
                return Some(cur);
            }
            let p = parent[cur];
            if p < 0 || depth_of(p as usize).is_none() {
                return None;
            }
            cur = p as usize;
        }
    }

    /// Random rooted forest (each node picks a parent among earlier
    /// indices or -1).
    fn rand_forest(rng: &mut SplitMix64, n: usize) -> Vec<i32> {
        (0..n)
            .map(|i| {
                if i == 0 || rng.below(4) == 0 {
                    -1
                } else {
                    rng.below(i as u32) as i32
                }
            })
            .collect()
    }

    #[test]
    fn lca_dist_ancestor_match_naive() {
        let mut rng = SplitMix64::new(0x1CA0);
        for _ in 0..200 {
            let n = 1 + rng.below(80) as usize;
            let parent = rand_forest(&mut rng, n);
            let t = Lca::new(&parent);
            let df = |v: usize| t.depth(v);
            for _ in 0..60 {
                let a = rng.below(n as u32) as usize;
                let b = rng.below(n as u32) as usize;
                let want = naive_lca(&parent, a, b, &df);
                assert_eq!(t.lca(a, b), want, "lca({a},{b})");
                match t.dist(a, b) {
                    Some(d) => {
                        let l = want.unwrap();
                        assert_eq!(
                            d,
                            t.depth(a).unwrap() + t.depth(b).unwrap() - 2 * t.depth(l).unwrap()
                        );
                    }
                    None => assert!(want.is_none()),
                }
                // ancestor(v, k) follows the parent chain exactly.
                let k = rng.below(4) as usize;
                let mut cur = Some(a);
                for _ in 0..k {
                    cur = cur.and_then(|c| (parent[c] >= 0).then(|| parent[c] as usize));
                }
                assert_eq!(t.ancestor(a, k), cur, "ancestor({a},{k})");
            }
        }
    }

    #[test]
    fn cycles_and_ranges_are_invalid() {
        // 0 ↔ 1 cycle, 2 hangs off 1 (unreachable from any root), 3 is root.
        let t = Lca::new(&[1, 0, 1, -1]);
        assert_eq!(t.depth(0), None);
        assert_eq!(t.depth(2), None); // hangs into the cycle
        assert_eq!(t.lca(0, 3), None);
        assert_eq!(t.depth(3), Some(0));
        // Out-of-range parent is invalid, not a panic.
        let t = Lca::new(&[5, 0]);
        assert_eq!(t.depth(0), None);
        assert_eq!(t.depth(1), None);
        // Out-of-range query is None, not a panic.
        assert_eq!(t.lca(0, 99), None);
        assert_eq!(t.ancestor(0, 0), None);
        // Empty forest.
        let t = Lca::new(&[]);
        assert_eq!(t.lca(0, 0), None);
    }

    #[test]
    fn deep_chains_and_self() {
        // Chain 0→1→…→99: lca(i,j) = min(i,j) (the shallower node is
        // the ancestor), dist = |i-j|.
        let parent: Vec<i32> = (0..100).map(|i| if i == 0 { -1 } else { i - 1 }).collect();
        let t = Lca::new(&parent);
        for i in 0..100usize {
            for j in 0..100usize {
                assert_eq!(t.lca(i, j), Some(i.min(j)));
                assert_eq!(t.dist(i, j), Some(i.abs_diff(j) as u32));
            }
        }
        // Self queries and k=0.
        assert_eq!(t.lca(42, 42), Some(42));
        assert_eq!(t.ancestor(42, 0), Some(42));
        assert_eq!(t.dist(42, 42), Some(0));
        // Beyond-root ancestor → None.
        assert_eq!(t.ancestor(10, 11), None);
    }
}
