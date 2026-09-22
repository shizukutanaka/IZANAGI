//! Rendezvous hashing (HRW / "highest random weight") — assign each
//! key to one of `N` nodes so that removing a node remaps *only* the
//! keys that lived on it (minimal disruption). Shard→peer assignment
//! for replicated sim stores, deterministic lobby selection, cache
//! partitioning without a coordinator. A pure function of
//! `(node list, key)` — every peer computes the same assignment with
//! zero communication.
//!
//! ```
//! use izanagi_kit::chash::pick;
//! let nodes = [10u64, 20, 30, 40];
//! let home = pick(&nodes, b"player:42").unwrap_or(0);
//! assert!(nodes.contains(&home));
//! // Removing a node that is NOT the home leaves the key in place.
//! let reduced: Vec<u64> = nodes.iter().copied().filter(|&n| n != 10).collect();
//! assert_eq!(pick(&reduced, b"player:42"), Some(home));
//! ```

use crate::world_hash::Fnv1a;

/// Weight of `(seed, node, key)` — the rendezvous score.
fn weight(seed: u64, node: u64, key: &[u8]) -> u64 {
    let mut h = Fnv1a::new();
    h.write_u64(seed);
    h.write_u64(node);
    h.write_bytes(key);
    h.finish()
}

/// The node `key` maps to: `argmax_n weight(seed, n, key)`, ties broken
/// by smallest node id so the winner never depends on node order.
/// `None` on an empty node list.
pub fn pick_seed(seed: u64, nodes: &[u64], key: &[u8]) -> Option<u64> {
    let mut best: Option<(u64, u64)> = None; // (weight, node)
    for &n in nodes {
        let w = weight(seed, n, key);
        let better = match best {
            None => true,
            Some((bw, bn)) => w > bw || (w == bw && n < bn),
        };
        if better {
            best = Some((w, n));
        }
    }
    best.map(|(_, n)| n)
}

/// `pick_seed(0, nodes, key)`.
pub fn pick(nodes: &[u64], key: &[u8]) -> Option<u64> {
    pick_seed(0, nodes, key)
}

/// `(counts)` — assign every key and return per-node hit counts in the
/// same order as `nodes`. Balance auditing; keys are byte slices.
pub fn distribution<'a>(nodes: &[u64], keys: impl Iterator<Item = &'a [u8]>) -> Vec<u64> {
    let mut counts = vec![0u64; nodes.len()];
    let mut index = std::collections::BTreeMap::new();
    for (i, &n) in nodes.iter().enumerate() {
        index.insert(n, i);
    }
    for k in keys {
        if let Some(home) = pick(nodes, k) {
            if let Some(&i) = index.get(&home) {
                counts[i] += 1;
            }
        }
    }
    counts
}

/// Top-`r` nodes by rendezvous score, best first — replication groups:
/// place `r` copies of the shard on the first `r` winners. Order is
/// canonical (score desc, node id asc).
pub fn pick_top_seed(seed: u64, nodes: &[u64], key: &[u8], r: usize) -> Vec<u64> {
    let mut scored: Vec<(u64, u64)> = nodes.iter().map(|&n| (weight(seed, n, key), n)).collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.truncate(r);
    scored.into_iter().map(|(_, n)| n).collect()
}

/// `pick_top_seed(0, nodes, key, r)`.
pub fn pick_top(nodes: &[u64], key: &[u8], r: usize) -> Vec<u64> {
    pick_top_seed(0, nodes, key, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn minimal_disruption_and_order_independence() {
        let mut rng = SplitMix64::new(0xDEAD_CAFE);
        let nodes: Vec<u64> = (1..=8).collect();
        let mut keys = Vec::new();
        for _ in 0..400 {
            let k = rng.next_u64().to_le_bytes();
            keys.push(k);
        }
        // Base assignment.
        let base: BTreeMap<Vec<u8>, u64> = keys
            .iter()
            .map(|k| (k.to_vec(), pick(&nodes, k).unwrap_or(0)))
            .collect();
        // Shuffle the node list — assignment must not move.
        let mut shuffled = nodes.clone();
        shuffled.reverse();
        for k in &keys {
            assert_eq!(pick(&shuffled, k), pick(&nodes, k));
        }
        // Remove each node in turn: only its own keys may remap, and
        // they land exactly on pick over the survivors.
        for &doomed in &nodes {
            let survivors: Vec<u64> = nodes.iter().copied().filter(|&n| n != doomed).collect();
            for k in &keys {
                let home = base[&k.to_vec()];
                let moved = pick(&survivors, k);
                if home != doomed {
                    assert_eq!(moved, Some(home));
                } else {
                    assert_ne!(moved, Some(doomed));
                    assert!(moved.is_some());
                }
            }
        }
        // pick_top: first entry == pick; distinct nodes; deterministic.
        let top = pick_top(&nodes, &keys[0], 3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0], pick(&nodes, &keys[0]).unwrap_or(0));
        let mut dedup = top.clone();
        dedup.dedup();
        assert_eq!(dedup, top);
        assert_eq!(pick(&[], b"k"), None);
        assert_eq!(pick_top(&nodes, b"k", 20).len(), nodes.len());
        // Seeded variants: different seed can move keys; same call
        // shape stays deterministic.
        let s1 = pick_seed(42, &nodes, &keys[0]);
        assert_eq!(s1, pick_seed(42, &nodes, &keys[0]));
        assert_eq!(pick_seed(42, &[], b"k"), None);
        let t1 = pick_top_seed(42, &nodes, &keys[0], 2);
        assert_eq!(t1.len(), 2);
        assert_eq!(t1[0], pick_seed(42, &nodes, &keys[0]).unwrap_or(0));
    }

    #[test]
    fn rough_balance() {
        let nodes: Vec<u64> = (1..=6).collect();
        let mut rng = SplitMix64::new(7);
        let keys: Vec<Vec<u8>> = (0..3000)
            .map(|_| rng.next_u64().to_le_bytes().to_vec())
            .collect();
        let counts = distribution(&nodes, keys.iter().map(|k| k.as_slice()));
        assert_eq!(counts.iter().sum::<u64>(), 3000);
        for &c in &counts {
            assert!(c > 150 && c < 1200, "count {c}");
        }
    }
}
