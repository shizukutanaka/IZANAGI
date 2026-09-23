//! LWW-Element-Set — a state-based CRDT where each element's
//! fate is decided by timestamp, not by observed dots.
//!
//! Every `add` and `remove` records a `(clock, replica)` stamp.
//! An element is live iff its winning stamp is an add; ties at
//! equal clocks resolve toward **remove** (the bias nearly all
//! deployments pick — an add/remove racing at the same logical
//! instant deletes the element, matching e.g. Riak's `.add` /
//! `.remove` conflict rule). Stamps are per-replica Lamport
//! clocks, so the set is fully deterministic — no wall time.
//!
//! Contrast with [`crate::orset`]: an OR-Set remove can only
//! kill dots it has *seen*, so a concurrent add always wins.
//! Here a remove always writes a stamp — a concurrent remove
//! with a later clock wins over an add it never observed.
//!
//! ```
//! use izanagi_kit::lww::LwwSet;
//! let mut a = LwwSet::new();
//! let mut b = LwwSet::new();
//! a.add(1, 'x');          // (t=1, r=1)
//! b.merge(&a);
//! b.remove(2, &'x');      // (t=2, r=2) — later wins → dead
//! assert!(!b.contains(&'x'));
//! a.add(1, 'x');          // (t=2, r=1) — tie → remove wins
//! b.merge(&a);
//! assert!(!b.contains(&'x'));
//! ```

use std::collections::BTreeMap;

/// A write stamp: Lamport clock plus the issuing replica id.
/// Ordering is `(clock, replica)` lexicographic — deterministic
/// and total, so merges never depend on message order.
pub type Stamp = (u64, u32);

/// Membership rule: live iff the add stamp strictly beats the
/// remove stamp — equal clocks bias toward remove, and the
/// stamp order itself is total, so `a > r` is the whole rule.
fn live_at(add: Option<Stamp>, rem: Option<Stamp>) -> bool {
    match (add, rem) {
        (Some(a), Some(r)) => a > r,
        (Some(_), None) => true,
        _ => false,
    }
}

/// LWW element set over `V`.
///
/// `adds` / `rems` keep the *latest* stamp each element was
/// added or removed with; `ctr` is this replica's Lamport clock
/// (bumped on every local write and on merge to `max(seen)+1`,
/// so two replicas' stamps interleave deterministically).
#[derive(Clone, Debug)]
pub struct LwwSet<V: Ord + Clone> {
    adds: BTreeMap<V, Stamp>,
    rems: BTreeMap<V, Stamp>,
    ctr: u64,
}

impl<V: Ord + Clone> LwwSet<V> {
    /// Empty set.
    pub fn new() -> LwwSet<V> {
        LwwSet {
            adds: BTreeMap::new(),
            rems: BTreeMap::new(),
            ctr: 0,
        }
    }

    /// Issue a fresh stamp for a local write.
    fn tick(&mut self, replica: u32) -> Stamp {
        self.ctr += 1;
        (self.ctr, replica)
    }

    /// Add `v` from `replica`; returns the stamp used.
    pub fn add(&mut self, replica: u32, v: V) -> Stamp {
        let s = self.tick(replica);
        let e = self.adds.entry(v).or_insert((0, 0));
        if s > *e {
            *e = s;
        }
        s
    }

    /// Remove `v` from `replica`. Unlike [`crate::orset`]'s
    /// remove, this always writes a stamp — even for an element
    /// the replica has never seen — so a concurrent remove can
    /// win by clock. Returns whether the new stamp currently
    /// makes `v` dead (i.e. beats the stored add stamp).
    pub fn remove(&mut self, replica: u32, v: &V) -> bool {
        let s = self.tick(replica);
        let e = self.rems.entry(v.clone()).or_insert((0, 0));
        if s > *e {
            *e = s;
        }
        !live_at(self.adds.get(v).copied(), self.rems.get(v).copied())
    }

    /// `v` live? Its add stamp beats its remove stamp.
    pub fn contains(&self, v: &V) -> bool {
        live_at(self.adds.get(v).copied(), self.rems.get(v).copied())
    }

    /// Live members, sorted.
    pub fn values(&self) -> Vec<V> {
        self.adds
            .iter()
            .filter(|(v, a)| live_at(Some(**a), self.rems.get(*v).copied()))
            .map(|(v, _)| v.clone())
            .collect()
    }

    /// Live members that joined at or after `clock` — a cheap
    /// "what changed since" for delta shipping.
    pub fn added_since(&self, clock: u64) -> Vec<V> {
        self.adds
            .iter()
            .filter(|(v, a)| a.0 >= clock && live_at(Some(**a), self.rems.get(*v).copied()))
            .map(|(v, _)| v.clone())
            .collect()
    }

    /// Number of live members.
    pub fn len(&self) -> usize {
        self.values().len()
    }

    /// True iff no live members.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Merge `other` into `self` — per-side max stamp wins.
    /// Commutative, associative, idempotent.
    pub fn merge(&mut self, other: &LwwSet<V>) {
        for (v, s) in &other.adds {
            let e = self.adds.entry(v.clone()).or_insert((0, 0));
            if *s > *e {
                *e = *s;
            }
        }
        for (v, s) in &other.rems {
            let e = self.rems.entry(v.clone()).or_insert((0, 0));
            if *s > *e {
                *e = *s;
            }
        }
        if other.ctr > self.ctr {
            self.ctr = other.ctr;
        }
    }
}

impl<V: Ord + Clone> Default for LwwSet<V> {
    fn default() -> LwwSet<V> {
        LwwSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Textbook membership over shadow stamp maps — the
    /// oracle never touches `LwwSet` internals.
    fn expect(adds: &BTreeMap<char, Stamp>, rems: &BTreeMap<char, Stamp>, v: char) -> bool {
        match (adds.get(&v), rems.get(&v)) {
            (Some(a), Some(r)) => a > r,
            (Some(_), None) => true,
            _ => false,
        }
    }

    #[test]
    fn add_remove_stamps() {
        let mut s = LwwSet::new();
        assert!(s.is_empty());
        s.add(1, 'a');
        assert!(s.contains(&'a') && s.len() == 1);
        // later remove wins
        assert!(s.remove(1, &'a'));
        assert!(!s.contains(&'a'));
        // still-later add revives
        s.add(1, 'a');
        assert!(s.contains(&'a'));
        // unseen-element remove writes a stamp anyway —
        // a delayed earlier add cannot resurrect 'b'
        assert!(s.remove(1, &'b'));
        let mut t = LwwSet::new();
        t.add(2, 'b'); // t's ctr=1 < s's stamp (2,·)
        s.merge(&t);
        assert!(!s.contains(&'b'));
        // ordering
        s.add(1, 'd');
        s.add(1, 'c');
        assert_eq!(s.values(), vec!['a', 'c', 'd']);
        assert_eq!(s.added_since(5), vec!['c', 'd']);
        // is_empty after removing all
        assert!(s.remove(1, &'a') && s.remove(1, &'c') && s.remove(1, &'d'));
        assert!(s.is_empty());
    }

    type View = (BTreeMap<char, Stamp>, BTreeMap<char, Stamp>, u64);

    fn view_merge(dst: &mut View, src: &View) {
        for (v, s) in &src.0 {
            let e = dst.0.entry(*v).or_insert((0, 0));
            if *s > *e {
                *e = *s;
            }
        }
        for (v, s) in &src.1 {
            let e = dst.1.entry(*v).or_insert((0, 0));
            if *s > *e {
                *e = *s;
            }
        }
        if src.2 > dst.2 {
            dst.2 = src.2;
        }
    }

    fn merge_into(sets: &mut [LwwSet<char>], dst: usize, src: usize) {
        if dst < src {
            let (l, r) = sets.split_at_mut(src);
            l[dst].merge(&r[0]);
        } else {
            let (l, r) = sets.split_at_mut(dst);
            r[0].merge(&l[src]);
        }
    }

    fn merge_view(views: &mut [View], dst: usize, src: usize) {
        if dst < src {
            let (l, r) = views.split_at_mut(src);
            view_merge(&mut l[dst], &r[0]);
        } else {
            let (l, r) = views.split_at_mut(dst);
            view_merge(&mut r[0], &l[src]);
        }
    }

    #[test]
    fn oracle_op_logs() {
        let mut rng = SplitMix64::new(31);
        for _round in 0..120 {
            let mut sets = [LwwSet::<char>::new(), LwwSet::new(), LwwSet::new()];
            let mut views: [View; 3] = [
                (BTreeMap::new(), BTreeMap::new(), 0),
                (BTreeMap::new(), BTreeMap::new(), 0),
                (BTreeMap::new(), BTreeMap::new(), 0),
            ];
            let check = |i: usize, sets: &[LwwSet<char>], views: &[View]| {
                for v in 'a'..='d' {
                    assert_eq!(
                        sets[i].contains(&v),
                        expect(&views[i].0, &views[i].1, v),
                        "replica {i} contains {v}"
                    );
                }
            };
            for _step in 0..60 {
                let r = rng.below(3) as usize;
                let v = (b'a' + rng.below(4) as u8) as char;
                match rng.below(4) {
                    0 | 1 => {
                        let s = sets[r].add(r as u32, v);
                        views[r].2 = s.0; // impl's ctr just ticked to s.0
                        let e = views[r].0.entry(v).or_insert((0, 0));
                        if s > *e {
                            *e = s;
                        }
                    }
                    2 => {
                        sets[r].remove(r as u32, &v);
                        views[r].2 += 1;
                        let s = (views[r].2, r as u32);
                        let e = views[r].1.entry(v).or_insert((0, 0));
                        if s > *e {
                            *e = s;
                        }
                    }
                    _ => {
                        let src = rng.below(3) as usize;
                        if src != r {
                            merge_into(&mut sets, r, src);
                            merge_view(&mut views, r, src);
                        }
                    }
                }
                check(r, &sets, &views);
            }
            // convergence + idempotency + order-independence
            let mut all = LwwSet::new();
            for s in &sets {
                all.merge(s);
            }
            for s in &sets {
                all.merge(s);
            }
            let mut all2 = LwwSet::new();
            for s in sets.iter().rev() {
                all2.merge(s);
            }
            assert_eq!(all.values(), all2.values(), "round {_round} convergence");
            let mut u: View = (BTreeMap::new(), BTreeMap::new(), 0);
            for v in &views {
                view_merge(&mut u, v);
            }
            let want: Vec<char> =
                u.0.keys()
                    .copied()
                    .chain(u.1.keys().copied())
                    .collect::<BTreeSet<char>>()
                    .into_iter()
                    .filter(|&v| expect(&u.0, &u.1, v))
                    .collect();
            assert_eq!(all.values(), want, "round {_round} membership");
        }
    }
}
