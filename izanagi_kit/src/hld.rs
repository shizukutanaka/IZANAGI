//! Heavy-light decomposition (Sleator & Tarjan 1983) — flattens a
//! rooted tree into an array where every root-to-node path decomposes
//! into `O(log n)` contiguous segments, so any associative range
//! query (`segtree`, `fenwick`) answers **path** queries on trees.
//! Also makes each subtree a single contiguous segment.
//!
//! Deterministic: the heavy child is the maximum-size child with
//! lowest index; segments are returned in `u → v` order.
//!
//! ```
//! use izanagi_kit::hld::Hld;
//! // 0
//! // ├── 1 ── 3
//! // └── 2 ── 4 ── 5
//! let hld = Hld::new(&[-1, 0, 0, 1, 2, 4]).unwrap();
//! assert_eq!(hld.path_vertices(3, 5), Some(vec![3, 1, 0, 2, 4, 5]));
//! // Decomposition puts {0,2,4,5} on the heavy chain first.
//! assert_eq!(hld.subtree_segment(2), Some((1, 4)));
//! ```

/// A rooted tree flattened by heavy-light decomposition. Positions in
/// the flattened order are `0..n`; `order()[p]` is the vertex at
/// position `p`.
pub struct Hld {
    n: usize,
    parent: Vec<i32>,
    /// Position of each vertex in the flat array.
    pos: Vec<u32>,
    /// Chain head (top vertex) of each vertex's heavy chain.
    head: Vec<u32>,
    /// Flat-array order: `order[pos[v]] == v`.
    order: Vec<u32>,
    /// Subtree size of each vertex.
    size: Vec<u32>,
}

impl Hld {
    /// Build over `parent[v]` (`-1` marks a root; forests are fine).
    /// `None` when any `parent[v]` is out of range or the structure
    /// contains a cycle.
    pub fn new(parent: &[i32]) -> Option<Self> {
        let n = parent.len();
        for &p in parent {
            if p < -1 || p >= n as i32 {
                return None;
            }
        }
        // Children adjacency in index order (deterministic).
        let mut children: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut roots = Vec::new();
        for (v, &p) in parent.iter().enumerate() {
            if p < 0 {
                roots.push(v as u32);
            } else {
                children[p as usize].push(v as u32);
            }
        }
        // Validate: every vertex reachable from a root, no repeats.
        // Iterative post-order computing subtree sizes.
        let mut size = vec![0u32; n];
        let mut seen = vec![false; n];
        let mut stack: Vec<(u32, bool)> = Vec::new();
        for &r in &roots {
            stack.push((r, false));
            while let Some((v, done)) = stack.pop() {
                let v = v as usize;
                if done {
                    size[v] = 1;
                    for &c in &children[v] {
                        size[v] += size[c as usize];
                    }
                    continue;
                }
                if seen[v] {
                    return None; // node entered twice ⇒ not a tree
                }
                seen[v] = true;
                stack.push((v as u32, true));
                for &c in &children[v] {
                    stack.push((c, false));
                }
            }
        }
        if seen.iter().any(|&s| !s) {
            return None; // unreachable ⇒ cycle
        }
        // Heavy child: max-size child, lowest index on ties.
        let mut heavy = vec![None; n];
        for (v, kids) in children.iter().enumerate() {
            heavy[v] = kids.iter().max_by_key(|&&c| size[c as usize]).copied();
        }
        // Decompose: follow heavy chains, push light children.
        let mut pos = vec![0u32; n];
        let mut head = vec![0u32; n];
        let mut order = vec![0u32; n];
        let mut t = 0u32;
        let mut stack: Vec<(u32, u32)> = roots.iter().map(|&r| (r, r)).collect();
        while let Some((v, h)) = stack.pop() {
            // Walk down the heavy spine assigning positions.
            let mut cur = v;
            let ch = h;
            loop {
                let cv = cur as usize;
                pos[cv] = t;
                head[cv] = ch;
                order[t as usize] = cur;
                t += 1;
                // Light children start new chains rooted at themselves.
                for &c in &children[cv] {
                    if Some(c) != heavy[cv] {
                        stack.push((c, c));
                    }
                }
                match heavy[cv] {
                    Some(hv) => cur = hv,
                    None => break,
                }
            }
        }
        Some(Self {
            n,
            parent: parent.to_vec(),
            pos,
            head,
            order,
            size,
        })
    }

    /// Position of `v` in the flat array.
    pub fn pos(&self, v: usize) -> Option<usize> {
        if v < self.n {
            Some(self.pos[v] as usize)
        } else {
            None
        }
    }

    /// Vertex at flat position `p`.
    pub fn vertex(&self, p: usize) -> Option<u32> {
        self.order.get(p).copied()
    }

    /// The subtree of `v` as one contiguous `[lo, hi)` range in the
    /// flat array (preorder layout makes subtrees contiguous).
    pub fn subtree_segment(&self, v: usize) -> Option<(usize, usize)> {
        if v >= self.n {
            return None;
        }
        let lo = self.pos[v] as usize;
        Some((lo, lo + self.size[v] as usize))
    }

    /// The `u → v` path as `O(log n)` flat-array segments.
    ///
    /// Returns `(lo, hi)` half-open ranges **in path order**; inside a
    /// segment the vertices occupy `pos[lo..hi]` contiguously but the
    /// path itself may traverse them in either direction — use
    /// [`Self::path_vertices`] when direction matters, or aggregate
    /// commutative operations directly over the ranges.
    pub fn path_segments(&self, u: usize, v: usize) -> Option<Vec<(usize, usize)>> {
        if u >= self.n || v >= self.n {
            return None;
        }
        let mut u = u as u32;
        let mut v = v as u32;
        let mut a_side: Vec<(usize, usize)> = Vec::new(); // u → lca
        let mut b_side: Vec<(usize, usize)> = Vec::new(); // v → lca
        while self.head[u as usize] != self.head[v as usize] {
            let hu = self.head[u as usize];
            let hv = self.head[v as usize];
            if self.pos[hu as usize] > self.pos[hv as usize] {
                // u's chain top is deeper: emit [hu ..= u] segment.
                a_side.push((
                    self.pos[hu as usize] as usize,
                    self.pos[u as usize] as usize + 1,
                ));
                u = self.parent[hu as usize] as u32;
            } else {
                b_side.push((
                    self.pos[hv as usize] as usize,
                    self.pos[v as usize] as usize + 1,
                ));
                v = self.parent[hv as usize] as u32;
            }
        }
        // Same chain: lca is the shallower of the two.
        let (lo, hi) = (
            self.pos[u as usize].min(self.pos[v as usize]) as usize,
            self.pos[u as usize].max(self.pos[v as usize]) as usize + 1,
        );
        let mut out = a_side;
        out.push((lo, hi));
        out.extend(b_side.into_iter().rev());
        Some(out)
    }

    /// Vertices on the `u → v` path in order (lca included once).
    pub fn path_vertices(&self, u: usize, v: usize) -> Option<Vec<u32>> {
        if u >= self.n || v >= self.n {
            return None;
        }
        let mut out = Vec::new();
        let mut tail: Vec<(u32, u32)> = Vec::new(); // v-side pos ranges
        let mut uu = u as u32;
        let mut vv = v as u32;
        while self.head[uu as usize] != self.head[vv as usize] {
            let hu = self.head[uu as usize];
            let hv = self.head[vv as usize];
            if self.pos[hu as usize] > self.pos[hv as usize] {
                // u-side segment: descend pos(uu) → pos(hu).
                for p in (self.pos[hu as usize]..=self.pos[uu as usize]).rev() {
                    out.push(self.order[p as usize]);
                }
                uu = self.parent[hu as usize] as u32;
            } else {
                // v-side segment: ascend pos(hv) → pos(vv) once the
                // tail is replayed in reverse collection order.
                tail.push((self.pos[hv as usize], self.pos[vv as usize]));
                vv = self.parent[hv as usize] as u32;
            }
        }
        // Same chain: bridge goes lca → deeper endpoint.
        let (lo, hi) = (
            self.pos[uu as usize].min(self.pos[vv as usize]),
            self.pos[uu as usize].max(self.pos[vv as usize]),
        );
        if self.pos[uu as usize] > self.pos[vv as usize] {
            for p in (lo..=hi).rev() {
                out.push(self.order[p as usize]);
            }
        } else {
            for p in lo..=hi {
                out.push(self.order[p as usize]);
            }
        }
        for &(lo, hi) in tail.iter().rev() {
            for p in lo..=hi {
                out.push(self.order[p as usize]);
            }
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive oracle: climb ancestors to find the path u→v.
    fn oracle_path(parent: &[i32], u: usize, v: usize) -> Vec<u32> {
        let mut ua = Vec::new();
        let mut x = u as i32;
        while x >= 0 {
            ua.push(x as usize);
            x = parent[x as usize];
        }
        let mut vb = Vec::new();
        let mut x = v as i32;
        while x >= 0 {
            vb.push(x as usize);
            x = parent[x as usize];
        }
        // Meet = first common ancestor from u's side.
        let meet = *ua.iter().find(|a| vb.contains(a)).unwrap();
        let mut out: Vec<u32> = ua
            .iter()
            .take_while(|&&a| a != meet)
            .map(|&a| a as u32)
            .collect();
        out.push(meet as u32);
        let mut rest: Vec<u32> = vb
            .iter()
            .take_while(|&&b| b != meet)
            .map(|&b| b as u32)
            .collect();
        rest.reverse();
        out.extend(rest);
        out
    }

    /// Naive subtree membership via ancestor climbing.
    fn in_subtree(parent: &[i32], root: usize, v: usize) -> bool {
        let mut x = v as i32;
        while x >= 0 {
            if x as usize == root {
                return true;
            }
            x = parent[x as usize];
        }
        false
    }

    #[test]
    fn path_vertices_match_ancestor_oracle() {
        let mut rng = SplitMix64::new(0x11D0);
        for _ in 0..150 {
            let n = (rng.below(20) + 2) as usize;
            let mut parent = vec![-1i32; n];
            for (v, p) in parent.iter_mut().enumerate().skip(1) {
                *p = rng.below(v as u32) as i32;
            }
            let hld = Hld::new(&parent).unwrap();
            for _ in 0..20 {
                let u = rng.below(n as u32) as usize;
                let v = rng.below(n as u32) as usize;
                assert_eq!(
                    hld.path_vertices(u, v).unwrap(),
                    oracle_path(&parent, u, v),
                    "u={u} v={v} parent={parent:?}"
                );
            }
        }
    }

    #[test]
    fn subtree_segments_cover_exactly_the_subtree() {
        let mut rng = SplitMix64::new(0xBEE5);
        for _ in 0..100 {
            let n = (rng.below(20) + 2) as usize;
            let mut parent = vec![-1i32; n];
            for (v, p) in parent.iter_mut().enumerate().skip(1) {
                *p = rng.below(v as u32) as i32;
            }
            let hld = Hld::new(&parent).unwrap();
            for r in 0..n {
                let (lo, hi) = hld.subtree_segment(r).unwrap();
                let covered: std::collections::BTreeSet<u32> =
                    (lo..hi).map(|p| hld.vertex(p).unwrap()).collect();
                let want: std::collections::BTreeSet<u32> = (0..n)
                    .filter(|&v| in_subtree(&parent, r, v))
                    .map(|v| v as u32)
                    .collect();
                assert_eq!(covered, want, "r={r} parent={parent:?}");
            }
        }
    }

    #[test]
    fn path_segments_cover_exactly_path_vertices() {
        let mut rng = SplitMix64::new(0x5E66);
        for _ in 0..100 {
            let n = (rng.below(20) + 2) as usize;
            let mut parent = vec![-1i32; n];
            for (v, p) in parent.iter_mut().enumerate().skip(1) {
                *p = rng.below(v as u32) as i32;
            }
            let hld = Hld::new(&parent).unwrap();
            let u = rng.below(n as u32) as usize;
            let v = rng.below(n as u32) as usize;
            let segs = hld.path_segments(u, v).unwrap();
            let mut covered: Vec<u32> = segs
                .iter()
                .flat_map(|&(lo, hi)| (lo..hi).map(|p| hld.vertex(p).unwrap()))
                .collect();
            covered.sort_unstable();
            let mut want = hld.path_vertices(u, v).unwrap();
            want.sort_unstable();
            assert_eq!(covered, want);
            // Segments stay O(log n).
            assert!(segs.len() <= 2 * (usize::BITS as usize));
        }
    }

    #[test]
    fn rejects_cycles_and_bad_parents() {
        assert!(Hld::new(&[0]).is_none()); // self-loop root claim
        assert!(Hld::new(&[-1, -1, 1]).is_some()); // forest
        assert!(Hld::new(&[1, 0]).is_none()); // 2-cycle, no root
        assert!(Hld::new(&[-1, 5]).is_none()); // out-of-range parent
    }

    #[test]
    fn pos_and_vertex_are_inverse() {
        let hld = Hld::new(&[-1, 0, 0, 1, 2]).unwrap();
        for v in 0..5 {
            assert_eq!(hld.vertex(hld.pos(v).unwrap()), Some(v as u32));
        }
    }
}
