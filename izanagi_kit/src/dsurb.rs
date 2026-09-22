//! Rollback union-find — connectivity you can *undo*.
//!
//! Plain union-find is irreversible, which is exactly what you don't
//! want for "what if this edge existed?" queries: hypothetically merge
//! regions, ask [`DsuRollback::connected`], then [`DsuRollback::rollback`]
//! to the snapshot. One data structure serves an unbounded stack of
//! hypotheticals in `O(α(n))` each.
//!
//! Cost of undo: **no path compression** — compressing rewrites parents
//! arbitrarily deep in the history and cannot be rolled back cheaply.
//! Union-by-size alone keeps find at `O(log n)`, plenty at game scale.
//!
//! ```
//! use izanagi_kit::dsurb::DsuRollback;
//! let mut d = DsuRollback::new(4);
//! d.union(0, 1);
//! let snap = d.snapshot();
//! d.union(1, 2);
//! assert!(d.connected(0, 2));
//! d.rollback(snap);
//! assert!(!d.connected(0, 2));
//! ```

/// Union-find with explicit snapshots and rollback.
pub struct DsuRollback {
    parent: Vec<u32>,
    size: Vec<u32>,
    comps: u32,
    /// Journal of `(child, old_size_of_root)` — one entry per union that
    /// actually merged two components.
    log: Vec<(u32, u32)>,
}

impl DsuRollback {
    /// `n` singleton components.
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n as u32).collect(),
            size: vec![1; n],
            comps: n as u32,
            log: Vec::new(),
        }
    }

    /// Root representative of `v` — plain walk, no compression.
    pub fn find(&self, mut v: u32) -> u32 {
        while self.parent[v as usize] != v {
            v = self.parent[v as usize];
        }
        v
    }

    /// Merge the components of `a` and `b` (smaller under larger).
    /// Returns whether they were separate — `false` unions are no-ops
    /// and do not push the journal.
    pub fn union(&mut self, a: u32, b: u32) -> bool {
        let mut ra = self.find(a);
        let mut rb = self.find(b);
        if ra == rb {
            return false;
        }
        if self.size[ra as usize] < self.size[rb as usize] {
            std::mem::swap(&mut ra, &mut rb);
        }
        self.parent[rb as usize] = ra;
        self.log.push((rb, self.size[ra as usize]));
        self.size[ra as usize] += self.size[rb as usize];
        self.comps -= 1;
        true
    }

    /// Whether `a` and `b` share a component.
    pub fn connected(&self, a: u32, b: u32) -> bool {
        self.find(a) == self.find(b)
    }

    /// Current component count.
    pub fn components(&self) -> u32 {
        self.comps
    }

    /// Opaque snapshot — pass to [`DsuRollback::rollback`]. Cheap: just a
    /// journal length.
    pub fn snapshot(&self) -> usize {
        self.log.len()
    }

    /// Undo every `union` performed since `snap` was taken. `snap` must
    /// have been produced by [`DsuRollback::snapshot`]; rolling back to a
    /// *newer* snapshot than the current state is a no-op.
    pub fn rollback(&mut self, snap: usize) {
        while self.log.len() > snap {
            let Some((child, old_root_size)) = self.log.pop() else {
                break;
            };
            let root = self.parent[child as usize];
            self.parent[child as usize] = child;
            self.size[root as usize] = old_root_size;
            self.comps += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    /// Reference: recompute connectivity by BFS over only the first `k`
    /// unions — a completely independent formulation.
    fn oracle_connected(n: usize, edges: &[(u32, u32)], k: usize, a: u32, b: u32) -> bool {
        let mut adj: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &(u, v) in edges.iter().take(k) {
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }
        let mut seen = vec![false; n];
        let mut q = vec![a];
        seen[a as usize] = true;
        while let Some(u) = q.pop() {
            if u == b {
                return true;
            }
            for &w in adj.get(&u).cloned().unwrap_or_default().iter() {
                if !seen[w as usize] {
                    seen[w as usize] = true;
                    q.push(w);
                }
            }
        }
        false
    }

    #[test]
    fn rollback_restores_exact_state() {
        let mut rng = SplitMix64::new(0xD5A0_BACC);
        for _ in 0..150 {
            let n = (rng.below(9) + 2) as usize;
            let ecount = (rng.below(10) + 1) as usize;
            let edges: Vec<(u32, u32)> = (0..ecount)
                .map(|_| (rng.below(n as u32), rng.below(n as u32)))
                .collect();
            let mut d = DsuRollback::new(n);
            let mut snaps = vec![d.snapshot()];
            for (i, &(a, b)) in edges.iter().enumerate() {
                d.union(a, b);
                snaps.push(d.snapshot());
                let _ = i;
            }
            // Pick a random rollback point and replay every query.
            let cut = rng.below((ecount + 1) as u32) as usize;
            d.rollback(snaps[cut]);
            for _ in 0..10 {
                let (a, b) = (rng.below(n as u32), rng.below(n as u32));
                assert_eq!(
                    d.connected(a, b),
                    oracle_connected(n, &edges, cut, a, b),
                    "cut={cut} {a}~{b}"
                );
            }
            // Component count: independent recomputation.
            let want = {
                let mut d2 = DsuRollback::new(n);
                for &(a, b) in edges.iter().take(cut) {
                    d2.union(a, b);
                }
                d2.components()
            };
            assert_eq!(d.components(), want);
        }
        // Rollback to a newer snapshot than current state: no-op.
        let mut d = DsuRollback::new(2);
        let snap = d.snapshot();
        d.rollback(snap);
        assert_eq!(d.components(), 2);
    }
}
