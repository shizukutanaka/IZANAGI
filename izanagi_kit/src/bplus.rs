//! B+ tree: ordered `u64 → u64` map with sorted leaf-chain scans.
//!
//! All keys live in leaves; internal nodes hold separator keys only.
//! `order` bounds children per internal node (and keys per leaf); the
//! default is 32. Leaves are linked left-to-right so `range` is a
//! linear scan once the first leaf is found.
//!
//! Split semantics: a leaf split *copies* the smallest key of the
//! right half up as the separator (it stays a real key in the leaf);
//! an internal split *moves* the middle key up. Deletes borrow from a
//! same-parent sibling when one has spare keys, else merge.
//!
//! `check()` audits every invariant — separator correctness, key
//! counts, fill bounds, leaf-chain continuity — and the tests run a
//! `BTreeMap` shadow oracle over interleaved insert/delete/range.
//!
//! ```
//! use izanagi_kit::bplus::Bplus;
//! let mut t = Bplus::new(4);
//! for k in 0..10 {
//!     t.insert(k, k * 7);
//! }
//! assert_eq!(t.get(5), Some(35));
//! assert_eq!(t.range(3, 6), vec![(3, 21), (4, 28), (5, 35), (6, 42)]);
//! ```

const NIL: u32 = !0;

struct Node {
    keys: Vec<u64>,
    /// Internal: children (keys.len()+1). Leaf: unused.
    kids: Vec<u32>,
    /// Leaf: values parallel to keys.
    vals: Vec<u64>,
    leaf: bool,
    next: u32,
    parent: u32,
}

impl Node {
    fn leaf() -> Self {
        Node {
            keys: Vec::new(),
            kids: Vec::new(),
            vals: Vec::new(),
            leaf: true,
            next: NIL,
            parent: NIL,
        }
    }
    fn internal() -> Self {
        Node {
            keys: Vec::new(),
            kids: Vec::new(),
            vals: Vec::new(),
            leaf: false,
            next: NIL,
            parent: NIL,
        }
    }
}

/// Ordered map backed by a B+ tree. `order` = max children of an
/// internal node (leaves hold up to `order` keys); 4..=64.
pub struct Bplus {
    order: usize,
    root: u32,
    nodes: Vec<Node>,
    free: Vec<u32>,
    len: usize,
}

impl Bplus {
    /// Empty tree. `order` is clamped to `[4, 64]`.
    pub fn new(order: usize) -> Self {
        let order = order.clamp(4, 64);
        Bplus {
            order,
            root: NIL,
            nodes: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Number of live key-value pairs.
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` when the tree has no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn alloc(&mut self, node: Node) -> u32 {
        if let Some(i) = self.free.pop() {
            self.nodes[i as usize] = node;
            i
        } else {
            self.nodes.push(node);
            (self.nodes.len() - 1) as u32
        }
    }

    fn leaf_for(&self, k: u64) -> u32 {
        let mut n = self.root;
        while !self.nodes[n as usize].leaf {
            let nd = &self.nodes[n as usize];
            // First kid whose separator exceeds k, else the last one.
            let idx = nd
                .keys
                .iter()
                .position(|&sep| k < sep)
                .unwrap_or(nd.kids.len() - 1);
            n = nd.kids[idx];
        }
        n
    }

    /// Look up `k`; `Some(v)` iff present.
    pub fn get(&self, k: u64) -> Option<u64> {
        if self.root == NIL {
            return None;
        }
        let l = self.leaf_for(k);
        let nd = &self.nodes[l as usize];
        nd.keys.binary_search(&k).ok().map(|i| nd.vals[i])
    }

    /// `true` iff `k` is in the tree.
    pub fn contains(&self, k: u64) -> bool {
        self.get(k).is_some()
    }

    /// Insert or overwrite `k → v`.
    ///
    /// ```
    /// use izanagi_kit::bplus::Bplus;
    /// let mut t = Bplus::new(4);
    /// t.insert(2, 9);
    /// t.insert(2, 5);
    /// assert_eq!(t.get(2), Some(5));
    /// ```
    pub fn insert(&mut self, k: u64, v: u64) {
        if self.root == NIL {
            let mut n = Node::leaf();
            n.keys.push(k);
            n.vals.push(v);
            self.root = self.alloc(n);
            self.len = 1;
            return;
        }
        // Path from root to leaf for split unwinding.
        let mut path: Vec<(u32, usize)> = Vec::new();
        let mut n = self.root;
        while !self.nodes[n as usize].leaf {
            let nd = &self.nodes[n as usize];
            let idx = nd
                .keys
                .iter()
                .position(|&sep| k < sep)
                .unwrap_or(nd.kids.len() - 1);
            path.push((n, idx));
            n = nd.kids[idx];
        }
        let leaf = n;
        {
            let nd = &mut self.nodes[leaf as usize];
            match nd.keys.binary_search(&k) {
                Ok(i) => {
                    nd.vals[i] = v;
                    return;
                }
                Err(i) => {
                    nd.keys.insert(i, k);
                    nd.vals.insert(i, v);
                }
            }
        }
        self.len += 1;
        if self.nodes[leaf as usize].keys.len() <= self.order {
            return;
        }
        // Leaf overflow: split, copy right half's first key up.
        let (promote, right) = self.split_leaf(leaf);
        self.insert_into_parent(leaf, promote, right, path);
    }

    fn split_leaf(&mut self, leaf: u32) -> (u64, u32) {
        let order = self.order;
        let mut right = Node::leaf();
        {
            let nd = &mut self.nodes[leaf as usize];
            let split = order.div_ceil(2);
            right.keys = nd.keys.split_off(split);
            right.vals = nd.vals.split_off(split);
            right.next = nd.next;
            right.parent = nd.parent;
        }
        let ri = self.alloc(right);
        self.nodes[leaf as usize].next = ri;
        let promote = self.nodes[ri as usize].keys[0];
        (promote, ri)
    }

    fn split_internal(&mut self, node: u32) -> (u64, u32) {
        let mut right = Node::internal();
        let up;
        {
            let nd = &mut self.nodes[node as usize];
            let mid = nd.keys.len() / 2;
            up = nd.keys[mid];
            right.keys = nd.keys.split_off(mid + 1);
            nd.keys.pop(); // removes `up`
            right.kids = nd.kids.split_off(mid + 1);
            right.parent = nd.parent;
        }
        let ri = self.alloc(right);
        // Fix children's parent pointers.
        let kids: Vec<u32> = self.nodes[ri as usize].kids.clone();
        for c in kids {
            self.nodes[c as usize].parent = ri;
        }
        (up, ri)
    }

    fn insert_into_parent(
        &mut self,
        mut left: u32,
        mut key: u64,
        mut right: u32,
        mut path: Vec<(u32, usize)>,
    ) {
        loop {
            match path.pop() {
                None => {
                    // Split reached the root: grow one level.
                    let mut new_root = Node::internal();
                    new_root.keys.push(key);
                    new_root.kids.push(left);
                    new_root.kids.push(right);
                    let ri = self.alloc(new_root);
                    self.nodes[left as usize].parent = ri;
                    self.nodes[right as usize].parent = ri;
                    self.root = ri;
                    return;
                }
                Some((parent, idx)) => {
                    self.nodes[left as usize].parent = parent;
                    self.nodes[right as usize].parent = parent;
                    {
                        let nd = &mut self.nodes[parent as usize];
                        nd.keys.insert(idx, key);
                        nd.kids.insert(idx + 1, right);
                    }
                    if self.nodes[parent as usize].keys.len() <= self.order {
                        return;
                    }
                    let (up, r2) = self.split_internal(parent);
                    left = parent;
                    key = up;
                    right = r2;
                }
            }
        }
    }

    /// Remove `k`; returns the stored value iff it was present.
    ///
    /// ```
    /// use izanagi_kit::bplus::Bplus;
    /// let mut t = Bplus::new(4);
    /// t.insert(1, 10);
    /// assert_eq!(t.remove(1), Some(10));
    /// assert_eq!(t.remove(1), None);
    /// ```
    pub fn remove(&mut self, k: u64) -> Option<u64> {
        if self.root == NIL {
            return None;
        }
        let leaf = self.leaf_for(k);
        let (i, v) = {
            let nd = &mut self.nodes[leaf as usize];
            let i = nd.keys.binary_search(&k).ok()?;
            nd.keys.remove(i);
            (i, nd.vals.remove(i))
        };
        self.len -= 1;
        if i == 0 && !self.nodes[leaf as usize].keys.is_empty() {
            // The leaf's minimum changed: refresh the separator in the
            // nearest ancestor that owns this leaf as a non-first child.
            self.fix_sep(leaf);
        }
        self.fix_underflow(leaf);
        Some(v)
    }

    /// Update the separator entry referencing `node`'s subtree after
    /// its minimum key changed (walk up until a non-first child slot).
    fn fix_sep(&mut self, node: u32) {
        let new_min = self.nodes[node as usize].keys[0];
        let mut cur = node;
        loop {
            let p = self.nodes[cur as usize].parent;
            if p == NIL {
                return;
            }
            let pos = self.nodes[p as usize]
                .kids
                .iter()
                .position(|&c| c == cur)
                .unwrap_or(0);
            if pos > 0 {
                self.nodes[p as usize].keys[pos - 1] = new_min;
                return;
            }
            cur = p;
        }
    }

    fn min_keys(&self, leaf: bool) -> usize {
        if leaf {
            self.order.div_ceil(2)
        } else {
            self.order.div_ceil(2) - 1
        }
    }

    fn fix_underflow(&mut self, mut node: u32) {
        loop {
            if node == self.root {
                // Root shrink: internal root with one child collapses.
                if !self.nodes[node as usize].leaf && self.nodes[node as usize].kids.len() == 1 {
                    let kid = self.nodes[node as usize].kids[0];
                    self.nodes[kid as usize].parent = NIL;
                    self.root = kid;
                    self.free.push(node);
                } else if self.nodes[node as usize].leaf
                    && self.nodes[node as usize].keys.is_empty()
                {
                    self.free.push(node);
                    self.root = NIL;
                }
                return;
            }
            let is_leaf = self.nodes[node as usize].leaf;
            if self.nodes[node as usize].keys.len() >= self.min_keys(is_leaf) {
                return;
            }
            let parent = self.nodes[node as usize].parent;
            let pidx = self.nodes[parent as usize]
                .kids
                .iter()
                .position(|&c| c == node)
                .unwrap_or(0);
            let p = parent as usize;
            let nkids = self.nodes[p].kids.len();
            let left_sib = if pidx > 0 {
                Some(self.nodes[p].kids[pidx - 1])
            } else {
                None
            };
            let right_sib = if pidx + 1 < nkids {
                Some(self.nodes[p].kids[pidx + 1])
            } else {
                None
            };
            // Borrow from a sibling with spare keys.
            if let Some(ls) = left_sib {
                let spare = self.nodes[ls as usize].keys.len() > self.min_keys(is_leaf);
                if spare {
                    self.borrow_left(node, ls, pidx);
                    return;
                }
            }
            if let Some(rs) = right_sib {
                let spare = self.nodes[rs as usize].keys.len() > self.min_keys(is_leaf);
                if spare {
                    self.borrow_right(node, rs, pidx);
                    return;
                }
            }
            // Merge into a sibling.
            if let Some(ls) = left_sib {
                self.merge_into(ls, node, pidx - 1);
                node = parent;
            } else if let Some(rs) = right_sib {
                self.merge_into(node, rs, pidx);
                node = parent;
            } else {
                return;
            }
        }
    }

    fn borrow_left(&mut self, node: u32, sib: u32, pidx: usize) {
        let parent = self.nodes[node as usize].parent as usize;
        let leaf = self.nodes[node as usize].leaf;
        if leaf {
            let (k, v) = {
                let s = &mut self.nodes[sib as usize];
                (s.keys.pop().unwrap_or(0), s.vals.pop().unwrap_or(0))
            };
            {
                let nd = &mut self.nodes[node as usize];
                nd.keys.insert(0, k);
                nd.vals.insert(0, v);
            }
            self.nodes[parent].keys[pidx - 1] = k;
        } else {
            let (sep, kid) = {
                let s = &mut self.nodes[sib as usize];
                (s.keys.pop().unwrap_or(0), s.kids.pop().unwrap_or(NIL))
            };
            let old_sep = self.nodes[parent].keys[pidx - 1];
            {
                let nd = &mut self.nodes[node as usize];
                nd.keys.insert(0, old_sep);
                nd.kids.insert(0, kid);
            }
            self.nodes[kid as usize].parent = node;
            self.nodes[parent].keys[pidx - 1] = sep;
        }
    }

    fn borrow_right(&mut self, node: u32, sib: u32, pidx: usize) {
        let parent = self.nodes[node as usize].parent as usize;
        let leaf = self.nodes[node as usize].leaf;
        if leaf {
            let (k, v) = {
                let s = &mut self.nodes[sib as usize];
                (s.keys.remove(0), s.vals.remove(0))
            };
            {
                let nd = &mut self.nodes[node as usize];
                nd.keys.push(k);
                nd.vals.push(v);
            }
            self.nodes[parent].keys[pidx] = self.nodes[sib as usize].keys[0];
        } else {
            let (sep, kid) = {
                let s = &mut self.nodes[sib as usize];
                (s.keys.remove(0), s.kids.remove(0))
            };
            let old_sep = self.nodes[parent].keys[pidx];
            {
                let nd = &mut self.nodes[node as usize];
                nd.keys.push(old_sep);
                nd.kids.push(kid);
            }
            self.nodes[kid as usize].parent = node;
            self.nodes[parent].keys[pidx] = sep;
        }
    }

    /// Merge `right` into `left`, deleting separator `sep_idx` from the
    /// parent (its child pointer to `right` also goes).
    fn merge_into(&mut self, left: u32, right: u32, sep_idx: usize) {
        let parent = self.nodes[left as usize].parent as usize;
        let leaf = self.nodes[left as usize].leaf;
        if leaf {
            let (mut rk, mut rv, rnext) = {
                let r = &mut self.nodes[right as usize];
                (
                    std::mem::take(&mut r.keys),
                    std::mem::take(&mut r.vals),
                    r.next,
                )
            };
            {
                let l = &mut self.nodes[left as usize];
                l.keys.append(&mut rk);
                l.vals.append(&mut rv);
                l.next = rnext;
            }
            let p = &mut self.nodes[parent];
            p.keys.remove(sep_idx);
            p.kids.remove(sep_idx + 1);
        } else {
            let sep = self.nodes[parent].keys[sep_idx];
            let (mut rk, mut rkids) = {
                let r = &mut self.nodes[right as usize];
                (std::mem::take(&mut r.keys), std::mem::take(&mut r.kids))
            };
            {
                let l = &mut self.nodes[left as usize];
                l.keys.push(sep);
                l.keys.append(&mut rk);
                l.kids.append(&mut rkids);
            }
            let kids: Vec<u32> = self.nodes[left as usize].kids.clone();
            for c in kids {
                self.nodes[c as usize].parent = left;
            }
            let p = &mut self.nodes[parent];
            p.keys.remove(sep_idx);
            p.kids.remove(sep_idx + 1);
        }
        self.free.push(right);
    }

    /// All pairs with `lo <= k <= hi` in ascending order (leaf-chain
    /// scan, `O(log n + out)`).
    ///
    /// ```
    /// use izanagi_kit::bplus::Bplus;
    /// let mut t = Bplus::new(4);
    /// for k in [5, 3, 8, 1] {
    ///     t.insert(k, k);
    /// }
    /// assert_eq!(t.range(3, 5), vec![(3, 3), (5, 5)]);
    /// ```
    pub fn range(&self, lo: u64, hi: u64) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        if self.root == NIL || lo > hi {
            return out;
        }
        let mut l = self.leaf_for(lo);
        'outer: loop {
            let nd = &self.nodes[l as usize];
            for (i, &k) in nd.keys.iter().enumerate() {
                if k > hi {
                    break 'outer;
                }
                if k >= lo {
                    out.push((k, nd.vals[i]));
                }
            }
            l = nd.next;
            if l == NIL {
                break;
            }
        }
        out
    }

    /// Every key in ascending order (leaf-chain walk).
    pub fn keys(&self) -> Vec<u64> {
        self.range(0, !0).iter().map(|&(k, _)| k).collect()
    }

    /// Full structural audit. Returns `Ok(())` iff every B+ invariant
    /// holds: sorted keys, fill bounds, separator correctness, parent
    /// links, leaf-chain order. Tests call this after every op batch.
    pub fn check(&self) -> Result<(), &'static str> {
        if self.root == NIL {
            return if self.len == 0 {
                Ok(())
            } else {
                Err("len>0 but no root")
            };
        }
        let mut min_leaf = !0usize;
        let mut max_leaf = 0usize;
        self.audit(self.root, None, NIL, 0, &mut min_leaf, &mut max_leaf)?;
        if min_leaf != max_leaf {
            return Err("leaves at different depths");
        }
        // Leaf chain must enumerate all keys in order.
        let mut l = self.root;
        while !self.nodes[l as usize].leaf {
            l = self.nodes[l as usize].kids[0];
        }
        let mut seen = 0usize;
        let mut prev: Option<u64> = None;
        loop {
            let nd = &self.nodes[l as usize];
            for &k in &nd.keys {
                if let Some(p) = prev {
                    if k <= p {
                        return Err("leaf chain not sorted");
                    }
                }
                prev = Some(k);
                seen += 1;
            }
            l = nd.next;
            if l == NIL {
                break;
            }
        }
        if seen != self.len {
            return Err("leaf count != len");
        }
        Ok(())
    }

    fn audit(
        &self,
        n: u32,
        parent: Option<u32>,
        expected_parent: u32,
        depth: usize,
        min_leaf: &mut usize,
        max_leaf: &mut usize,
    ) -> Result<(), &'static str> {
        let nd = &self.nodes[n as usize];
        if nd.parent != expected_parent {
            return Err("bad parent link");
        }
        if parent.is_some() && nd.keys.len() < self.min_keys(nd.leaf) {
            return Err("underfull node");
        }
        for w in nd.keys.windows(2) {
            if w[0] >= w[1] {
                return Err("unsorted keys");
            }
        }
        if nd.leaf {
            if nd.keys.len() != nd.vals.len() {
                return Err("leaf keys/vals mismatch");
            }
            if nd.keys.len() > self.order {
                return Err("leaf overflow");
            }
            *min_leaf = (*min_leaf).min(depth);
            *max_leaf = (*max_leaf).max(depth);
            return Ok(());
        }
        if nd.kids.len() != nd.keys.len() + 1 {
            return Err("internal kids != keys+1");
        }
        if nd.keys.len() > self.order {
            return Err("internal overflow");
        }
        for &c in &nd.kids {
            self.audit(c, Some(n), n, depth + 1, min_leaf, max_leaf)?;
        }
        // Separator = smallest key of the subtree at kids[i+1].
        for (i, &sep) in nd.keys.iter().enumerate() {
            let mut r = nd.kids[i + 1];
            while !self.nodes[r as usize].leaf {
                r = self.nodes[r as usize].kids[0];
            }
            let first = self.nodes[r as usize].keys[0];
            if sep != first {
                return Err("separator != right subtree min");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn basics() {
        let mut t = Bplus::new(4);
        assert!(t.is_empty());
        t.insert(3, 30);
        t.insert(1, 10);
        t.insert(2, 20);
        assert_eq!(t.get(1), Some(10));
        assert_eq!(t.get(4), None);
        assert_eq!(t.keys(), vec![1, 2, 3]);
        assert_eq!(t.remove(2), Some(20));
        assert_eq!(t.keys(), vec![1, 3]);
        assert!(t.check().is_ok());
    }

    #[test]
    fn sequential_insert_splits_and_audits() {
        let mut t = Bplus::new(4);
        for k in 0..200u64 {
            t.insert(k, k * 3);
            t.check()
                .unwrap_or_else(|e| panic!("after insert {k}: {e}"));
        }
        assert_eq!(t.len(), 200);
        assert_eq!(t.keys(), (0..200).collect::<Vec<_>>());
        assert_eq!(
            t.range(50, 60),
            (50..=60).map(|k| (k, k * 3)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn btree_shadow_random_ops() {
        let mut rng = SplitMix64::new(0xB7A5);
        for order in [4usize, 6, 16] {
            let mut t = Bplus::new(order);
            let mut oracle: BTreeMap<u64, u64> = BTreeMap::new();
            for step in 0..4000 {
                let op = rng.below(100);
                let k = rng.below(300) as u64;
                if op < 55 {
                    t.insert(k, k * 2 + 1);
                    oracle.insert(k, k * 2 + 1);
                } else if op < 95 {
                    let got = t.remove(k);
                    assert_eq!(got, oracle.remove(&k), "remove {k} step {step}");
                } else {
                    let lo = rng.below(300) as u64;
                    let hi = (lo + rng.below(40) as u64).min(299);
                    let want: Vec<(u64, u64)> =
                        oracle.range(lo..=hi).map(|(&a, &b)| (a, b)).collect();
                    assert_eq!(t.range(lo, hi), want, "range {lo}..={hi}");
                }
                if step % 64 == 0 {
                    t.check()
                        .unwrap_or_else(|e| panic!("order {order} step {step}: {e}"));
                }
            }
            t.check().unwrap();
            assert_eq!(t.len(), oracle.len());
            assert_eq!(t.keys(), oracle.keys().copied().collect::<Vec<_>>());
        }
    }

    #[test]
    fn delete_everything_leaves_empty_tree() {
        let mut t = Bplus::new(4);
        for k in 0..100 {
            t.insert(k, k);
        }
        for k in (0..100).rev() {
            t.remove(k);
        }
        assert!(t.is_empty());
        assert_eq!(t.keys(), Vec::<u64>::new());
        // Reuse after collapse.
        t.insert(7, 70);
        assert_eq!(t.get(7), Some(70));
        assert!(t.check().is_ok());
    }

    #[test]
    fn adversarial_orders_hold_invariants() {
        let mut rng = SplitMix64::new(7);
        for seq in [1usize, 2, 3] {
            let mut t = Bplus::new(5);
            let mut oracle = BTreeMap::new();
            let n = 120u64;
            // Pattern seq 1: ascending, 2: descending, 3: random.
            let order_keys: Vec<u64> = (0..n)
                .map(|i| match seq {
                    1 => i,
                    2 => n - 1 - i,
                    _ => rng.below(500) as u64,
                })
                .collect();
            for &k in &order_keys {
                t.insert(k, k + 1);
                oracle.insert(k, k + 1);
            }
            for &k in order_keys.iter().step_by(2) {
                assert_eq!(t.remove(k), oracle.remove(&k));
            }
            t.check().unwrap();
            assert_eq!(t.keys(), oracle.keys().copied().collect::<Vec<_>>());
        }
    }
}
