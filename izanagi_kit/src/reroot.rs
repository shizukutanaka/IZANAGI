//! Rerooting DP (全方位木DP): for every vertex `v` of a tree, the aggregate
//! "as if the tree were rooted at `v`" — in `O(n)` total, instead of one
//! `O(n)` rooted DP per vertex. The standard technique for eccentricities,
//! sum-of-distances, "subtree of every root" problems.
//!
//! The caller supplies a commutative monoid (`unit`, `merge`), a per-vertex
//! `vertex` weight, and `lift(agg, from, to)` — how a subtree aggregate
//! crosses a directed edge. Two sweeps do the rest: a post-order pass gives
//! `down[v]` (aggregate of the subtree below `v`), then a pre-order pass
//! gives every child its "rest of the tree" contribution via prefix/suffix
//! merges over siblings — so each edge is folded exactly twice.
//!
//! `adj` must be a **tree** (undirected, `n−1` edges, connected — the usual
//! competitive-programming input shape, both directions listed). Anything
//! else returns `Some`-shaped garbage: cycles feed a vertex into its own
//! aggregate; forests give each component a `unit`-rooted answer that is
//! *locally* correct but probably not what was meant. The function is total
//! on that contract: malformed shapes still return a `Vec` of `adj.len()`.
//!
//! ```
//! use izanagi_kit::reroot::reroot;
//!
//! // Sum of distances from every vertex — the classic reroot demo. The
//! // monoid is a (sum, count) pair: lifting across an edge adds the
//! // subtree's *size* to its distance sum (every member moves one hop
//! // farther), not one. tree: 0-1-2 path → distances [3, 2, 3].
//! let adj = vec![vec![1], vec![0, 2], vec![1]];
//! let ans = reroot(
//!     &adj,
//!     &[(0i64, 1i64); 3],              // every vertex weighs one
//!     (0i64, 0i64),                    // monoid unit
//!     |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2), // merge
//!     |(s, c), _from, _to| (s + c, c), // lift: +size across the edge
//! );
//! assert_eq!(ans.iter().map(|&(s, _)| s).collect::<Vec<_>>(), vec![3, 2, 3]);
//! ```

/// Rerooting DP over a tree — see the module docs for the contract.
///
/// - `adj[u]` lists `u`'s neighbors (both directions of each edge).
/// - `vertex[u]` is the weight folded in at `u` itself.
/// - `unit` is the monoid identity; `merge` must be commutative and
///   associative (the result is built by bracketing in traversal order, so
///   a non-commutative merge makes the output order-dependent — which here
///   means *adjacency-list*-dependent).
/// - `lift(agg, from, to)` re-roots a subtree aggregate across edge
///   `from→to` — `+1` for hop-count metrics like eccentricity; for sums
///   that must grow per member (distances, subtree sizes), carry a count
///   inside `T` and add it here (`(s + c, c)`), since every member of the
///   subtree recedes by one hop.
///
/// `vertex.len()` must equal `adj.len()`; mismatched lengths return an
/// empty `Vec` rather than folding a partial answer.
pub fn reroot<T: Copy>(
    adj: &[Vec<u32>],
    vertex: &[T],
    unit: T,
    merge: impl Fn(T, T) -> T,
    lift: impl Fn(T, u32, u32) -> T,
) -> Vec<T> {
    let n = adj.len();
    if vertex.len() != n || n == 0 {
        return Vec::new();
    }

    // Iterative DFS from 0: parent links + a post-order visit list. A tree
    // has no cycles, so the parent link alone is the visited-set.
    let mut parent = vec![u32::MAX; n];
    let mut order: Vec<u32> = Vec::with_capacity(n);
    let mut stack = vec![0u32];
    while let Some(v) = stack.pop() {
        order.push(v);
        for &c in &adj[v as usize] {
            if c != parent[v as usize]
                && (c as usize) < n
                && c != 0
                && parent[c as usize] == u32::MAX
            {
                parent[c as usize] = v;
                stack.push(c);
            }
        }
    }
    // `order` is pre-order; the reverse is a valid post-order for a tree
    // rooted at 0 (children appear before... no: children come after their
    // parent in pre-order, so reversed pre-order is parent-after — correct).

    // Pass 1 — down[v]: merge(vertex[v], lift(down[c], c, v) for children c).
    let mut down = vec![unit; n];
    for &v in order.iter().rev() {
        let (vi, p) = (v as usize, parent[v as usize]);
        let mut acc = vertex[vi];
        for &c in &adj[vi] {
            if c != p && parent[c as usize] == v {
                acc = merge(acc, lift(down[c as usize], c, v));
            }
        }
        down[vi] = acc;
    }

    // Pass 2 — up[v]: the "rest of the tree" aggregate rooted at v (what
    // arrives at v from its parent side, already lifted). For child c of v:
    //   up[c] = lift( merge(vertex[v], up[v], Σ_{sib≠c} lift(down[sib],sib,v)), v, c )
    // where up[root] = unit. Prefix/suffix sibling merges make it O(degree)
    // per child instead of O(degree²) per node.
    let mut up = vec![unit; n];
    up[0] = unit;
    let mut ans = vec![unit; n];
    ans[0] = down[0];
    for &v in &order {
        let vi = v as usize;
        // Children of v in traversal order.
        let kids: Vec<u32> = adj[vi]
            .iter()
            .copied()
            .filter(|&c| parent[c as usize] == v)
            .collect();
        let k = kids.len();
        // pref[i] = merge(vertex[v], up[v], lift(down[kids[0..i]]))
        // suff[i] = merge(lift(down[kids[i..]])) — split around each kid.
        let mut pref = vec![unit; k + 1];
        let mut base = merge(vertex[vi], up[vi]);
        pref[0] = base;
        for i in 0..k {
            base = merge(base, lift(down[kids[i] as usize], kids[i], v));
            pref[i + 1] = base;
        }
        let mut suff = vec![unit; k + 1];
        for i in (0..k).rev() {
            suff[i] = merge(suff[i + 1], lift(down[kids[i] as usize], kids[i], v));
        }
        // ans[v] folds vertex[v] + up[v] + all child lifts = pref[k] (already
        // includes vertex[v] and up[v]). For the root up[v]=unit — same form.
        ans[vi] = pref[k];
        for i in 0..k {
            // Everything at v *except* kid i's contribution:
            //   merge(pref[i], suff[i+1]) — pref[i] already holds vertex+up.
            let rest = merge(pref[i], suff[i + 1]);
            let c = kids[i] as usize;
            up[c] = lift(rest, v, kids[i]);
        }
    }
    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sum-of-distances oracle: BFS from `src`, sum every hop count.
    fn sum_dist(adj: &[Vec<u32>], src: u32) -> i64 {
        let n = adj.len();
        let mut dist = vec![-1i64; n];
        let mut q = std::collections::VecDeque::new();
        dist[src as usize] = 0;
        q.push_back(src);
        let mut total = 0;
        while let Some(v) = q.pop_front() {
            total += dist[v as usize];
            for &c in &adj[v as usize] {
                if dist[c as usize] < 0 {
                    dist[c as usize] = dist[v as usize] + 1;
                    q.push_back(c);
                }
            }
        }
        total
    }

    fn ecc(adj: &[Vec<u32>], src: u32) -> i64 {
        let n = adj.len();
        let mut dist = vec![-1i64; n];
        let mut q = std::collections::VecDeque::new();
        dist[src as usize] = 0;
        q.push_back(src);
        let mut best = 0;
        while let Some(v) = q.pop_front() {
            best = best.max(dist[v as usize]);
            for &c in &adj[v as usize] {
                if dist[c as usize] < 0 {
                    dist[c as usize] = dist[v as usize] + 1;
                    q.push_back(c);
                }
            }
        }
        best
    }

    fn random_tree(n: usize, seed: u64) -> Vec<Vec<u32>> {
        // Deterministic random tree: each i>0 attaches to a uniform earlier
        // vertex — a Prüfer-free generator good enough for an oracle harness.
        let mut rng = crate::rng::SplitMix64::new(seed);
        let mut adj = vec![Vec::new(); n];
        for i in 1..n {
            let p = rng.below(i as u32);
            adj[i].push(p);
            adj[p as usize].push(i as u32);
        }
        adj
    }

    #[test]
    fn sum_of_distances_matches_bfs_on_every_root() {
        // (sum, count) monoid: lift adds the subtree size, not one.
        for n in [1usize, 2, 5, 17, 40] {
            let adj = random_tree(n, 0xB00 + n as u64);
            let vertex = vec![(0i64, 1i64); n];
            let ans = reroot(
                &adj,
                &vertex,
                (0i64, 0i64),
                |(s1, c1), (s2, c2)| (s1 + s2, c1 + c2),
                |(s, c), _, _| (s + c, c),
            );
            let want: Vec<i64> = (0..n as u32).map(|v| sum_dist(&adj, v)).collect();
            let got: Vec<i64> = ans.iter().map(|&(s, _)| s).collect();
            assert_eq!(got, want, "n={n} adj={adj:?}");
        }
    }

    #[test]
    fn eccentricity_matches_bfs_max() {
        let adj = random_tree(30, 0xE00);
        let ans = reroot(
            &adj,
            &[0i64; 30],
            0i64,
            |a: i64, b: i64| a.max(b),
            |a, _, _| a + 1,
        );
        let want: Vec<i64> = (0..30u32).map(|v| ecc(&adj, v)).collect();
        assert_eq!(ans, want);
    }

    #[test]
    fn vertex_weights_sum_to_the_same_total_everywhere() {
        // With identity lift the whole-tree weight is root-independent.
        let adj = random_tree(25, 0xF00);
        let w: Vec<i64> = (0..25).map(|i| i as i64 * 7 + 1).collect();
        let ans = reroot(&adj, &w, 0i64, |a, b| a + b, |a, _, _| a);
        let total: i64 = w.iter().sum();
        assert!(ans.iter().all(|&x| x == total), "{ans:?}");
    }

    #[test]
    fn path_and_star_edge_cases() {
        // Path of 4: eccentricities are 3,2,2,3.
        let path = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        let ans = reroot(
            &path,
            &[0i64; 4],
            0i64,
            |a: i64, b: i64| a.max(b),
            |a, _, _| a + 1,
        );
        assert_eq!(ans, vec![3, 2, 2, 3]);
        // Star: center ecc 1, leaves ecc 2.
        let star = vec![vec![1, 2, 3], vec![0], vec![0], vec![0]];
        let ans = reroot(
            &star,
            &[0i64; 4],
            0i64,
            |a: i64, b: i64| a.max(b),
            |a, _, _| a + 1,
        );
        assert_eq!(ans, vec![1, 2, 2, 2]);
        // Single vertex folds just its weight.
        let solo = vec![vec![]];
        let ans = reroot(&solo, &[42i64], 0i64, |a, b| a + b, |a, _, _| a + 1);
        assert_eq!(ans, vec![42]);
    }

    #[test]
    fn malformed_inputs_stay_total() {
        assert!(reroot::<i64>(&[], &[], 0, |a, b| a + b, |a, _, _| a).is_empty());
        assert!(reroot::<i64>(&[vec![1]], &[1, 2], 0, |a, b| a + b, |a, _, _| a).is_empty());
    }

    #[test]
    fn result_is_deterministic_and_traversal_independent() {
        // Same tree, adjacency lists shuffled → identical answers (merge is
        // commutative, so bracketing order cannot leak into the result).
        let adj = random_tree(24, 0xA11);
        let mut shuffled = adj.clone();
        let mut rng = crate::rng::SplitMix64::new(9);
        for list in &mut shuffled {
            rng.shuffle(list);
        }
        let f = |a: &[Vec<u32>]| {
            reroot(
                a,
                &[0i64; 24],
                0i64,
                |x: i64, y: i64| x + y,
                |x, _, _| x + 1,
            )
        };
        assert_eq!(f(&adj), f(&shuffled));
        assert_eq!(f(&adj), f(&adj));
    }
}
