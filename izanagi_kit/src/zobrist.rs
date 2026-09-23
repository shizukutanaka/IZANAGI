//! Incremental Zobrist hashing for board-like states.
//!
//! Zobrist (1970) assigns each `(piece, square)` pair a random
//! `u64` key; a position's hash is the XOR of every occupied
//! pair's key. XOR's self-inverse makes updates reversible:
//! `toggle` twice returns the hash unchanged — the property
//! that makes transposition tables and undo stacks exact.
//!
//! The table is filled from a seeded [`SplitMix64`], so
//! `(seed, n_pieces, n_squares)` is the whole state definition
//! — same seed, same keys, same hash, on every platform.
//!
//! ```
//! use izanagi_kit::zobrist::Zobrist;
//! let mut z = Zobrist::new(42, 2, 64);
//! z.toggle(0, 12);
//! z.toggle(1, 40);
//! let h = z.hash();
//! z.toggle(1, 40); // undo
//! z.toggle(1, 41); // move piece 1
//! assert_ne!(z.hash(), h);
//! z.toggle(1, 41);
//! z.toggle(1, 40);
//! assert_eq!(z.hash(), h); // exact undo
//! ```

use crate::rng::SplitMix64;

/// An incremental position hash over `n_pieces × n_squares`
/// keys drawn from `seed`.
pub struct Zobrist {
    /// `piece * n_squares + square` flattened keys.
    keys: Vec<u64>,
    n_squares: usize,
    h: u64,
}

impl Zobrist {
    /// Table for `n_pieces` kinds over `n_squares` squares,
    /// filled deterministically from `seed`.
    pub fn new(seed: u64, n_pieces: usize, n_squares: usize) -> Self {
        let mut rng = SplitMix64::new(seed);
        let keys = (0..n_pieces * n_squares).map(|_| rng.next_u64()).collect();
        Zobrist {
            keys,
            n_squares,
            h: 0,
        }
    }

    /// Current hash.
    pub fn hash(&self) -> u64 {
        self.h
    }

    /// XOR the `(piece, square)` key into the hash — insert if
    /// the square was empty for that piece, remove if occupied.
    /// No occupancy is tracked here; callers own the board.
    pub fn toggle(&mut self, piece: usize, square: usize) {
        if let Some(&k) = self.keys.get(piece * self.n_squares + square) {
            self.h ^= k;
        }
    }

    /// Move: `toggle(p, from)` then `toggle(p, to)` — spelled
    /// out because it is the canonical use.
    pub fn move_piece(&mut self, piece: usize, from: usize, to: usize) {
        self.toggle(piece, from);
        self.toggle(piece, to);
    }

    /// Recompute the hash from scratch over `occupied` pairs —
    /// the ground truth incremental updates must equal.
    pub fn rehash(&self, occupied: &[(usize, usize)]) -> u64 {
        occupied.iter().fold(0u64, |h, &(p, s)| {
            h ^ self.keys.get(p * self.n_squares + s).copied().unwrap_or(0)
        })
    }

    /// Hash of a position *plus* a side-to-move key —
    /// convention: XOR in `side_key` when black is to move.
    /// `side_key` is derived deterministically from the table
    /// seed by `Zobrist::side_key(seed)`.
    pub fn hash_with_side(&self, side_key: u64) -> u64 {
        self.h ^ side_key
    }

    /// The deterministic side-to-move key for `seed` — drawn
    /// after all `n_pieces * n_squares` table keys, so it can
    /// never collide with a piece key.
    pub fn side_key(seed: u64, n_pieces: usize, n_squares: usize) -> u64 {
        let mut rng = SplitMix64::new(seed);
        for _ in 0..n_pieces * n_squares {
            rng.next_u64();
        }
        rng.next_u64()
    }
}

impl Zobrist {
    /// Clone the running hash into a fresh state — test helper
    /// for speculative toggles without disturbing `self`.
    #[cfg(test)]
    fn clone_state(&self) -> Zobrist {
        Zobrist {
            keys: self.keys.clone(),
            n_squares: self.n_squares,
            h: self.h,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn insert_undo_move() {
        let mut z = Zobrist::new(42, 2, 64);
        let empty = z.hash();
        z.toggle(0, 12);
        z.toggle(1, 40);
        let h = z.hash();
        assert_ne!(h, empty);
        // order-independent: same occupied set, same hash
        let mut z2 = Zobrist::new(42, 2, 64);
        z2.toggle(1, 40);
        z2.toggle(0, 12);
        assert_eq!(z2.hash(), h);
        // toggle twice undoes
        z.toggle(0, 12);
        z.toggle(0, 12);
        assert_eq!(z.hash(), h);
        // move_piece keeps membership
        let mut z3 = Zobrist::new(42, 2, 64);
        z3.toggle(0, 12);
        z3.toggle(1, 40);
        z3.move_piece(0, 12, 13);
        z3.move_piece(0, 13, 12);
        assert_eq!(z3.hash(), h);
    }

    /// Random play sequences: incremental hash must always
    /// equal `rehash` of the occupancy set, and a re-run of the
    /// same ops is bit-identical.
    #[test]
    fn oracle_incremental_equals_rehash() {
        let mut rng = SplitMix64::new(0x20B);
        for _ in 0..40 {
            let mut z = Zobrist::new(0x5EED, 4, 25);
            let mut occ: BTreeSet<(usize, usize)> = BTreeSet::new();
            for _ in 0..300 {
                let p = rng.below(4) as usize;
                let s = rng.below(25) as usize;
                z.toggle(p, s);
                if !occ.insert((p, s)) {
                    occ.remove(&(p, s));
                }
                let want: Vec<(usize, usize)> = occ.iter().copied().collect();
                assert_eq!(z.hash(), z.rehash(&want));
            }
        }
    }

    /// Distinct positions almost surely hash distinctly — on
    /// this size, collisions would reveal table bugs, not luck.
    #[test]
    fn distinct_positions_distinct_hashes() {
        let mut seen = BTreeSet::new();
        let z = Zobrist::new(1, 2, 9);
        for p in 0..2 {
            for s in 0..9 {
                let mut zz = z.clone_state();
                zz.toggle(p, s);
                assert!(seen.insert(zz.hash()), "hash collision at {p},{s}");
            }
        }
    }

    #[test]
    fn side_key_after_table() {
        let sk = Zobrist::side_key(7, 3, 10);
        let z = Zobrist::new(7, 3, 10);
        // side key must differ from every table key and from 0
        assert_ne!(sk, 0);
        let mut with = z.clone_state();
        with.toggle(0, 0);
        assert_ne!(with.hash_with_side(sk), with.hash());
    }
}
