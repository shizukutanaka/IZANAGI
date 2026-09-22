//! Weighted union-find — a DSU that also tracks a per-element
//! *potential*, so `unite(u, v, w)` asserts `pot[v] − pot[u] == w`
//! inside each component. Contradictions are detected instead of
//! silently merged (unlike `graph::UnionFind`, which carries no
//! weights).
//!
//! Use cases: relative-distance / height constraints between entities
//! ("this cell is 3 rows above that one"), offset consistency between
//! clock domains, or `x − y ≤ c` style difference-logic fragments.
//!
//! `pot[x]` is only defined relative to a component root — absolute
//! values carry no meaning; `diff(u, v)` returns `pot[v] − pot[u]` or
//! `None` for different components.
//!
//! ```
//! use izanagi_kit::wdsu::WeightedDsu;
//! let mut d = WeightedDsu::new(3);
//! assert!(d.unite(0, 1, 5));     // pot[1] - pot[0] = 5
//! assert!(d.unite(1, 2, -2));    // pot[2] - pot[1] = -2
//! assert_eq!(d.diff(0, 2), Some(3));
//! assert!(!d.unite(0, 2, 0));    // contradiction: would need 3 = 0
//! assert_eq!(d.diff(0, 1), Some(5));
//! ```

/// Potential-annotated union-find over `0..n`.
pub struct WeightedDsu {
    parent: Vec<u32>,
    size: Vec<u32>,
    /// `weight[x]` = `pot[x] − pot[parent[x]]` (0 when `x` is a root).
    weight: Vec<i64>,
}

impl WeightedDsu {
    /// `n` isolated elements, each with potential 0.
    pub fn new(n: usize) -> Self {
        WeightedDsu {
            parent: (0..n as u32).collect(),
            size: vec![1; n],
            weight: vec![0; n],
        }
    }

    /// Root of `x`, compressing the path and accumulating `weight`
    /// into `pot[x] − pot[root]` along it.
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] as usize == x {
            return x;
        }
        let p = self.parent[x] as usize;
        let r = self.find(p);
        self.weight[x] += self.weight[p];
        self.parent[x] = r as u32;
        r
    }

    /// `pot[x] − pot[root(x)]` — requires `find(x)` first.
    fn rel(&mut self, x: usize) -> i64 {
        self.find(x);
        self.weight[x]
    }

    /// Whether `u` and `v` share a component.
    pub fn same(&mut self, u: usize, v: usize) -> bool {
        self.find(u) == self.find(v)
    }

    /// `pot[v] − pot[u]` when `u`, `v` are connected, else `None`.
    pub fn diff(&mut self, u: usize, v: usize) -> Option<i64> {
        if !self.same(u, v) {
            return None;
        }
        Some(self.rel(v) - self.rel(u))
    }

    /// Merge `u` and `v` under the constraint `pot[v] − pot[u] == w`.
    ///
    /// Returns `false` when the constraint contradicts already-known
    /// potentials (component unchanged); `true` on success — including
    /// when the constraint was already implied.
    pub fn unite(&mut self, u: usize, v: usize, w: i64) -> bool {
        let ru = self.find(u);
        let rv = self.find(v);
        if ru == rv {
            return self.rel(v) - self.rel(u) == w;
        }
        // Attach the smaller root under the larger, setting the new
        // edge's weight so every potential stays consistent:
        //   weight[ru] = pot[ru] − pot[rv] = rel[v] − rel[u] − w
        // (pot[v] − pot[u] = (pot[rv] − pot[ru]) + rel[v] − rel[u] = w).
        let (ru, rv, wu, wv, w) = (ru, rv, self.weight[u], self.weight[v], w);
        if self.size[ru] < self.size[rv] {
            // ru under rv.
            self.parent[ru] = rv as u32;
            self.weight[ru] = wv - wu - w;
            self.size[rv] += self.size[ru];
        } else {
            // rv under ru: weight[rv] = pot[rv] − pot[ru] = wu + w − wv.
            self.parent[rv] = ru as u32;
            self.weight[rv] = wu + w - wv;
            self.size[ru] += self.size[rv];
        }
        true
    }

    /// Number of connected components.
    pub fn components(&mut self) -> usize {
        let n = self.parent.len();
        (0..n).filter(|&i| self.find(i) == i).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: per-component potentials over the accepted-constraint
    /// graph, tracked incrementally. Returns `pot[v] - pot[u]` when
    /// `u`, `v` share a component, else `None`.
    fn oracle(n: usize, ops: &[(usize, usize, i64)], u: usize, v: usize) -> Option<i64> {
        let mut comp: Vec<Option<usize>> = vec![None; n];
        let mut pot: Vec<i64> = vec![0; n];
        let mut next_comp = 0usize;
        for &(a, b, w) in ops {
            let (ca, cb) = (comp[a], comp[b]);
            match (ca, cb) {
                (None, None) => {
                    // New component: pot[a] = 0, pot[b] = w.
                    pot[a] = 0;
                    pot[b] = w;
                    comp[a] = Some(next_comp);
                    comp[b] = Some(next_comp);
                    next_comp += 1;
                }
                (Some(c), None) => {
                    pot[b] = pot[a] + w;
                    comp[b] = Some(c);
                }
                (None, Some(c)) => {
                    pot[a] = pot[b] - w;
                    comp[a] = Some(c);
                }
                (Some(ca), Some(cb)) => {
                    if ca == cb {
                        if pot[b] - pot[a] != w {
                            continue; // contradiction — rejected
                        }
                    } else {
                        // Merge cb into ca: shift pot of cb so the edge holds.
                        let shift = pot[a] + w - pot[b];
                        for i in 0..n {
                            if comp[i] == Some(cb) {
                                comp[i] = Some(ca);
                                pot[i] += shift;
                            }
                        }
                    }
                }
            }
        }
        match (comp[u], comp[v]) {
            (Some(a), Some(b)) if a == b => Some(pot[v] - pot[u]),
            _ => None,
        }
    }

    #[test]
    fn ops_match_bfs_potential_oracle() {
        let mut rng = SplitMix64::new(0xD5A1);
        for _ in 0..200 {
            let n = (rng.below(8) + 2) as usize;
            let mut d = WeightedDsu::new(n);
            let mut ops: Vec<(usize, usize, i64)> = Vec::new();
            let mut accepted: Vec<(usize, usize, i64)> = Vec::new();
            for _ in 0..(rng.below(20) + 4) {
                let u = rng.below(n as u32) as usize;
                let v = rng.below(n as u32) as usize;
                let w = rng.below(21) as i64 - 10;
                ops.push((u, v, w));
                let ok = d.unite(u, v, w);
                // Acceptance oracle: independent component replay. A
                // self-edge is trivially the same vertex — implied
                // diff 0 regardless of component membership.
                let implied = if u == v {
                    Some(0)
                } else {
                    oracle(n, &accepted, u, v)
                };
                let expect_ok = match implied {
                    None => true, // different components — always accepted
                    Some(dif) => dif == w,
                };
                assert_eq!(ok, expect_ok, "ops={ops:?} n={n}");
                if ok {
                    accepted.push((u, v, w));
                }
                // diff agrees with the BFS oracle after each op.
                for _ in 0..4 {
                    let x = rng.below(n as u32) as usize;
                    let y = rng.below(n as u32) as usize;
                    let want = if x == y {
                        Some(0)
                    } else {
                        oracle(n, &accepted, x, y)
                    };
                    assert_eq!(d.diff(x, y), want, "x={x} y={y}");
                }
            }
        }
    }

    #[test]
    fn self_edge_constraints() {
        let mut d = WeightedDsu::new(2);
        assert!(d.unite(0, 0, 0)); // trivially consistent
        assert!(!d.unite(0, 0, 5)); // self-contradiction
        assert!(d.unite(0, 1, 7));
        assert!(!d.unite(1, 1, -3));
        assert_eq!(d.diff(0, 0), Some(0));
    }

    #[test]
    fn transitive_contradiction() {
        let mut d = WeightedDsu::new(3);
        assert!(d.unite(0, 1, 4));
        assert!(d.unite(1, 2, 1));
        assert!(!d.unite(0, 2, 6)); // implies 5, not 6
        assert!(d.unite(0, 2, 5)); // implied — accepted no-op
        assert_eq!(d.components(), 1);
        // New element merges with correct shift.
        let mut d = WeightedDsu::new(4);
        d.unite(0, 1, 4);
        d.unite(2, 3, 10);
        assert!(d.unite(1, 2, 1));
        assert_eq!(d.diff(0, 3), Some(15));
        assert_eq!(d.diff(3, 0), Some(-15));
    }

    #[test]
    fn edge_cases() {
        let mut d = WeightedDsu::new(0);
        assert_eq!(d.components(), 0);
        let mut d = WeightedDsu::new(1);
        assert_eq!(d.diff(0, 0), Some(0));
        assert_eq!(d.components(), 1);
    }
}
