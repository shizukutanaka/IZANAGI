//! Karp's minimum mean cycle — the smallest average edge weight over
//! all directed cycles of a weighted digraph, exact and deterministic.
//!
//! `dp[k][v]` = minimum weight of a length-`k` directed walk ending
//! at `v`. Karp's theorem: the minimum mean weight is
//!
//! ```text
//! μ = min over v of  max over 0 ≤ k < n of  (dp[n][v] − dp[k][v]) / (n − k)
//! ```
//!
//! where `n` is the vertex count — walks, not simple paths, so the
//! table is a plain `O(V·E)` dynamic program (the theorem that the
//! optimum is always a simple cycle is *why* the mean is the answer).
//! The cycle itself is recovered by backtracking the predecessor
//! chain of the minimizing vertex for `n − k*` steps: the last
//! `n − k*` edges of that walk are provably a cycle of weight
//! `dp[n][v] − dp[k*][v]`.
//!
//! ```
//! use izanagi_kit::frac::Frac;
//! use izanagi_kit::karp::min_mean_cycle;
//! // 0→1→2→0 has weights 1,1,1 (mean 1); 0→3→0 has 4,0 (mean 2).
//! let (mean, cyc) = min_mean_cycle(
//!     4,
//!     &[(0, 1, 1), (1, 2, 1), (2, 0, 1), (0, 3, 4), (3, 0, 0)],
//! )
//! .unwrap();
//! assert_eq!(mean, Frac::from_int(1));
//! assert_eq!(cyc.len(), 3);
//! assert!(min_mean_cycle(2, &[(0, 1, 5)]).is_none()); // acyclic
//! ```
use crate::frac::Frac;

const INF: i128 = i64::MAX as i128 * 4;

/// Minimum mean cycle of a directed graph — `None` when the graph is
/// acyclic or empty, `Some((μ, cycle))` otherwise.
///
/// `edges` are `(from, to, weight)`; weights may be negative. The
/// mean is an exact `Frac`; `cycle` lists each vertex once (the last
/// edge returns to the first). Parallel edges and self-loops are
/// legal — `(v, v, w)` is a length-1 cycle of mean `w`. Output is
/// deterministic: ties resolve for the smallest ratio at each vertex
/// then the smallest vertex.
pub fn min_mean_cycle(n: usize, edges: &[(usize, usize, i64)]) -> Option<(Frac, Vec<usize>)> {
    if n == 0 {
        return None;
    }
    let mut dp = vec![vec![INF; n]; n + 1];
    let mut pred = vec![vec![u32::MAX as usize; n]; n + 1];
    dp[0].fill(0);
    for k in 1..=n {
        for &(u, v, w) in edges {
            if u < n && v < n && dp[k - 1][u] != INF {
                let cand = dp[k - 1][u] + i128::from(w);
                if cand < dp[k][v] {
                    dp[k][v] = cand;
                    pred[k][v] = u;
                }
            }
        }
    }
    let mut best: Option<Frac> = None;
    let mut bv = 0usize;
    for (v, &dn) in dp[n].iter().enumerate() {
        if dn == INF {
            continue;
        }
        let mut vm: Option<Frac> = None;
        for (k, row) in dp.iter().enumerate().take(n) {
            if row[v] == INF {
                continue;
            }
            let r = Frac::new(dn - row[v], (n - k) as i128);
            if vm.map_or(true, |m| r.cmp_frac(&m) == std::cmp::Ordering::Greater) {
                vm = Some(r);
            }
        }
        if let Some(m) = vm {
            if best.map_or(true, |b| m.cmp_frac(&b) == std::cmp::Ordering::Less) {
                best = Some(m);
                bv = v;
            }
        }
    }
    let mu = best?;
    // Backtrack all n edges from bv: n+1 vertices on n nodes force a
    // repeat, and the segment between the first repeated vertex is a
    // simple cycle of mean exactly μ — if it cost more, splicing it
    // out would leave a shorter walk to bv containing a cycle below μ.
    let mut walk = Vec::with_capacity(n + 1);
    let mut cur = bv;
    for k in (1..=n).rev() {
        walk.push(cur);
        cur = pred[k][cur];
    }
    walk.push(cur);
    walk.reverse();
    let mut seen = std::collections::BTreeMap::new();
    let mut cyc = walk;
    for (i, &v) in cyc.iter().enumerate() {
        if let Some(&j) = seen.get(&v) {
            cyc = cyc[j..i].to_vec();
            break;
        }
        seen.insert(v, i);
    }
    Some((mu, cyc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute(n: usize, edges: &[(usize, usize, i64)]) -> Option<Frac> {
        // Enumerate every simple cycle: DFS from each start vertex,
        // closing when the path returns to it.
        let mut best: Option<Frac> = None;
        for s in 0..n {
            let mut stack = vec![(s, vec![s], 0i64)];
            while let Some((v, path, w)) = stack.pop() {
                for &(a, b, e) in edges {
                    if a != v {
                        continue;
                    }
                    if b == s {
                        let f = Frac::new(i128::from(w + e), path.len() as i128);
                        if best.map_or(true, |x| f.cmp_frac(&x) == std::cmp::Ordering::Less) {
                            best = Some(f);
                        }
                    } else if !path.contains(&b) {
                        let mut np = path.clone();
                        np.push(b);
                        stack.push((b, np, w + e));
                    }
                }
            }
        }
        best
    }

    #[test]
    fn textbook() {
        let (m, c) = min_mean_cycle(3, &[(0, 1, 2), (1, 2, 2), (2, 0, 2), (0, 2, 9)]).unwrap();
        assert_eq!(m, Frac::from_int(2));
        assert_eq!(c.len(), 3);
        let (m, c) = min_mean_cycle(2, &[(0, 1, 3), (1, 1, -4)]).unwrap();
        assert_eq!(m, Frac::from_int(-4));
        assert_eq!(c, vec![1]);
        assert!(min_mean_cycle(0, &[]).is_none());
        assert!(min_mean_cycle(3, &[]).is_none());
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x6a7e_0001_beef);
        for _ in 0..300 {
            let n = 1 + rng.below(6) as usize;
            let m = rng.below(16) as usize;
            let edges: Vec<(usize, usize, i64)> = (0..m)
                .map(|_| {
                    (
                        rng.below(n as u32) as usize,
                        rng.below(n as u32) as usize,
                        rng.below(21) as i64 - 10,
                    )
                })
                .collect();
            let want = brute(n, &edges);
            match (want, min_mean_cycle(n, &edges)) {
                (None, None) => {}
                (Some(mu), Some((mg, cyc))) => {
                    assert_eq!(mu, mg);
                    // Returned list is a real cycle of mean μ.
                    assert!(!cyc.is_empty());
                    let mut w = 0i64;
                    for i in 0..cyc.len() {
                        let (a, b) = (cyc[i], cyc[(i + 1) % cyc.len()]);
                        let e = edges
                            .iter()
                            .filter(|&&(u, v, _)| u == a && v == b)
                            .map(|&(_, _, e)| e)
                            .min()
                            .expect("cycle edge exists");
                        w += e;
                    }
                    assert_eq!(Frac::new(i128::from(w), cyc.len() as i128), mg);
                }
                _ => panic!("cyclicity disagreement"),
            }
        }
    }
}
