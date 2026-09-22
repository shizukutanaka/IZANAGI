//! Centroid decomposition — recursively split a tree at its
//! weight-balancing vertex, producing a "centroid tree" of
//! guaranteed `O(log n)` depth.
//!
//! A centroid of a tree is a vertex whose removal leaves no
//! connected piece larger than half the original size — every tree
//! has one (Jordan 1869), and removing it recursively gives the
//! centroid tree: vertex `c`'s parent is the centroid removed in the
//! larger component `c` belonged to. Because each level at least
//! halves the component, the centroid tree has depth `≤ ⌈log₂ n⌉ + 1`
//! regardless of input shape — the classic trick behind
//! `O(log n)`-per-query answers to "how many painted vertices near
//! `v`", weighted path sums, and dynamic-tree distance aggregates.
//!
//! [`Centroid::new`] accepts a forest: each connected component gets
//! its own root in the centroid forest ([`roots`]). Vertex `u`'s
//! parent link is [`parent`]; [`depth`] is its removal depth and
//! [`lca`] finds the first common centroid ancestor — the "meeting
//! point" used by distance-decomposition queries.
//!
//! ```
//! use izanagi_kit::centroid::Centroid;
//! // Path 0–1–2–3–4: vertex 2 is the centroid.
//! let c = Centroid::new(5, &[(0, 1), (1, 2), (2, 3), (3, 4)]).unwrap();
//! assert_eq!(c.roots(), &[2]);
//! assert_eq!(c.depth()[2], 0);
//! assert_eq!(c.lca(0, 4), Some(2));
//! ```
//!
//! [`roots`]: Centroid::roots
//! [`parent`]: Centroid::parent
//! [`depth`]: Centroid::depth
//! [`lca`]: Centroid::lca

/// A centroid decomposition of a forest: the centroid tree's
/// parent/children/depth vectors plus each component's root and the
/// removal order in which centroids were assigned.
#[derive(Clone, Debug)]
pub struct Centroid {
    /// `parent[v]` = centroid removed in the component `v` belonged
    /// to, or `None` when `v` is a component's top centroid.
    parent: Vec<Option<u32>>,
    /// Centroid-tree children, sorted ascending for determinism.
    children: Vec<Vec<u32>>,
    /// Removal depth of `v` (root = 0, its component pieces = 1, …).
    depth: Vec<u32>,
    /// Top centroid of each connected component, input order.
    roots: Vec<u32>,
    /// Centroids in assignment order (parents before children).
    order: Vec<u32>,
}

impl Centroid {
    /// Decompose the graph `(n, edges)` — edges are undirected
    /// `(u, v)`. Returns `None` on an out-of-range endpoint. The
    /// graph need not be a tree: cycles are fine — the decomposition
    /// only needs connected components — but the classic guarantees
    /// (depth `⌈log₂ n⌉ + 1`, halving) are stated for trees; cyclic
    /// inputs still produce a valid centroid forest since any vertex
    /// whose removal balances component pieces qualifies.
    pub fn new(n: usize, edges: &[(u32, u32)]) -> Option<Self> {
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for &(u, v) in edges {
            if u as usize >= n || v as usize >= n {
                return None;
            }
            let (u, v) = (u as usize, v as usize);
            if u != v {
                adj[u].push(v);
                adj[v].push(u);
            }
        }
        for a in &mut adj {
            a.sort_unstable();
            a.dedup();
        }
        let mut dead = vec![false; n];
        let mut sz = vec![0usize; n];
        let mut par: Vec<Option<usize>> = vec![None; n];
        let mut seen = vec![false; n];
        let mut parent: Vec<Option<u32>> = vec![None; n];
        let mut children: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut depth = vec![0u32; n];
        let mut roots = Vec::new();
        let mut order = Vec::new();
        for s in 0..n {
            if dead[s] {
                continue;
            }
            let top = order.len();
            decompose(
                s,
                &adj,
                &mut dead,
                &mut sz,
                &mut par,
                &mut seen,
                &mut parent,
                &mut children,
                &mut depth,
                &mut order,
                None,
                0,
            );
            roots.push(order[top]); // first centroid assigned in this component
        }
        Some(Centroid {
            parent,
            children,
            depth,
            roots,
            order,
        })
    }

    /// Centroid-tree parent of `v` (`None` at a component root).
    pub fn parent(&self) -> &[Option<u32>] {
        &self.parent
    }

    /// Centroid-tree children of `v`, sorted ascending.
    pub fn children(&self, v: usize) -> &[u32] {
        &self.children[v]
    }

    /// Removal depth of `v` — component root is 0.
    pub fn depth(&self) -> &[u32] {
        &self.depth
    }

    /// Top centroid of each connected component.
    pub fn roots(&self) -> &[u32] {
        &self.roots
    }

    /// Centroids in assignment order — every vertex appears exactly
    /// once, parents always before their children.
    pub fn order(&self) -> &[u32] {
        &self.order
    }

    /// First common centroid ancestor of `u` and `v` — `None` when
    /// they live in different components. `O(log n)`.
    pub fn lca(&self, u: usize, v: usize) -> Option<u32> {
        if u >= self.depth.len() || v >= self.depth.len() {
            return None;
        }
        let (mut a, mut b) = (u as u32, v as u32);
        while self.depth[a as usize] > self.depth[b as usize] {
            a = self.parent[a as usize]?;
        }
        while self.depth[b as usize] > self.depth[a as usize] {
            b = self.parent[b as usize]?;
        }
        while a != b {
            a = self.parent[a as usize]?;
            b = self.parent[b as usize]?;
        }
        Some(a)
    }
}

/// Compute subtree sizes of `root`'s component (`dead` excluded),
/// filling `sz` and `par`. Iterative post-order.
fn sizes(
    root: usize,
    adj: &[Vec<usize>],
    dead: &[bool],
    sz: &mut [usize],
    par: &mut [Option<usize>],
    seen: &mut [bool],
) -> usize {
    let mut stack = vec![root];
    par[root] = None;
    seen[root] = true;
    let mut order = Vec::new();
    while let Some(u) = stack.pop() {
        order.push(u);
        sz[u] = 1;
        for &w in &adj[u] {
            if !dead[w] && !seen[w] {
                seen[w] = true;
                par[w] = Some(u);
                stack.push(w);
            }
        }
    }
    // order is pre-order; reverse gives children before parents.
    for &u in order.iter().rev() {
        if let Some(p) = par[u] {
            sz[p] += sz[u];
        }
        seen[u] = false; // clear the visit mark for the next call
    }
    order.len()
}

/// Find the centroid inside `root`'s component: walk toward the only
/// oversized neighbor until every side is `≤ total/2`.
fn find(
    root: usize,
    total: usize,
    adj: &[Vec<usize>],
    dead: &[bool],
    sz: &[usize],
    par: &[Option<usize>],
) -> usize {
    let mut c = root;
    loop {
        // Largest side: the bigger of (a child's subtree) and
        // (everything above `c`).
        let mut heavy = total - sz[c];
        let mut heavy_at = par[c];
        for &w in &adj[c] {
            // Walk only spanning-tree edges: a non-tree neighbor's
            // `sz` is accounted inside some other side and moving to
            // it could loop on cyclic inputs.
            if !dead[w] && par[w] == Some(c) && sz[w] > heavy {
                heavy = sz[w];
                heavy_at = Some(w);
            }
        }
        if heavy * 2 <= total {
            return c;
        }
        match heavy_at {
            Some(h) => c = h,
            None => return c,
        }
    }
}

/// Recursively decompose `u`'s component: centroid `c` gets parent
/// `p` and `depth` `d`, then each live neighbor's piece decomposes
/// under `c`.
#[allow(clippy::too_many_arguments)]
fn decompose(
    u: usize,
    adj: &[Vec<usize>],
    dead: &mut [bool],
    sz: &mut [usize],
    par: &mut [Option<usize>],
    seen: &mut [bool],
    parent: &mut [Option<u32>],
    children: &mut [Vec<u32>],
    depth: &mut [u32],
    order: &mut Vec<u32>,
    p: Option<u32>,
    d: u32,
) {
    let total = sizes(u, adj, dead, sz, par, seen);
    let c = find(u, total, adj, dead, sz, par);
    dead[c] = true;
    parent[c] = p;
    depth[c] = d;
    order.push(c as u32);
    if let Some(pp) = p {
        children[pp as usize].push(c as u32);
    }
    for &w in &adj[c] {
        if !dead[w] {
            decompose(
                w,
                adj,
                dead,
                sz,
                par,
                seen,
                parent,
                children,
                depth,
                order,
                Some(c as u32),
                d + 1,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Random tree on `n` vertices: `parent[i] = rng < i`.
    fn random_tree(n: usize, rng: &mut SplitMix64) -> Vec<(u32, u32)> {
        (1..n).map(|i| (rng.below(i as u32), i as u32)).collect()
    }

    /// Pieces of `comp` after deleting `c`, via BFS on the induced
    /// subgraph. Returns the sizes of each side.
    fn pieces(n: usize, edges: &[(u32, u32)], comp: &[bool], c: usize) -> Vec<usize> {
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for &(u, v) in edges {
            adj[u as usize].push(v as usize);
            adj[v as usize].push(u as usize);
        }
        let mut seen = vec![false; n];
        seen[c] = true;
        let mut out = Vec::new();
        for s in 0..n {
            if !comp[s] || seen[s] {
                continue;
            }
            let mut q = vec![s];
            seen[s] = true;
            let mut cnt = 0;
            while let Some(u) = q.pop() {
                cnt += 1;
                for &w in &adj[u] {
                    if comp[w] && !seen[w] {
                        seen[w] = true;
                        q.push(w);
                    }
                }
            }
            out.push(cnt);
        }
        out
    }

    #[test]
    fn every_removal_halves_the_component() {
        let mut rng = SplitMix64::new(0xCE07);
        for _ in 0..200 {
            let n = (rng.below(14) + 1) as usize;
            let edges = random_tree(n, &mut rng);
            let c = Centroid::new(n, &edges).unwrap();
            // Rank each vertex by its position in `order`.
            let mut rank = vec![n; n]; // `n` = never removed (still alive)
            for (k, &v) in c.order().iter().enumerate() {
                rank[v as usize] = k;
            }
            let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
            for &(u, w) in &edges {
                adj[u as usize].push(w as usize);
                adj[w as usize].push(u as usize);
            }
            for k in 0..c.order().len() {
                let v = c.order()[k] as usize;
                // The component `v` headed = vertices alive at step k
                // reachable from `v` through alive vertices only.
                let alive: Vec<bool> = (0..n).map(|w| rank[w] >= k).collect();
                let mut comp = vec![false; n];
                let mut stack = vec![v];
                comp[v] = true;
                while let Some(u) = stack.pop() {
                    for &w in &adj[u] {
                        if alive[w] && !comp[w] {
                            comp[w] = true;
                            stack.push(w);
                        }
                    }
                }
                let total = comp.iter().filter(|&&b| b).count();
                for piece in pieces(n, &edges, &comp, v) {
                    assert!(
                        piece * 2 <= total,
                        "vertex {v} leaves a {piece}/{total} piece"
                    );
                }
            }
        }
    }

    #[test]
    fn centroid_depth_is_logarithmic() {
        let mut rng = SplitMix64::new(0xD377);
        for _ in 0..100 {
            let n = (rng.below(40) + 1) as usize;
            let edges = random_tree(n, &mut rng);
            let c = Centroid::new(n, &edges).unwrap();
            let max_d = c.depth().iter().copied().max().unwrap_or(0);
            // depth ≤ ceil(log2 n): each level at least halves.
            let mut bound = 0u32;
            while (1usize << bound) < n {
                bound += 1;
            }
            assert!(max_d <= bound, "n={n} depth {max_d} > {bound}");
        }
        // A long path: depth ≈ log2 n.
        let n = 31usize;
        let edges: Vec<(u32, u32)> = (1..n as u32).map(|i| (i - 1, i)).collect();
        let c = Centroid::new(n, &edges).unwrap();
        let max_d = c.depth().iter().copied().max().unwrap_or(0);
        assert_eq!(max_d, 4); // 31 → 15 → 7 → 3 → 1
    }

    #[test]
    fn lca_answers_and_forest() {
        // Star: center 0 is the centroid; leaves decompose under it.
        let c = Centroid::new(5, &[(0, 1), (0, 2), (0, 3), (0, 4)]).unwrap();
        assert_eq!(c.lca(1, 2), Some(0));
        assert_eq!(c.lca(3, 3), Some(3));
        // Forest: two disjoint components.
        let f = Centroid::new(6, &[(0, 1), (2, 3), (4, 5)]).unwrap();
        assert_eq!(f.roots().len(), 3);
        assert_eq!(f.lca(0, 4), None);
        // lca lies on both ancestor chains.
        let t = Centroid::new(7, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6)]).unwrap();
        let m = t.lca(0, 6).unwrap() as usize;
        let mut anc = [false; 7];
        let mut a = Some(0u32);
        while let Some(x) = a {
            anc[x as usize] = true;
            a = t.parent()[x as usize];
        }
        a = Some(6u32);
        while let Some(x) = a {
            if anc[x as usize] {
                assert_eq!(x as usize, m, "first common ancestor");
                break;
            }
            a = t.parent()[x as usize];
        }
    }

    #[test]
    fn validates_and_handles_degrees() {
        assert!(Centroid::new(3, &[(0, 5)]).is_none());
        // Single vertex and empty graph.
        assert_eq!(Centroid::new(1, &[]).unwrap().roots(), &[0]);
        assert!(Centroid::new(0, &[]).unwrap().order().is_empty());
        // Cycles are tolerated (decomposition still completes).
        let c = Centroid::new(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]).unwrap();
        assert_eq!(c.order().len(), 4);
    }
}
