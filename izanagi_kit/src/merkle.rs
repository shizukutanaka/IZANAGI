//! Merkle hash tree over `u64` leaf hashes — the state-sync witness
//! for determinism. Two peers can compare roots to learn *whether*
//! they diverged, then [`Merkle::first_diff`] walks down to *where*
//! (the leaf index of the first mismatch) in `O(log n)` node
//! comparisons — the stored-state analogue of
//! [`crate::replay::first_divergence`].
//!
//! Parent hashes combine children through the kit's canonical
//! [`Fnv1a`](crate::world_hash) hasher with a fixed domain
//! tag. Odd nodes at a level are promoted unchanged rather than
//! duplicated, which avoids the malleability of Bitcoin-style
//! duplication.
//!
//! ```
//! use izanagi_kit::merkle::Merkle;
//! let a = Merkle::build(&[1, 2, 3, 4, 5]);
//! let b = Merkle::build(&[1, 2, 3, 4, 6]);
//! assert_ne!(a.root(), b.root());
//! assert_eq!(a.first_diff(&b), Some(4));
//! let proof = a.proof(2).unwrap();
//! assert!(Merkle::verify(3, &proof, a.root()));
//! ```

use crate::world_hash::Fnv1a;

/// Root hash of an empty tree (FNV-1a offset basis — no writes).
pub const EMPTY_ROOT: u64 = 0xcbf2_9ce4_8422_2325;

const DOMAIN_LEAF: u64 = 0x4d45_524b_4c45_4146; // "MERKLEAF"
const DOMAIN_NODE: u64 = 0x4d45_524b_4e4f_4445; // "MERKNODE"

fn hash_leaf(v: u64) -> u64 {
    let mut h = Fnv1a::new();
    h.write_u64(DOMAIN_LEAF);
    h.write_u64(v);
    h.finish()
}

fn hash_node(a: u64, b: u64) -> u64 {
    let mut h = Fnv1a::new();
    h.write_u64(DOMAIN_NODE);
    h.write_u64(a);
    h.write_u64(b);
    h.finish()
}

/// One step of a Merkle proof: the sibling hash, and whether that
/// sibling sits on the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// Sibling's hash at this level.
    pub hash: u64,
    /// True when the sibling is the right child (so the proven path is
    /// the left one).
    pub right: bool,
}

/// A full binary hash tree over domain-hashed leaves. `levels[0]`
/// holds the leaf hashes; each higher level halves, promoting an odd
/// trailing node unchanged.
pub struct Merkle {
    levels: Vec<Vec<u64>>,
    n: usize,
}

impl Merkle {
    /// Build the tree over `leaves`.
    pub fn build(leaves: &[u64]) -> Self {
        let n = leaves.len();
        let mut levels = vec![leaves.iter().map(|&v| hash_leaf(v)).collect::<Vec<u64>>()];
        while levels.last().map(|l| l.len() > 1).unwrap_or(false) {
            let Some(lower) = levels.last() else {
                break;
            };
            let mut next = Vec::with_capacity(lower.len() / 2 + lower.len() % 2);
            for i in 0..lower.len() / 2 {
                next.push(hash_node(lower[2 * i], lower[2 * i + 1]));
            }
            if lower.len() % 2 == 1 {
                next.push(lower[lower.len() - 1]); // promote unchanged
            }
            levels.push(next);
        }
        Self { levels, n }
    }

    /// Number of leaves.
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Root hash — [`EMPTY_ROOT`] when empty.
    pub fn root(&self) -> u64 {
        match self.levels.last() {
            Some(top) if !top.is_empty() => top[0],
            _ => EMPTY_ROOT,
        }
    }

    /// The (domain-hashed) leaf at `i`.
    pub fn leaf(&self, i: usize) -> Option<u64> {
        self.levels.first().and_then(|l| l.get(i)).copied()
    }

    /// Merkle proof for leaf `i` — sibling hashes bottom to top.
    pub fn proof(&self, i: usize) -> Option<Vec<Step>> {
        if i >= self.n {
            return None;
        }
        let mut out = Vec::new();
        let mut idx = i;
        for level in &self.levels {
            if level.len() <= 1 {
                break;
            }
            let sibling = idx ^ 1;
            if sibling < level.len() {
                out.push(Step {
                    hash: level[sibling],
                    right: sibling > idx,
                });
            }
            // No sibling → the node was promoted; nothing to absorb.
            idx /= 2;
        }
        Some(out)
    }

    /// Verify `leaf` (raw value) against `proof` and `root`.
    pub fn verify(leaf: u64, proof: &[Step], root: u64) -> bool {
        let mut acc = hash_leaf(leaf);
        for s in proof {
            acc = if s.right {
                hash_node(acc, s.hash)
            } else {
                hash_node(s.hash, acc)
            };
        }
        acc == root
    }

    /// First leaf index whose hash differs from `other`'s — `None`
    /// when both trees are identical. When lengths differ the answer
    /// is `min(len)` (the first structurally absent leaf). O(log n)
    /// node comparisons when lengths match.
    pub fn first_diff(&self, other: &Merkle) -> Option<usize> {
        if self.n != other.n {
            return Some(self.n.min(other.n));
        }
        if self.root() == other.root() {
            return None;
        }
        // Equal n ⇒ identical level shapes, so node indices align
        // across the two trees. Descend: prefer the left child when it
        // differs, else the right — the leftmost differing leaf.
        let mut i = 0usize;
        for level in (1..self.levels.len()).rev() {
            let (a, b) = (&self.levels[level - 1], &other.levels[level - 1]);
            let (li, ri) = (2 * i, 2 * i + 1);
            i = if li < a.len() && a[li] != b[li] {
                li
            } else if ri < a.len() && a[ri] != b[ri] {
                ri
            } else {
                return None; // unreachable when roots differ
            };
        }
        Some(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn naive_diff(a: &[u64], b: &[u64]) -> Option<usize> {
        if a.len() != b.len() {
            return Some(a.len().min(b.len()));
        }
        (0..a.len()).find(|&i| a[i] != b[i])
    }

    #[test]
    fn proofs_verify_and_first_diff_matches_naive() {
        let mut rng = SplitMix64::new(0x1EAF);
        for _ in 0..300 {
            let n = rng.below(64) as usize;
            let leaves: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
            let a = Merkle::build(&leaves);
            // Proof round-trip on every leaf.
            for (i, &lv) in leaves.iter().enumerate() {
                let p = a.proof(i).unwrap();
                assert!(Merkle::verify(lv, &p, a.root()), "proof {i}/{n}");
            }
            // Mutate one leaf → diff must equal naive first mismatch.
            if n > 0 {
                let mut leaves2 = leaves.clone();
                let idx = rng.below(n as u32) as usize;
                leaves2[idx] = rng.next_u64() | 1;
                if leaves2[idx] == leaves[idx] {
                    leaves2[idx] = leaves[idx].wrapping_add(1);
                }
                let b = Merkle::build(&leaves2);
                assert_eq!(a.first_diff(&b), naive_diff(&leaves, &leaves2));
                // And the proof of a foreign leaf under a's root fails.
                let p = a.proof(idx).unwrap();
                assert!(!Merkle::verify(leaves2[idx], &p, a.root()));
            }
            // Length difference → structural diff index.
            let short = &leaves[..n.saturating_sub(1)];
            let b = Merkle::build(short);
            assert_eq!(a.first_diff(&b), naive_diff(&leaves, short));
        }
        // Identical trees → no diff; empty trees.
        let x = Merkle::build(&[7, 7, 7]);
        let y = Merkle::build(&[7, 7, 7]);
        assert_eq!(x.first_diff(&y), None);
        assert_eq!(x.root(), y.root());
        let e = Merkle::build(&[]);
        assert_eq!(e.root(), EMPTY_ROOT);
        assert!(e.is_empty());
        assert_eq!(e.proof(0), None);
        // Nothing proves membership in an empty tree.
        assert!(!Merkle::verify(0, &[], EMPTY_ROOT));
    }

    #[test]
    fn root_is_a_pure_function_of_leaves() {
        let a = Merkle::build(&[1, 2, 3, 4, 5, 6, 7]);
        let b = Merkle::build(&[1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(a.root(), b.root());
        // Order matters: swapped leaves change the root.
        let c = Merkle::build(&[1, 2, 3, 4, 5, 7, 6]);
        assert_ne!(a.root(), c.root());
    }

    #[test]
    fn deep_first_diff_uses_log_levels() {
        let n = 1024;
        let leaves: Vec<u64> = (0..n).map(|i| i as u64 * 31).collect();
        let a = Merkle::build(&leaves);
        for idx in [0, 1, 511, 512, 1023] {
            let mut leaves2 = leaves.clone();
            leaves2[idx] = u64::MAX - idx as u64;
            let b = Merkle::build(&leaves2);
            assert_eq!(a.first_diff(&b), Some(idx));
        }
    }
}
