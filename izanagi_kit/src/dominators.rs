//! Dominator analysis on a rooted flow graph — Cooper–Harvey–Kennedy
//! iterative `idom` computation over reverse-postorder (2001).
//!
//! Node `d` *dominates* `v` when every path from `root` to `v` passes
//! through `d`. `idom[v]` is the *immediate* dominator — the strict
//! dominator closest to `v` — and the `idom` edges form the dominator
//! tree. [`Dominators::dominance_frontier`] gives the Cytron frontier
//! (where `v`'s dominance stops), used by control-flow and behavior-
//! graph audits.
//!
//! Deterministic: RPO is built from the adjacency order given, and the
//! fixpoint always processes nodes in RPO order, so the result is a
//! pure function of `(root, succ)`.
//!
//! ```
//! use izanagi_kit::dominators::Dominators;
//! // 0 -> 1 -> 3, 0 -> 2 -> 3: 3 is dominated by 0 only.
//! let d = Dominators::new(0, &[vec![1, 2], vec![3], vec![3], vec![]]);
//! assert_eq!(d.idom(3), Some(0));
//! assert!(d.dominates(0, 3));
//! assert!(!d.dominates(1, 3));
//! ```

/// Dominator tree + queries for a flow graph over `0..succ.len()`.
#[derive(Clone, Debug)]
pub struct Dominators {
    root: usize,
    /// `idom[v]` — `Some(root)` for the root itself, `None` for
    /// unreachable nodes.
    idom: Vec<Option<u32>>,
    /// `children[v]` in the dominator tree, ascending order.
    children: Vec<Vec<u32>>,
    /// `frontier[v]` — Cytron dominance frontier, ascending order.
    frontier: Vec<Vec<u32>>,
    reachable: Vec<bool>,
}

impl Dominators {
    /// Compute dominators of the flow graph rooted at `root` with
    /// successor lists `succ`. Out-of-range successors and roots are
    /// ignored; an out-of-range `root` yields an all-`None` result.
    pub fn new(root: usize, succ: &[Vec<u32>]) -> Dominators {
        let n = succ.len();
        let empty = Dominators {
            root,
            idom: vec![None; n],
            children: vec![Vec::new(); n],
            frontier: vec![Vec::new(); n],
            reachable: vec![false; n],
        };
        if root >= n {
            return empty;
        }
        // preds[v] = all u with v in succ[u] (in-range only).
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (u, outs) in succ.iter().enumerate() {
            for &w in outs {
                if (w as usize) < n {
                    preds[w as usize].push(u);
                }
            }
        }
        // Iterative postorder DFS → reverse-postorder numbering.
        // rpo[v] = position in RPO (smaller = closer to root);
        // UNREACH marks unreached nodes.
        const UNREACH: u32 = u32::MAX;
        let mut rpo = vec![UNREACH; n];
        let mut order: Vec<usize> = Vec::new();
        {
            let mut seen = vec![false; n];
            seen[root] = true;
            let mut st: Vec<(usize, usize)> = vec![(root, 0)];
            while let Some(&(v, ci)) = st.last() {
                if ci < succ[v].len() {
                    let w = succ[v][ci] as usize;
                    if let Some(e) = st.last_mut() {
                        e.1 += 1;
                    }
                    if w < n && !seen[w] {
                        seen[w] = true;
                        st.push((w, 0));
                    }
                } else {
                    order.push(v);
                    st.pop();
                }
            }
            // order is postorder; reverse → RPO numbering.
            for (i, &v) in order.iter().rev().enumerate() {
                rpo[v] = i as u32;
            }
        }
        let mut reachable = vec![false; n];
        for &v in &order {
            reachable[v] = true;
        }
        // CHK iterative idom.
        let mut idom: Vec<Option<u32>> = vec![None; n];
        idom[root] = Some(root as u32);
        let rpo_order: Vec<usize> = order.iter().rev().copied().collect();
        // Both arguments have an idom chain to the root; advance the
        // deeper finger (larger RPO number) until they meet.
        let intersect = |idom: &[Option<u32>], rpo: &[u32], mut a: usize, mut b: usize| -> usize {
            while a != b {
                while rpo[a] > rpo[b] {
                    match idom[a] {
                        Some(x) => a = x as usize,
                        None => return b,
                    }
                }
                while rpo[b] > rpo[a] {
                    match idom[b] {
                        Some(x) => b = x as usize,
                        None => return a,
                    }
                }
            }
            a
        };
        let mut changed = true;
        while changed {
            changed = false;
            for &b in &rpo_order {
                if b == root {
                    continue;
                }
                // First reachable predecessor with an assigned idom.
                let mut new_idom: Option<usize> = None;
                for &p in &preds[b] {
                    if idom[p].is_some() {
                        new_idom = Some(match new_idom {
                            None => p,
                            Some(cur) => intersect(&idom, &rpo, cur, p),
                        });
                    }
                }
                if new_idom != idom[b].map(|x| x as usize) {
                    idom[b] = new_idom.map(|x| x as u32);
                    changed = true;
                }
            }
        }
        // Dominator tree children (ascending).
        let mut children = vec![Vec::new(); n];
        for (v, im) in idom.iter().enumerate() {
            if let Some(p) = im {
                if v != root {
                    children[*p as usize].push(v as u32);
                }
            }
        }
        for c in children.iter_mut() {
            c.sort_unstable();
        }
        // Cytron dominance frontier: for each join node b (≥2 reachable
        // preds), run each pred up the idom chain until idom[b].
        let mut frontier = vec![Vec::new(); n];
        for (b, ps) in preds.iter().enumerate() {
            if !reachable[b] {
                continue;
            }
            let live: Vec<usize> = ps.iter().copied().filter(|&p| reachable[p]).collect();
            if live.len() < 2 {
                continue;
            }
            for &p in &live {
                let mut runner = p;
                while Some(runner as u32) != idom[b] {
                    frontier[runner].push(b as u32);
                    match idom[runner] {
                        Some(next) => runner = next as usize,
                        None => break,
                    }
                }
            }
        }
        for f in frontier.iter_mut() {
            f.sort_unstable();
            f.dedup();
        }
        Dominators {
            root,
            idom,
            children,
            frontier,
            reachable,
        }
    }

    /// Immediate dominator of `v` — `Some(root)` for the root,
    /// `None` for unreachable or out-of-range nodes.
    pub fn idom(&self, v: usize) -> Option<u32> {
        self.idom.get(v).copied().flatten()
    }

    /// Whether `v` is reachable from the root.
    pub fn reachable(&self, v: usize) -> bool {
        self.reachable.get(v).copied().unwrap_or(false)
    }

    /// Whether `a` dominates `b` (every root→b path passes `a`).
    /// Unreachable `b` is vacuously dominated by nothing except itself.
    pub fn dominates(&self, a: usize, b: usize) -> bool {
        if a >= self.idom.len() || !self.reachable(b) {
            return false;
        }
        if a == b {
            return true;
        }
        let mut cur = self.idom[b];
        while let Some(c) = cur {
            if c as usize == a {
                return true;
            }
            let next = self.idom[c as usize];
            if next == Some(c) {
                break; // reached root
            }
            cur = next;
        }
        false
    }

    /// All dominators of `v`, root→v order (includes `v` itself when
    /// reachable). Empty for unreachable nodes.
    pub fn dominators(&self, v: usize) -> Vec<u32> {
        if !self.reachable(v) {
            return Vec::new();
        }
        let mut chain = vec![v as u32];
        let mut cur = self.idom[v];
        while let Some(c) = cur {
            if c as usize == v {
                break; // root's idom points at itself
            }
            chain.push(c);
            if self.idom[c as usize] == Some(c) {
                break;
            }
            cur = self.idom[c as usize];
        }
        chain.reverse();
        chain
    }

    /// Strict dominators of `v` (all but `v` itself), root→v order.
    pub fn strict_dominators(&self, v: usize) -> Vec<u32> {
        let mut d = self.dominators(v);
        d.pop();
        d
    }

    /// Dominator-tree children of `v` (ascending).
    pub fn children(&self, v: usize) -> &[u32] {
        self.children.get(v).map(|c| c.as_slice()).unwrap_or(&[])
    }

    /// Cytron dominance frontier of `v` — nodes where `v`'s dominance
    /// ends (a successor has a path that avoids strictly-`v`-dominated
    /// control). Ascending order.
    pub fn dominance_frontier(&self, v: usize) -> &[u32] {
        self.frontier.get(v).map(|f| f.as_slice()).unwrap_or(&[])
    }

    /// The root this analysis was computed from.
    pub fn root(&self) -> usize {
        self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::{BTreeSet, VecDeque};

    /// Oracle: dom[v] = {v} ∪ ⋂{dom[p] : p ∈ reachable preds(v)}
    /// iterated to fixpoint on small graphs.
    fn oracle_dom_sets(root: usize, succ: &[Vec<u32>]) -> Vec<BTreeSet<usize>> {
        let n = succ.len();
        let mut reach = vec![false; n];
        let mut q = VecDeque::from([root]);
        reach[root] = true;
        while let Some(u) = q.pop_front() {
            for &w in &succ[u] {
                let w = w as usize;
                if w < n && !reach[w] {
                    reach[w] = true;
                    q.push_back(w);
                }
            }
        }
        let mut dom: Vec<BTreeSet<usize>> = (0..n)
            .map(|v| {
                if reach[v] {
                    (0..n).collect()
                } else {
                    BTreeSet::new()
                }
            })
            .collect();
        dom[root] = BTreeSet::from([root]);
        loop {
            let mut changed = false;
            for v in 0..n {
                if !reach[v] || v == root {
                    continue;
                }
                let mut acc: Option<BTreeSet<usize>> = None;
                for (p, outs) in succ.iter().enumerate() {
                    if !reach[p] || !outs.iter().any(|&w| w as usize == v) {
                        continue;
                    }
                    acc = Some(match acc {
                        None => dom[p].clone(),
                        Some(a) => a.intersection(&dom[p]).copied().collect(),
                    });
                }
                let mut new = acc.unwrap_or_default();
                new.insert(v);
                if new != dom[v] {
                    dom[v] = new;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        dom
    }

    fn random_cfg(rng: &mut SplitMix64, n: usize) -> Vec<Vec<u32>> {
        (0..n)
            .map(|_| {
                (0..n)
                    .filter(|_| rng.below(3) == 0)
                    .map(|w| w as u32)
                    .collect()
            })
            .collect()
    }

    #[test]
    fn dominates_and_idom_match_set_fixpoint_oracle() {
        let mut rng = SplitMix64::new(0xD011);
        for _ in 0..200 {
            let n = (rng.below(9) + 1) as usize;
            let root = rng.below(n as u32) as usize;
            let succ = random_cfg(&mut rng, n);
            let d = Dominators::new(root, &succ);
            let dom = oracle_dom_sets(root, &succ);
            // dominates() for all pairs.
            for a in 0..n {
                for (b, db) in dom.iter().enumerate() {
                    let want = db.contains(&a);
                    assert_eq!(d.dominates(a, b), want, "a={a} b={b} succ={succ:?}");
                }
            }
            // idom[v] = the strict dominator dominated by every other
            // strict dominator (deepest in the chain).
            for (v, dv) in dom.iter().enumerate() {
                let strict: Vec<usize> = dv.iter().copied().filter(|&s| s != v).collect();
                let want = if !dv.contains(&v) {
                    None // unreachable
                } else if v == root {
                    Some(root)
                } else {
                    strict
                        .iter()
                        .copied()
                        .find(|&s| strict.iter().all(|&o| o == s || dom[s].contains(&o)))
                };
                assert_eq!(d.idom(v).map(|x| x as usize), want, "v={v} succ={succ:?}");
            }
            // dominators(v) chain equals dom[v] sorted root→v.
            for (v, dv) in dom.iter().enumerate() {
                let chain = d.dominators(v);
                let set: BTreeSet<u32> = chain.iter().copied().collect();
                let want: BTreeSet<u32> = dv.iter().map(|&x| x as u32).collect();
                assert_eq!(set, want, "v={v}");
                // Chain order: each non-root entry's idom is the previous.
                for w in 1..chain.len() {
                    assert_eq!(d.idom(chain[w] as usize), Some(chain[w - 1]));
                }
            }
        }
    }

    #[test]
    fn tree_and_frontier_invariants() {
        let mut rng = SplitMix64::new(0xF0A7);
        for _ in 0..100 {
            let n = (rng.below(8) + 1) as usize;
            let succ = random_cfg(&mut rng, n);
            let d = Dominators::new(0, &succ);
            // Tree: children(v) = { w : idom[w] == v, w != root }.
            for v in 0..n {
                for &c in d.children(v) {
                    assert_eq!(d.idom(c as usize), Some(v as u32));
                    assert!(d.dominates(v, c as usize));
                }
            }
            // Frontier membership sanity: every f in DF[v] is reachable
            // and has a reachable pred dominated by v.
            for v in 0..n {
                for &f in d.dominance_frontier(v) {
                    let f = f as usize;
                    assert!(d.reachable(f));
                    let has_pred_dom = (0..n)
                        .any(|p| succ[p].iter().any(|&w| w as usize == f) && d.dominates(v, p));
                    assert!(has_pred_dom, "v={v} f={f} succ={succ:?}");
                    assert!(!d.strict_dominators(f).contains(&(v as u32)) || v == f);
                }
            }
        }
    }

    #[test]
    fn known_shapes() {
        // Chain: each node dominated by everything above.
        let d = Dominators::new(0, &[vec![1], vec![2], vec![]]);
        assert_eq!(d.idom(2), Some(1));
        assert_eq!(d.dominators(2), vec![0, 1, 2]);
        assert_eq!(d.strict_dominators(2), vec![0, 1]);
        // Diamond: merge dominated only by branch point.
        let d = Dominators::new(0, &[vec![1, 2], vec![3], vec![3], vec![]]);
        assert_eq!(d.idom(3), Some(0));
        assert_eq!(d.dominance_frontier(1), &[3]);
        assert_eq!(d.dominance_frontier(2), &[3]);
        assert_eq!(d.dominance_frontier(0), &[] as &[u32]);
        // Loop: 0→1→2→1, 1→3.
        let d = Dominators::new(0, &[vec![1], vec![2, 3], vec![1], vec![]]);
        assert_eq!(d.idom(1), Some(0));
        assert_eq!(d.idom(2), Some(1));
        // DF[2] includes 1 (back-edge target's frontier member).
        assert!(d.dominance_frontier(2).contains(&1));
        // Unreachable nodes: no idom, not dominated.
        let d = Dominators::new(0, &[vec![1], vec![], vec![3], vec![]]);
        assert!(!d.reachable(2));
        assert_eq!(d.idom(2), None);
        assert_eq!(d.dominators(2), Vec::<u32>::new());
    }

    #[test]
    fn edge_cases() {
        let d = Dominators::new(0, &[]);
        assert_eq!(d.idom(0), None);
        let d = Dominators::new(9, &[vec![1], vec![]]);
        assert!(!d.reachable(0));
        let d = Dominators::new(0, &[vec![]]);
        assert_eq!(d.idom(0), Some(0));
        assert_eq!(d.dominators(0), vec![0]);
        assert_eq!(d.dominance_frontier(0), &[] as &[u32]);
    }
}
