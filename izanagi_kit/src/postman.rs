//! Chinese postman problem — the minimum-cost closed walk
//! (or open trail) covering every edge of a weighted
//! undirected graph at least once.
//!
//! The augmentation problem is identical in shape to
//! `christofides`: the odd-degree vertices of the graph
//! must be paired up, but here the pairing edges are
//! *shortest paths* in the graph (not a given metric):
//!
//! 1. Odd-degree vertices `T` (`|T|` is even by the
//!    handshake lemma; `|T| = 0` is already Eulerian).
//! 2. `bellman::shortest` from each `t ∈ T` gives the
//!    metric closure `d(t_i, t_j)` plus a reconstructable
//!    path.
//! 3. Min-weight perfect matching on T via subset DP
//!    `O(2^|T| · |T|)` when `|T| ≤ 20`, sorted greedy
//!    beyond (still a perfect matching — only the bound
//!    is lost).
//! 4. Duplicate every edge on each matched path → a
//!    multigraph where every vertex is even →
//!    `euler::euler_walk` circuit. Cost = `Σw + match`.
//!
//! `open_postman` leaves two vertices unmatched — the
//! trail's endpoints. The DP generalizes: `solve(mask,
//! free)` leaves exactly `free` vertices unpaired, so the
//! open version minimizes over the `C(|T|, 2)` endpoint
//! choices in one pass.
//!
//! Determinism: canonical `(cost, u, v)` tie-breaks
//! everywhere; `open_postman` additionally fixes the
//! endpoint pair to the lexicographically smallest
//! achieving the optimum.
//!
//! ```
//! use izanagi_kit::postman::closed_postman;
//! // a square of unit edges — already Eulerian, cost = 4
//! let e = [(0u32, 1u32, 1i64), (1, 2, 1), (2, 3, 1), (3, 0, 1)];
//! let r = closed_postman(4, &e).unwrap();
//! assert_eq!(r.total_cost, 4);
//! // add a dangling edge 0-4: now {0,4} are odd, and the
//! // optimal augmentation duplicates the edge 0-4 once —
//! // sum of edges 5 + 1 = 6
//! let e2 = [
//!     (0u32, 1u32, 1i64), (1, 2, 1), (2, 3, 1), (3, 0, 1), (0, 4, 1),
//! ];
//! let r2 = closed_postman(5, &e2).unwrap();
//! assert_eq!(r2.total_cost, 6);
//! ```

use crate::bellman::shortest;
use crate::euler::euler_walk;
use std::collections::BTreeMap;

/// Bitmask-DP ceiling for the matching step.
const DP_LIMIT: usize = 20;

/// A postman route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Vertex walk covering every input edge at least once.
    pub walk: Vec<u32>,
    /// Extra distance travelled beyond the sum of edge
    /// weights — the matched augmentation.
    pub extra_cost: i64,
    /// `Σ edge weights + extra_cost`.
    pub total_cost: i64,
}

/// `(pairs, leftover-unmatched, cost)`
type MatchOut = (Vec<(usize, usize)>, Vec<usize>, i64);

/// Min-weight matching on `t` leaving exactly `free`
/// vertices unpaired (`free = 0` → perfect matching,
/// `free = 2` → open-route endpoints). `dist` is the metric
/// closure between the original ids. Returns `(pairs,
/// free_leftover, cost)`.
fn matching_free(t: &[usize], free: usize, dist: &[Vec<i64>]) -> Option<MatchOut> {
    let k = t.len();
    // "infinity" — large enough to dominate every real
    // augmentation yet small enough that adding a distance
    // can never overflow i128
    const BIG: i128 = i128::MAX / 4;
    // memo[(mask, free)] = min cost
    let mut memo: BTreeMap<(u32, u8), i128> = BTreeMap::new();
    memo.insert((0, 0), 0);
    fn solve(mask: u32, free: u8, dist: &[Vec<i64>], memo: &mut BTreeMap<(u32, u8), i128>) -> i128 {
        if mask == 0 {
            let c = if free == 0 { 0 } else { BIG };
            memo.insert((mask, free), c);
            return c;
        }
        if let Some(&c) = memo.get(&(mask, free)) {
            return c;
        }
        let v = mask.trailing_zeros() as usize;
        let rest = mask & !(1 << v);
        let mut best = BIG;
        // leave v unmatched
        if free > 0 {
            best = solve(rest, free - 1, dist, memo);
        }
        // pair v with some u
        let mut r = rest;
        while r != 0 {
            let u = r.trailing_zeros() as usize;
            r &= !(1 << u);
            let sub = solve(rest & !(1 << u), free, dist, memo);
            if sub < BIG {
                let c = i128::from(dist[v][u]) + sub;
                if c < best {
                    best = c;
                }
            }
        }
        let best = best.min(BIG);
        memo.insert((mask, free), best);
        best
    }
    if (k + free) % 2 != 0 {
        return None; // parity: even count needed to pair the rest
    }
    let full = (1u32 << k) - 1;
    let cost = solve(full, free as u8, dist, &mut memo);
    if cost >= BIG {
        return None;
    }
    // replay
    let mut pairs = Vec::with_capacity(k / 2);
    let mut left = Vec::new();
    let mut mask = full;
    let mut f = free as u8;
    while mask != 0 {
        let want = memo[&(mask, f)];
        let v = mask.trailing_zeros() as usize;
        let rest = mask & !(1 << v);
        // was v left unmatched?
        if f > 0 && memo.get(&(rest, f - 1)).copied().unwrap_or(BIG) == want {
            left.push(t[v]);
            mask = rest;
            f -= 1;
            continue;
        }
        let mut r = rest;
        let mut advanced = false;
        while r != 0 {
            let u = r.trailing_zeros() as usize;
            r &= !(1 << u);
            if i128::from(dist[v][u]) + memo[&(rest & !(1 << u), f)] == want {
                pairs.push((t[v].min(t[u]), t[v].max(t[u])));
                mask = rest & !(1 << u);
                advanced = true;
                break;
            }
        }
        if !advanced {
            return None;
        }
    }
    pairs.sort_unstable();
    left.sort_unstable();
    Some((pairs, left, i64::try_from(cost).unwrap_or(0)))
}

fn odd_vertices(n: usize, deg: &[u32]) -> Vec<usize> {
    (0..n).filter(|&v| deg[v] % 2 == 1).collect()
}

/// Shared driver: `free` leftover unmatched vertices.
fn drive(n: usize, edges: &[(u32, u32, i64)], free: usize) -> Option<Route> {
    // validate + undirected edge list for bellman
    let mut dir: Vec<(usize, usize, i64)> = Vec::with_capacity(edges.len() * 2);
    let mut deg = vec![0u32; n];
    let mut sum = 0i64;
    for &(a, b, w) in edges {
        if a as usize >= n || b as usize >= n || w < 0 || a == b {
            return None;
        }
        dir.push((a as usize, b as usize, w));
        dir.push((b as usize, a as usize, w));
        deg[a as usize] += 1;
        deg[b as usize] += 1;
        sum = sum.checked_add(w)?;
    }
    if edges.is_empty() {
        return (free == 0).then(|| Route {
            walk: vec![0],
            extra_cost: 0,
            total_cost: 0,
        });
    }
    // connectivity on edge-bearing vertices
    {
        let mut uf: Vec<usize> = (0..n).collect();
        fn find(uf: &mut [usize], x: usize) -> usize {
            let mut r = x;
            while uf[r] != r {
                r = uf[r];
            }
            let mut c = x;
            while uf[c] != r {
                let nx = uf[c];
                uf[c] = r;
                c = nx;
            }
            r
        }
        for &(a, b, _) in edges {
            let (ra, rb) = (find(&mut uf, a as usize), find(&mut uf, b as usize));
            if ra != rb {
                uf[ra] = rb;
            }
        }
        let root = find(&mut uf, edges[0].0 as usize);
        for &(a, b, _) in edges {
            if find(&mut uf, a as usize) != root || find(&mut uf, b as usize) != root {
                return None;
            }
        }
    }
    let odd = odd_vertices(n, &deg);
    if odd.is_empty() {
        // already Eulerian — the same answer either way
        let mut e: Vec<(u32, u32)> = edges.iter().map(|&(a, b, _)| (a, b)).collect();
        e.sort_unstable();
        let (_, walk) = euler_walk(&e)?;
        return Some(Route {
            walk,
            extra_cost: 0,
            total_cost: sum,
        });
    }
    if free > odd.len() || (odd.len() + free) % 2 != 0 {
        return None;
    }
    // metric closure on T
    let mut dist = vec![vec![i64::MAX; odd.len()]; odd.len()];
    for (i, &s) in odd.iter().enumerate() {
        let sp = shortest(n, &dir, s)?;
        for (j, &t) in odd.iter().enumerate() {
            dist[i][j] = sp.dist[t]?;
        }
    }
    let (pairs, _left, extra) = if odd.len() <= DP_LIMIT {
        matching_free(&odd, free, &dist)?
    } else {
        // sorted-greedy pairing for large T (still a valid
        // augmentation; bound lost)
        let mut all: Vec<(i64, usize, usize)> = Vec::new();
        for (i, row) in dist.iter().enumerate() {
            for (j, &w) in row.iter().enumerate().skip(i + 1) {
                all.push((w, i, j));
            }
        }
        all.sort_unstable();
        let mut used = vec![false; odd.len()];
        let mut leftn = 0usize;
        let mut pairs = Vec::new();
        let mut extra = 0i64;
        for &(w, i, j) in &all {
            if used[i] || used[j] {
                continue;
            }
            if odd.len() - leftn <= free {
                break;
            }
            used[i] = true;
            used[j] = true;
            leftn += 2;
            pairs.push((odd[i].min(odd[j]), odd[i].max(odd[j])));
            extra = extra.checked_add(w)?;
        }
        (pairs, Vec::new(), extra)
    };
    // multigraph = originals + each matched path's edges
    let mut multi: Vec<(u32, u32)> = edges.iter().map(|&(a, b, _)| (a, b)).collect();
    for &(u, v) in &pairs {
        // reconstruct shortest path u→v with bellman
        let sp = shortest(n, &dir, u)?;
        let mut cur = v;
        while cur != u {
            let p = sp.prev[cur]?;
            multi.push((p as u32, cur as u32));
            cur = p;
        }
    }
    multi.sort_unstable();
    let (_, walk) = euler_walk(&multi)?;
    Some(Route {
        walk,
        extra_cost: extra,
        total_cost: sum.checked_add(extra)?,
    })
}

/// Closed Chinese postman route (circuit).
///
/// `None` on self-loops, negative weights, or a
/// disconnected edge set.
pub fn closed_postman(n: usize, edges: &[(u32, u32, i64)]) -> Option<Route> {
    drive(n, edges, 0)
}

/// Open postman route — an Eulerian *trail* (two odd
/// endpoints remain; they are the walk's endpoints).
pub fn open_postman(n: usize, edges: &[(u32, u32, i64)]) -> Option<Route> {
    drive(n, edges, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::{BTreeMap, BTreeSet};

    /// Verify the walk covers each undirected edge at least
    /// its input multiplicity.
    fn covers(walk: &[u32], edges: &[(u32, u32, i64)]) -> bool {
        let mut have: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for w in 0..walk.len().saturating_sub(1) {
            let (a, b) = (walk[w].min(walk[w + 1]), walk[w].max(walk[w + 1]));
            *have.entry((a, b)).or_insert(0) += 1;
        }
        let mut need: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for &(a, b, _) in edges {
            let e = (a.min(b), a.max(b));
            *need.entry(e).or_insert(0) += 1;
        }
        need.iter()
            .all(|(e, &c)| have.get(e).copied().unwrap_or(0) >= c)
    }

    fn cost_of(walk: &[u32], edges: &[(u32, u32, i64)]) -> i64 {
        let w: BTreeMap<(u32, u32), i64> = edges
            .iter()
            .map(|&(a, b, w)| ((a.min(b), a.max(b)), w))
            .collect();
        walk.windows(2)
            .map(|p| w[&(p[0].min(p[1]), p[0].max(p[1]))])
            .sum()
    }

    /// Doc example: square → Eulerian (cost 4); add a
    /// pendant edge → augment by d(2,4)=3 → total 8.
    #[test]
    fn basics() {
        let e = [(0u32, 1u32, 1i64), (1, 2, 1), (2, 3, 1), (3, 0, 1)];
        let r = closed_postman(4, &e).unwrap();
        assert_eq!(r.total_cost, 4);
        assert_eq!(r.extra_cost, 0);
        assert_eq!(r.walk.len(), 5);
        let e2 = [
            (0u32, 1u32, 1i64),
            (1, 2, 1),
            (2, 3, 1),
            (3, 0, 1),
            (0, 4, 1),
        ];
        let r2 = closed_postman(5, &e2).unwrap();
        assert_eq!(r2.total_cost, 6);
        assert_eq!(r2.extra_cost, 1);
        assert!(covers(&r2.walk, &e2));
        assert_eq!(r2.walk.first(), r2.walk.last());
        // invalid: self-loop, negative, disconnected
        assert_eq!(closed_postman(3, &[(0, 0, 1)]), None);
        assert_eq!(closed_postman(3, &[(0, 1, -1)]), None);
        assert_eq!(
            closed_postman(4, &[(0, 1, 1), (2, 3, 1)]),
            None,
            "disconnected"
        );
    }

    /// Walk-cost oracle: the returned walk's traversal cost
    /// must equal `total_cost` — i.e. the augmentation
    /// really is the reported matching.
    #[test]
    fn oracle_walk_cost_and_coverage() {
        let mut rng = SplitMix64::new(0xC10A);
        for _ in 0..60 {
            let n = 2 + rng.below(8) as usize;
            let mut es: BTreeSet<(u32, u32)> = BTreeSet::new();
            // cap: more edges than n(n-1)/2 can never be
            // added — an uncapped while loop would never
            // terminate
            let m = (n - 1 + rng.below(6) as usize).min(n * (n - 1) / 2);
            // connect first, then extras
            for v in 1..n {
                es.insert((rng.below(v as u32).min(v as u32 - 1), v as u32));
            }
            while es.len() < m {
                let (a, b) = (rng.below(n as u32), rng.below(n as u32));
                if a != b {
                    es.insert((a.min(b), a.max(b)));
                }
            }
            let edges: Vec<(u32, u32, i64)> = es
                .iter()
                .map(|&(a, b)| (a, b, 1 + rng.below(9) as i64))
                .collect();
            let r = closed_postman(n, &edges).unwrap();
            assert!(covers(&r.walk, &edges));
            assert_eq!(cost_of(&r.walk, &edges), r.total_cost);
            assert_eq!(r.walk.first(), r.walk.last());
        }
    }

    /// Brute-force optimality oracle: on small T the
    /// augmentation must equal the best pairing among ALL
    /// perfect matchings — verified by enumerating them.
    #[test]
    fn oracle_bruteforce_matching() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..40 {
            let n = 3 + rng.below(5) as usize;
            let mut edges = Vec::new();
            // random spanning structure guaranteeing odd count
            for v in 1..n {
                edges.push((rng.below(v as u32), v as u32, 1 + rng.below(9) as i64));
            }
            for _ in 0..rng.below(4) {
                let (a, b) = (rng.below(n as u32), rng.below(n as u32));
                if a != b {
                    edges.push((a.min(b), a.max(b), 1 + rng.below(9) as i64));
                }
            }
            let r = closed_postman(n, &edges).unwrap();
            // recompute optimum independently: all-pairs dist
            let odd = {
                let mut d = vec![0u32; n];
                for &(a, b, _) in &edges {
                    d[a as usize] += 1;
                    d[b as usize] += 1;
                }
                (0..n).filter(|&v| d[v] % 2 == 1).collect::<Vec<usize>>()
            };
            let dir: Vec<(usize, usize, i64)> = edges
                .iter()
                .flat_map(|&(a, b, w)| [(a as usize, b as usize, w), (b as usize, a as usize, w)])
                .collect();
            // enumerate all perfect matchings of `odd`
            let mut best = i64::MAX;
            let mut pairs = Vec::new();
            fn enum_match(
                avail: &[usize],
                pairs: &mut Vec<(usize, usize)>,
                dist: &dyn Fn(usize, usize) -> i64,
                cur: i64,
                best: &mut i64,
            ) {
                if avail.is_empty() {
                    *best = cur.min(*best);
                    return;
                }
                let v = avail[0];
                for i in 1..avail.len() {
                    let u = avail[i];
                    let rest: Vec<usize> = avail
                        .iter()
                        .copied()
                        .filter(|&x| x != v && x != u)
                        .collect();
                    pairs.push((v, u));
                    enum_match(&rest, pairs, dist, cur + dist(v, u), best);
                    pairs.pop();
                }
            }
            let dist = |a: usize, b: usize| -> i64 {
                let sp = shortest(n, &dir, a).unwrap();
                sp.dist[b].unwrap_or(i64::MAX)
            };
            enum_match(&odd, &mut pairs, &dist, 0, &mut best);
            assert_eq!(r.extra_cost, best, "{edges:?}");
        }
    }

    /// Open route: the trail's endpoints are the two
    /// remaining odd vertices, and it never costs more than
    /// the closed route.
    #[test]
    fn open_route_endpoints() {
        // path graph 0-1-2-3-4: endpoints already odd, no
        // augmentation needed for open
        let e = [(0u32, 1u32, 2i64), (1, 2, 2), (2, 3, 2), (3, 4, 2)];
        let r = open_postman(5, &e).unwrap();
        assert_eq!(r.extra_cost, 0);
        assert_eq!(r.total_cost, 8);
        assert_ne!(r.walk.first(), r.walk.last());
        assert!(covers(&r.walk, &e));
        // closed on the same graph duplicates the whole path
        let c = closed_postman(5, &e).unwrap();
        assert_eq!(c.extra_cost, 8);
        // open ≤ closed generally
        assert!(r.total_cost <= c.total_cost);
    }

    /// Determinism: identical inputs → identical walk.
    #[test]
    fn deterministic() {
        let e = [
            (0u32, 1u32, 3i64),
            (1, 2, 1),
            (2, 3, 2),
            (0, 3, 4),
            (1, 3, 5),
        ];
        assert_eq!(closed_postman(4, &e), closed_postman(4, &e));
        assert_eq!(open_postman(4, &e), open_postman(4, &e));
    }
}
