//! CRDTs (conflict-free replicated data types, Shapiro et al. 2011):
//! state-based counters and sets whose `merge` is a join in a
//! join-semilattice — **commutative, associative, idempotent** — so
//! replicas converge to the same value no matter the order or duplication
//! of merges. For a deterministic sim this gives a principled model of
//! replicated tallies: scores or resource counts merged from several
//! sources (multi-source kill feeds, sharded harvest, delta-synced
//! inventories) where the merge order must not matter for the replay hash.
//!
//! Everything is integers: counters are per-replica `u64` vectors joined
//! by elementwise max; the signed counter subtracts two of them; sets are
//! `BTreeSet`s (canonical order, deterministic iteration) joined by union
//! or — for `TwoPhaseSet` — by union-of-additions plus union-of-tombstones.
//!
//! ```
//! use izanagi_kit::crdt::PNCounter;
//!
//! let mut a = PNCounter::new(2); // 2 replicas
//! let mut b = PNCounter::new(2);
//! a.increment(0); a.increment(0); a.decrement(0);
//! b.increment(1);
//! let mut x = a.clone();
//! x.merge(&b);
//! let mut y = b.clone();
//! y.merge(&a); // merges commute
//! assert_eq!(x.value(), y.value());
//! assert_eq!(x.value(), 2); // +2 -1 +1
//! ```

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::BTreeSet;

/// G-Counter: grow-only counter over `replicas` lanes. `merge` takes the
/// elementwise maximum, so a lane's own increments are never lost and
/// stale merges are absorbed (idempotent).
#[derive(Clone, Debug, Default)]
pub struct GCounter {
    counts: Vec<u64>,
}

impl GCounter {
    /// A counter with `replicas` lanes, all zero.
    pub fn new(replicas: usize) -> Self {
        GCounter {
            counts: vec![0; replicas],
        }
    }

    /// Number of replica lanes.
    pub fn replicas(&self) -> usize {
        self.counts.len()
    }

    /// Increment replica `r`'s lane by one (saturating at `u64::MAX`).
    /// Out-of-range `r` is ignored — a wrong replica id must never panic.
    pub fn increment(&mut self, r: usize) {
        if let Some(c) = self.counts.get_mut(r) {
            *c = c.saturating_add(1);
        }
    }

    /// Add `n` to replica `r`'s lane (saturating). Out-of-range ignored.
    pub fn add(&mut self, r: usize, n: u64) {
        if let Some(c) = self.counts.get_mut(r) {
            *c = c.saturating_add(n);
        }
    }

    /// The global total: saturating sum over all lanes.
    pub fn value(&self) -> u64 {
        self.counts.iter().fold(0u64, |a, &c| a.saturating_add(c))
    }

    /// Join: elementwise max. Different replica counts take the shorter
    /// length — lanes the other side has are treated as zero.
    pub fn merge(&mut self, other: &Self) {
        let n = self.counts.len().min(other.counts.len());
        for i in 0..n {
            self.counts[i] = self.counts[i].max(other.counts[i]);
        }
    }

    /// Per-lane counts (read-only: the join needs max, and increments own them).
    pub fn lanes(&self) -> &[u64] {
        &self.counts
    }
}

impl DetHash for GCounter {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.counts.det_hash(h);
    }
}

/// PN-Counter: signed counter as two G-Counters — `value = p − n`.
/// Increments go to `p`, decrements to `n`; the pair merges independently,
/// which is what makes negative totals order-safe.
#[derive(Clone, Debug, Default)]
pub struct PNCounter {
    p: GCounter,
    n: GCounter,
}

impl PNCounter {
    /// A signed counter with `replicas` lanes, zero.
    pub fn new(replicas: usize) -> Self {
        PNCounter {
            p: GCounter::new(replicas),
            n: GCounter::new(replicas),
        }
    }

    /// `replica + 1`.
    pub fn increment(&mut self, r: usize) {
        self.p.increment(r);
    }

    /// `replica − 1`.
    pub fn decrement(&mut self, r: usize) {
        self.n.increment(r);
    }

    /// Current signed value, `p − n` (saturating at `i64` bounds).
    pub fn value(&self) -> i64 {
        self.p.value().min(i64::MAX as u64) as i64 - self.n.value().min(i64::MAX as u64) as i64
    }

    /// Join both halves; order and duplication still cannot matter.
    pub fn merge(&mut self, other: &Self) {
        self.p.merge(&other.p);
        self.n.merge(&other.n);
    }

    /// The positive half (how many increments are recorded).
    pub fn increments(&self) -> &GCounter {
        &self.p
    }

    /// The negative half (how many decrements are recorded).
    pub fn decrements(&self) -> &GCounter {
        &self.n
    }
}

impl DetHash for PNCounter {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.p.det_hash(h);
        self.n.det_hash(h);
    }
}

/// G-Set: grow-only set; `merge` is union. Converges by definition —
/// `insert` can only ever grow the answer. Used as the positive half of
/// `TwoPhaseSet` and alone where deletions do not exist (e.g. "which
/// pickups were ever collected" is not a G-Set — use the 2P-Set below).
#[derive(Clone, Debug, Default)]
pub struct GSet<T: Ord> {
    items: BTreeSet<T>,
}

impl<T: Ord> GSet<T> {
    /// Empty set.
    pub fn new() -> Self {
        GSet {
            items: BTreeSet::new(),
        }
    }

    /// Insert `x`; reports whether it was new.
    pub fn insert(&mut self, x: T) -> bool {
        self.items.insert(x)
    }

    /// Membership.
    pub fn contains(&self, x: &T) -> bool {
        self.items.contains(x)
    }

    /// Cardinality.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Join: union.
    pub fn merge(&mut self, other: &Self)
    where
        T: Clone,
    {
        for x in &other.items {
            self.items.insert(x.clone());
        }
    }

    /// Iterate in sorted (canonical) order — deterministic, not insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

impl<T: Ord + DetHash> DetHash for GSet<T> {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.items.det_hash(h);
    }
}

/// 2P-Set: two G-Sets — additions and tombstones. `remove` wins **for
/// ever**: once removed, a later `insert` (or a merge carrying the
/// addition but not yet the removal) does not resurrect the element. That
/// asymmetry is the price of deletions without timestamps; where
/// add-after-remove must work you need an LWW/OR-Set (tagged elements),
/// which this module does not provide.
#[derive(Clone, Debug, Default)]
pub struct TwoPhaseSet<T: Ord> {
    added: GSet<T>,
    removed: GSet<T>,
}

impl<T: Ord> TwoPhaseSet<T> {
    /// Empty set.
    pub fn new() -> Self {
        TwoPhaseSet {
            added: GSet::new(),
            removed: GSet::new(),
        }
    }

    /// Insert `x` — a no-op if `x` was ever removed (tombstone wins).
    pub fn insert(&mut self, x: T)
    where
        T: Clone,
    {
        if !self.removed.contains(&x) {
            self.added.insert(x);
        }
    }

    /// Remove `x`: records the tombstone even if `x` is not currently
    /// present — removes propagate to additions that arrive concurrently.
    pub fn remove(&mut self, x: T)
    where
        T: Clone,
    {
        if self.added.contains(&x) {
            self.removed.insert(x);
        }
    }

    /// Membership of the *live* set: added and not tombstoned.
    pub fn contains(&self, x: &T) -> bool {
        self.added.contains(x) && !self.removed.contains(x)
    }

    /// Live cardinality.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Whether the live set is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Join: union both halves; the tombstone union makes removals sticky.
    pub fn merge(&mut self, other: &Self)
    where
        T: Clone,
    {
        self.added.merge(&other.added);
        self.removed.merge(&other.removed);
    }

    /// Iterate live elements in canonical order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.added
            .items
            .iter()
            .filter(move |x| !self.removed.contains(x))
    }
}

impl<T: Ord + DetHash> DetHash for TwoPhaseSet<T> {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.added.det_hash(h);
        self.removed.det_hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcounter_merge_is_commutative_associative_idempotent() {
        let mk = |lanes: &[(usize, u64)]| {
            let mut g = GCounter::new(3);
            for &(r, n) in lanes {
                g.add(r, n);
            }
            g
        };
        let (a, b, c) = (mk(&[(0, 5)]), mk(&[(1, 3), (2, 7)]), mk(&[(0, 2), (2, 1)]));
        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);
        assert_eq!(ab.lanes(), ba.lanes()); // commutative
        let mut ab_c = ab.clone();
        ab_c.merge(&c);
        let mut bc = b.clone();
        bc.merge(&c);
        let mut a_bc = a.clone();
        a_bc.merge(&bc);
        assert_eq!(ab_c.lanes(), a_bc.lanes()); // associative
        let mut idem = a.clone();
        idem.merge(&a);
        assert_eq!(idem.lanes(), a.lanes()); // idempotent
        assert_eq!(ab.lanes(), &[5, 3, 7]);
    }

    #[test]
    fn lane_and_half_accessors_reflect_state() {
        let mut g = GCounter::new(3);
        g.add(2, 7);
        assert_eq!(g.replicas(), 3);
        assert_eq!(g.lanes()[2], 7);
        let mut p = PNCounter::new(1);
        p.increment(0);
        p.decrement(0);
        p.decrement(0);
        assert_eq!(p.increments().value(), 1);
        assert_eq!(p.decrements().value(), 2);
        assert_eq!(p.value(), -1);
    }

    #[test]
    fn gcounter_adds_only_merge_upward() {
        // Simulated divergence: two replicas each see half the traffic.
        let mut a = GCounter::new(2);
        let mut b = GCounter::new(2);
        a.increment(0);
        a.increment(0);
        b.increment(1);
        a.merge(&b);
        assert_eq!(a.value(), 3);
        b.merge(&a); // redundant re-merge is absorbed
        assert_eq!(b.value(), 3);
        b.merge(&a);
        assert_eq!(b.value(), 3);
    }

    #[test]
    fn pncounter_negative_totals_converge() {
        let mut a = PNCounter::new(2);
        let mut b = PNCounter::new(2);
        a.decrement(0); // a goes negative locally
        b.increment(1);
        b.increment(1);
        a.merge(&b);
        b.merge(&a);
        assert_eq!(a.value(), 1);
        assert_eq!(b.value(), 1);
    }

    #[test]
    fn out_of_range_replica_is_ignored() {
        let mut g = GCounter::new(1);
        g.increment(9);
        g.add(usize::MAX, 4);
        assert_eq!(g.value(), 0);
        let mut p = PNCounter::new(0);
        p.increment(0);
        p.decrement(0);
        assert_eq!(p.value(), 0);
    }

    #[test]
    fn gset_union_converges_in_any_order() {
        let mut a = GSet::new();
        let mut b = GSet::new();
        a.insert(3);
        a.insert(1);
        b.insert(2);
        b.insert(3);
        let mut x = a.clone();
        x.merge(&b);
        let mut y = b.clone();
        y.merge(&a);
        assert_eq!(x.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(x.iter().collect::<Vec<_>>(), y.iter().collect::<Vec<_>>());
        assert!(x.contains(&2));
        assert_eq!(x.len(), 3);
    }

    #[test]
    fn two_phase_remove_wins_over_concurrent_add() {
        // Replica a adds, replica b removes before seeing the add — remove wins.
        let mut a = TwoPhaseSet::new();
        let mut b = TwoPhaseSet::new();
        a.insert(7u32);
        b.insert(7u32);
        b.remove(7u32);
        let mut merged = a.clone();
        merged.merge(&b);
        assert!(!merged.contains(&7));
        merged.insert(7); // tombstone is permanent
        assert!(!merged.contains(&7));
        // Other order: remove-then-add on the same replica — still dead.
        let mut c = TwoPhaseSet::new();
        c.insert(9u32);
        c.remove(9u32);
        c.insert(9u32);
        assert!(!c.contains(&9));
        assert!(c.is_empty());
    }

    #[test]
    fn two_phase_live_iteration_skips_tombstones() {
        let mut s = TwoPhaseSet::new();
        for x in [5u32, 1, 8, 3] {
            s.insert(x);
        }
        s.remove(1);
        assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![3, 5, 8]);
        assert_eq!(s.len(), 3);
        let mut t = TwoPhaseSet::new();
        t.insert(1u32);
        t.insert(9u32);
        s.merge(&t);
        // 1 stays removed (tombstone wins over t's add), 9 appears.
        assert_eq!(s.iter().copied().collect::<Vec<_>>(), vec![3, 5, 8, 9]);
    }
}
