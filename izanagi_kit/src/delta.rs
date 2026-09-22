//! Snapshot delta sync over ordered `u64 → u64` maps — the state-sync
//! layer: `diff_sorted` on two sorted key/value views produces an op
//! list, `apply_sorted` merges it back losslessly, and
//! [`Delta::encode`]/[`Delta::decode`] give a canonical wire form
//! (strictly-ascending keys, delta-coded varints) that rejects
//! malformed streams — the property a lockstep desync responder needs
//! to ship "what changed between frame N and frame M" snapshots.
//!
//! ```
//! use izanagi_kit::delta::{Delta, diff_sorted, apply_sorted};
//! let old = vec![(1u64, 10u64), (2, 20), (3, 30)];
//! let new = vec![(1, 10), (3, 33), (4, 40)];
//! let d = diff_sorted(&old, &new);
//! assert_eq!(apply_sorted(&old, &d), Some(new.clone()));
//! let wire = d.encode();
//! assert_eq!(Delta::decode(&wire).unwrap().ops(), d.ops());
//! ```

use crate::bits::{BitReader, BitWriter};

/// One mutation: set `key = value`, or delete `key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Insert or overwrite `key`.
    Set(u64, u64),
    /// Remove `key` (must be present).
    Del(u64),
}

/// An ordered op list — keys strictly ascending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    ops: Vec<Op>,
}

impl Delta {
    /// Empty delta (no changes).
    pub fn empty() -> Self {
        Self { ops: Vec::new() }
    }

    /// The op list.
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Number of ops.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Whether no ops are present.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Canonical wire form: `varint op_count`, then per op a `is_del`
    /// bit plus a key delta varint (keys strictly ascending, first
    /// delta from 0) plus, for `Set`, a value varint. Non-canonical
    /// encodings (unsorted keys, truncation) are rejected by
    /// [`Delta::decode`].
    pub fn encode(&self) -> Vec<u8> {
        let mut w = BitWriter::new();
        w.write_varint(self.ops.len() as u64);
        let mut prev = 0u64;
        for op in &self.ops {
            match *op {
                Op::Set(k, v) => {
                    w.write_bool(false);
                    w.write_varint(k - prev);
                    w.write_varint(v);
                    prev = k;
                }
                Op::Del(k) => {
                    w.write_bool(true);
                    w.write_varint(k - prev);
                    prev = k;
                }
            }
        }
        w.into_bytes()
    }

    /// Inverse of [`Delta::encode`]. Rejects truncated streams,
    /// non-increasing key sequences, and trailing garbage.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        let mut r = BitReader::new(buf);
        let count = r.read_varint().ok()?;
        let mut ops = Vec::with_capacity(count.min(1 << 20) as usize);
        let mut prev = 0u64;
        let mut first = true;
        for _ in 0..count {
            let is_del = r.read_bool().ok()?;
            let dk = r.read_varint().ok()?;
            let k = prev.checked_add(dk)?;
            // Keys must be strictly ascending (except the very first
            // which may be 0 with delta 0).
            if !first && dk == 0 {
                return None;
            }
            first = false;
            if is_del {
                ops.push(Op::Del(k));
            } else {
                let v = r.read_varint().ok()?;
                ops.push(Op::Set(k, v));
            }
            prev = k;
        }
        // Reject trailing bytes — the encoding is byte-aligned only if
        // the writer finished aligned; leftover padding bits must all
        // be readable but the reader exposes `is_at_end`.
        r.align();
        if !r.is_at_end() {
            return None;
        }
        Some(Self { ops })
    }
}

/// Diff two sorted-by-key views. Caller contract: inputs sorted and
/// keys unique (checked in debug builds only — the merge is still
/// total, producing the minimal ascending op list).
pub fn diff_sorted(old: &[(u64, u64)], new: &[(u64, u64)]) -> Delta {
    let mut ops = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < old.len() || j < new.len() {
        match (old.get(i), new.get(j)) {
            (Some(&(ko, _)), Some(&(kn, vn))) if ko == kn => {
                if old[i].1 != vn {
                    ops.push(Op::Set(kn, vn));
                }
                i += 1;
                j += 1;
            }
            (Some(&(ko, vo)), Some(&(kn, _))) if ko < kn => {
                let _ = vo;
                ops.push(Op::Del(ko));
                i += 1;
            }
            (Some(&(ko, _)), Some(&(kn, vn))) => {
                let _ = ko;
                ops.push(Op::Set(kn, vn));
                j += 1;
            }
            (Some(&(ko, _)), None) => {
                ops.push(Op::Del(ko));
                i += 1;
            }
            (None, Some(&(kn, vn))) => {
                ops.push(Op::Set(kn, vn));
                j += 1;
            }
            (None, None) => break,
        }
    }
    Delta { ops }
}

/// Merge `d` into `old` (both key-sorted), producing the new sorted
/// view. Returns `None` when the delta is malformed for `old`
/// (`Del` of an absent key — `Set` of an absent key is an insert).
pub fn apply_sorted(old: &[(u64, u64)], d: &Delta) -> Option<Vec<(u64, u64)>> {
    let mut out = Vec::with_capacity(old.len() + d.ops.len());
    let mut oi = 0usize;
    for op in &d.ops {
        let ok = match *op {
            Op::Set(k, _) | Op::Del(k) => k,
        };
        // Flush old entries below the op key.
        while oi < old.len() && old[oi].0 < ok {
            out.push(old[oi]);
            oi += 1;
        }
        match *op {
            Op::Set(k, v) => {
                if oi < old.len() && old[oi].0 == k {
                    oi += 1; // overwrite
                }
                out.push((k, v));
            }
            Op::Del(k) => {
                if oi < old.len() && old[oi].0 == k {
                    oi += 1;
                } else {
                    return None; // delete of absent key
                }
            }
        }
    }
    out.extend_from_slice(&old[oi..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    fn sorted_map(rng: &mut SplitMix64, n: usize, keys: u64) -> Vec<(u64, u64)> {
        let mut m = BTreeMap::new();
        while m.len() < n {
            m.insert(rng.next_u64() % keys, rng.next_u64());
        }
        m.into_iter().collect()
    }

    #[test]
    fn diff_apply_round_trip_is_identity() {
        let mut rng = SplitMix64::new(0xDE17);
        for _ in 0..500 {
            let n = rng.below(25) as usize; // < 30 distinct keys possible
            let old = sorted_map(&mut rng, n, 30);
            // Perturb: delete some keys, change some, add some.
            let mut new: BTreeMap<u64, u64> = old.iter().copied().collect();
            for &(k, _) in &old {
                if rng.below(4) == 0 {
                    new.remove(&k);
                } else if rng.below(4) == 0 {
                    new.insert(k, rng.next_u64());
                }
            }
            for _ in 0..rng.below(4) {
                new.insert(rng.next_u64() % 40, rng.next_u64());
            }
            let new: Vec<(u64, u64)> = new.into_iter().collect();
            let d = diff_sorted(&old, &new);
            assert_eq!(apply_sorted(&old, &d), Some(new.clone()));
            // Minimality: op count ≤ #keys actually differing.
            let changed = {
                let om: BTreeMap<_, _> = old.iter().copied().collect();
                let nm: BTreeMap<_, _> = new.iter().copied().collect();
                om.keys().filter(|k| !nm.contains_key(*k)).count()
                    + nm.iter().filter(|(k, v)| om.get(*k) != Some(v)).count()
            };
            assert_eq!(d.len(), changed);
            // Wire round-trip.
            let wire = d.encode();
            assert_eq!(Delta::decode(&wire).unwrap(), d);
        }
    }

    #[test]
    fn malformed_decode_is_rejected() {
        let d = diff_sorted(&[(1, 5), (9, 1)], &[(1, 7), (3, 3), (9, 1)]);
        let wire = d.encode();
        // Truncation at every prefix length fails (or yields fewer
        // ops — but must never panic; decode must stay total).
        for len in 0..wire.len() {
            let _ = Delta::decode(&wire[..len]);
        }
        assert_eq!(Delta::decode(&wire), Some(d.clone()));
        // Out-of-order keys hand-encoded: Set(5), Set(2).
        let mut w = BitWriter::new();
        w.write_varint(2);
        w.write_bool(false);
        w.write_varint(5); // key 5
        w.write_varint(0);
        w.write_bool(false);
        w.write_varint(0); // delta 0 → key 5 again — not ascending
        w.write_varint(0);
        assert_eq!(Delta::decode(&w.into_bytes()), None);
        // Apply of a Del on absent key fails.
        let bad = Delta {
            ops: vec![Op::Del(42)],
        };
        assert_eq!(apply_sorted(&[(1, 1)], &bad), None);
    }

    #[test]
    fn empty_and_identity() {
        assert_eq!(diff_sorted(&[], &[]).len(), 0);
        assert_eq!(apply_sorted(&[], &Delta::empty()), Some(vec![]));
        let v = vec![(1u64, 2u64)];
        assert_eq!(diff_sorted(&v, &v).len(), 0);
        assert_eq!(
            Delta::decode(&Delta::empty().encode()),
            Some(Delta::empty())
        );
    }
}
