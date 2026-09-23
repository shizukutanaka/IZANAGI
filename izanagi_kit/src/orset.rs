//! Observed-Remove Set (add-wins OR-Set) — a convergent
//! replicated data type for lockstep/distributed state.
//! Each `add` tags the element with a unique *dot*
//! `(replica, counter)`; each `remove` only covers the dots
//! the removing replica had observed, so an `add` concurrent
//! with a `remove` always survives. `merge` is set-union on
//! both dot maps — commutative, associative, idempotent, and
//! therefore order-independent (safe for eventual delivery
//! in any interleaving).
//!
//! Membership rule: `v ∈ set` iff some add-dot of `v` is not
//! covered by a remove-dot — the "add-wins" semantics, and
//! the reason a re-add after a remove makes the element
//! live again (its new dot was never observed).
//!
//! Each replica must `add` under a **distinct** `replica`
//! id — two replicas issuing dots under the same id can
//! collide (standard OR-Set caveat).
//!
//! ```
//! use izanagi_kit::orset::OrSet;
//! let mut a = OrSet::new();
//! let mut b = OrSet::new();
//! a.add(1, 'x');
//! b.merge(&a);
//! b.remove(&'x'); // saw the add
//! assert!(!b.contains(&'x'));
//! a.add(1, 'x'); // concurrent with the remove → survives
//! b.merge(&a);
//! assert!(b.contains(&'x'));
//! ```
//!
//! Reference: Shapiro et al. (2011) "A comprehensive study
//! of Convergent and Commutative Replicated Data Types".

use std::collections::{BTreeMap, BTreeSet};

/// (replica, counter) unique event tag.
pub type Dot = (u32, u64);

/// Add-wins observed-remove set. `V` is the member type.
pub struct OrSet<V: Ord> {
    adds: BTreeMap<V, BTreeSet<Dot>>,
    removes: BTreeMap<V, BTreeSet<Dot>>,
    /// next counter per replica id issued by this set
    ctr: BTreeMap<u32, u64>,
}

impl<V: Ord + Clone> OrSet<V> {
    /// Empty set.
    pub fn new() -> OrSet<V> {
        OrSet {
            adds: BTreeMap::new(),
            removes: BTreeMap::new(),
            ctr: BTreeMap::new(),
        }
    }

    /// Record `add v` from `replica`; returns the dot issued.
    pub fn add(&mut self, replica: u32, v: V) -> Dot {
        let c = self.ctr.entry(replica).or_insert(0);
        *c += 1;
        let dot = (replica, *c);
        self.adds.entry(v).or_default().insert(dot);
        dot
    }

    /// Remove `v`: covers every add-dot *currently observed*
    /// — concurrent (unseen) adds are unaffected. Returns
    /// whether `v` had any live dot (i.e. was present).
    pub fn remove(&mut self, v: &V) -> bool {
        let Some(dots) = self.adds.get(v) else {
            return false;
        };
        let rem = self.removes.get(v);
        let live = dots.iter().any(|d| match rem {
            Some(r) => !r.contains(d),
            None => true,
        });
        if !live {
            return false;
        }
        let covered: BTreeSet<Dot> = dots.iter().copied().collect();
        self.removes.entry(v.clone()).or_default().extend(covered);
        true
    }

    /// `v` live? Some add-dot not covered by any remove-dot.
    pub fn contains(&self, v: &V) -> bool {
        let Some(dots) = self.adds.get(v) else {
            return false;
        };
        let dead = self.removes.get(v);
        dots.iter().any(|d| match dead {
            Some(rem) => !rem.contains(d),
            None => true,
        })
    }

    /// Live members, sorted.
    pub fn values(&self) -> Vec<V> {
        self.adds
            .iter()
            .filter(|(v, _)| self.contains(v))
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

    /// Merge `other` into `self` — union of add- and
    /// remove-dot sets. `merge` is commutative, associative,
    /// and idempotent: replicas converge under any message
    /// order, duplication, or re-merge.
    pub fn merge(&mut self, other: &OrSet<V>) {
        for (v, dots) in &other.adds {
            self.adds
                .entry(v.clone())
                .or_default()
                .extend(dots.iter().copied());
        }
        for (v, dots) in &other.removes {
            self.removes
                .entry(v.clone())
                .or_default()
                .extend(dots.iter().copied());
        }
    }
}

impl<V: Ord + Clone> Default for OrSet<V> {
    fn default() -> OrSet<V> {
        OrSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Expected membership rebuilt from each replica's op
    /// log — adds/removes are plain set unions over dots;
    /// `contains` is the textbook rule. Independent of the
    /// implementation's data layout.
    fn expect(
        adds: &BTreeMap<char, BTreeSet<Dot>>,
        removes: &BTreeMap<char, BTreeSet<Dot>>,
        v: char,
    ) -> bool {
        let Some(dots) = adds.get(&v) else {
            return false;
        };
        match removes.get(&v) {
            Some(rem) => dots.iter().any(|d| !rem.contains(d)),
            None => !dots.is_empty(),
        }
    }

    #[test]
    fn add_wins_concurrency() {
        let mut a = OrSet::new();
        let mut b = OrSet::new();
        a.add(1, 'x');
        b.merge(&a);
        b.remove(&'x');
        a.add(1, 'x');
        b.merge(&a);
        assert!(b.contains(&'x'));
        // remove observed → dead
        b.remove(&'x');
        assert!(!b.contains(&'x'));
        // another replica's *new* dot still survives
        let mut c = OrSet::new();
        c.add(3, 'x');
        b.merge(&c);
        assert!(b.contains(&'x'));
    }

    type View = (BTreeMap<char, BTreeSet<Dot>>, BTreeMap<char, BTreeSet<Dot>>);

    fn view_union(dst: &mut View, src: &View) {
        for (v, ds) in &src.0 {
            dst.0.entry(*v).or_default().extend(ds.iter().copied());
        }
        for (v, ds) in &src.1 {
            dst.1.entry(*v).or_default().extend(ds.iter().copied());
        }
    }

    /// merge sets[src] into sets[dst] despite the borrow
    /// checker (split_at_mut both ways).
    fn merge_into(sets: &mut [OrSet<char>], dst: usize, src: usize) {
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
            view_union(&mut l[dst], &r[0]);
        } else {
            let (l, r) = views.split_at_mut(dst);
            view_union(&mut r[0], &l[src]);
        }
    }

    #[test]
    fn oracle_op_logs() {
        let mut rng = SplitMix64::new(19);
        for _round in 0..120 {
            // 3 replicas; per-replica expected views shadow the
            // implementation — adds/removes are plain set unions
            // over dots, exactly the textbook membership rule
            let mut sets = [OrSet::<char>::new(), OrSet::new(), OrSet::new()];
            let mut views: [View; 3] = [
                (BTreeMap::new(), BTreeMap::new()),
                (BTreeMap::new(), BTreeMap::new()),
                (BTreeMap::new(), BTreeMap::new()),
            ];
            let mut ctr = [0u64; 3];
            let check = |i: usize, sets: &[OrSet<char>], views: &[View]| {
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
                        ctr[r] += 1;
                        sets[r].add(r as u32, v);
                        views[r].0.entry(v).or_default().insert((r as u32, ctr[r]));
                    }
                    2 => {
                        sets[r].remove(&v);
                        // observed = all add-dots this replica
                        // currently holds for v
                        let seen: BTreeSet<Dot> = views[r].0.get(&v).cloned().unwrap_or_default();
                        views[r].1.entry(v).or_default().extend(seen);
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
            // convergence: union all replicas — order and
            // duplication must not matter
            let mut all = OrSet::new();
            for s in &sets {
                all.merge(s);
            }
            for s in &sets {
                all.merge(s); // idempotent re-merge
            }
            let mut all2 = OrSet::new();
            for s in sets.iter().rev() {
                all2.merge(s);
            }
            assert_eq!(all.values(), all2.values(), "round {_round} convergence");
            // the merged set agrees with unioned expected views
            let mut u: View = (BTreeMap::new(), BTreeMap::new());
            for v in &views {
                view_union(&mut u, v);
            }
            let want: Vec<char> =
                u.0.keys()
                    .copied()
                    .filter(|&v| expect(&u.0, &u.1, v))
                    .collect();
            assert_eq!(all.values(), want, "round {_round} membership");
        }
    }

    #[test]
    fn basics() {
        let mut s = OrSet::new();
        assert!(s.is_empty());
        s.add(1, 'a');
        assert!(s.contains(&'a') && s.len() == 1);
        assert!(s.remove(&'a') && s.is_empty());
        assert!(!s.remove(&'a')); // nothing live → reports false
                                  // value ordering
        s.add(1, 'c');
        s.add(1, 'b');
        assert_eq!(s.values(), vec!['b', 'c']);
    }
}
