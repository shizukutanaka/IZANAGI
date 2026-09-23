//! Magic bitboards — multiply-and-shift perfect hashing of
//! sliding-piece attack sets. `bitboard::rook_attacks`/`bishop_attacks`
//! already answer `(square, occupancy)` queries in `O(ray)`; magic
//! bitboards compress each square's relevant occupancy to a dense
//! table index, so attacks are one lookup.
//!
//! For each square the "relevant" occupancy is the ray squares
//! *excluding* the board edge (edge squares are never blockers).
//! A per-square magic `m` and shift `s` are chosen so that
//! `(occ & mask) * m >> s` injects every relevant occupancy into
//! a table of `1 << relevant_bits` entries — the search is seeded
//! with `SplitMix64`, so the tables are a pure function of the
//! seed, not of platform or run order.
//!
//! ```
//! use izanagi_kit::magic::{MagicTables, Piece};
//! let t = MagicTables::new(0xC0FFEE);
//! // rook on e4 with blockers on e7 and c4
//! let occ = izanagi_kit::bitboard::sq(4, 6) | izanagi_kit::bitboard::sq(2, 3);
//! let sq = izanagi_kit::bitboard::sq(4, 3);
//! let atk = t.attacks(Piece::Rook, 4 + 3 * 8, occ);
//! assert_eq!(atk, izanagi_kit::bitboard::rook_attacks(sq, occ));
//! ```
//!
//! References: Kannan (2007) magic-move-bitboards; Tord Romstad's
//! original CCC post introducing the multiply-shift occupancy
//! index; Pradyumna Kannan's generator distributed with the
//! chessprogramming wiki article.

use crate::bitboard;
use crate::rng::SplitMix64;

/// Sliding piece kind for `attacks`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Rook — orthogonal sliders.
    Rook,
    /// Bishop — diagonal sliders.
    Bishop,
}

/// Relevant-occupancy mask for a slider on `sq`: every ray
/// square EXCEPT each ray's terminal square — a blocker on the
/// terminal changes nothing about the attacks, so it is not
/// relevant. For direction `dir` with opposite `opp`, the ray
/// interior is `ray & opp(ray)`: every ray square whose
/// predecessor is also a ray square.
type DirPair = (fn(u64) -> u64, fn(u64) -> u64);

fn relevant_mask(sq: u64, piece: Piece) -> u64 {
    let dirs: &[DirPair] = match piece {
        Piece::Rook => &[
            (bitboard::north, bitboard::south),
            (bitboard::south, bitboard::north),
            (bitboard::east, bitboard::west),
            (bitboard::west, bitboard::east),
        ],
        Piece::Bishop => &[
            (bitboard::north_east, bitboard::south_west),
            (bitboard::south_west, bitboard::north_east),
            (bitboard::north_west, bitboard::south_east),
            (bitboard::south_east, bitboard::north_west),
        ],
    };
    let mut m = 0u64;
    for &(dir, opp) in dirs {
        // full ray from sq to the board edge (source excluded)
        let mut ray = 0u64;
        let mut g = dir(sq);
        while g != 0 {
            ray |= g;
            g = dir(g);
        }
        m |= ray & opp(ray);
    }
    m
}

/// Enumerate every subset of `mask` (Carry-Rippler trick).
fn subsets(mask: u64) -> Vec<u64> {
    let mut v = Vec::with_capacity(1usize << mask.count_ones());
    let mut s = mask;
    loop {
        v.push(s);
        if s == 0 {
            break;
        }
        s = s.wrapping_sub(1) & mask;
    }
    v
}

fn attack_of(sq: u64, occ: u64, piece: Piece) -> u64 {
    match piece {
        Piece::Rook => bitboard::rook_attacks(sq, occ),
        Piece::Bishop => bitboard::bishop_attacks(sq, occ),
    }
}

/// One square's magic entry: the perfect `magic`, `shift`, and
/// the attack table it indexes into (`tables[offset..]`).
#[derive(Clone)]
struct Entry {
    magic: u64,
    shift: u32,
    offset: usize,
    len: usize,
    /// fallback path: skip the table, call the dumb7fill oracle
    /// (unreached in practice — the seeded search always lands)
    brute: bool,
}

/// Per-square magic tables for both slider kinds. `new(seed)`
/// runs the deterministic magic search; the same seed always
/// builds identical tables.
pub struct MagicTables {
    rook: [Entry; 64],
    bishop: [Entry; 64],
    rook_tables: Vec<u64>,
    bishop_tables: Vec<u64>,
}

impl MagicTables {
    /// Build both tables. The search tries seeded random `u64`s;
    /// for every board square a magic exists in expectation within
    /// a few thousand draws, so the bounded try loop never
    /// exhausts in practice — if it did, the entry falls back to
    /// the dumb7fill oracle at query time rather than a wrong
    /// table value.
    pub fn new(seed: u64) -> Self {
        let mut rook_tables = Vec::new();
        let mut bishop_tables = Vec::new();
        let rook = Self::build(seed, Piece::Rook, &mut rook_tables);
        let bishop = Self::build(seed ^ 0xFF, Piece::Bishop, &mut bishop_tables);
        MagicTables {
            rook,
            bishop,
            rook_tables,
            bishop_tables,
        }
    }

    fn build(seed: u64, piece: Piece, tables: &mut Vec<u64>) -> [Entry; 64] {
        let mut r = SplitMix64::new(seed);
        std::array::from_fn(|i| Self::build_square(i as u32, piece, &mut r, tables))
    }

    fn build_square(i: u32, piece: Piece, r: &mut SplitMix64, tables: &mut Vec<u64>) -> Entry {
        let sq = bitboard::sq(i % 8, i / 8);
        let mask = relevant_mask(sq, piece);
        let bits = mask.count_ones();
        let shift = 64 - bits;
        let occs = subsets(mask);
        let attacks: Vec<u64> = occs.iter().map(|&o| attack_of(sq, o, piece)).collect();
        let len = occs.len();
        let offset = tables.len();
        for _ in 0..2_000_000u32 {
            let magic = r.next_u64() & r.next_u64() & r.next_u64();
            let mut seen = vec![false; len];
            let mut ok = true;
            for &o in &occs {
                let idx = (o.wrapping_mul(magic) >> shift) as usize;
                if idx < len {
                    if seen[idx] {
                        ok = false;
                        break;
                    }
                    seen[idx] = true;
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                // scatter each attack into its hashed slot —
                // index i is the subset enumeration order, the
                // query index is (o*magic)>>shift
                let start = tables.len();
                tables.resize(start + len, 0);
                for (i, &o) in occs.iter().enumerate() {
                    let idx = (o.wrapping_mul(magic) >> shift) as usize;
                    tables[start + idx] = attacks[i];
                }
                return Entry {
                    magic,
                    shift,
                    offset,
                    len,
                    brute: false,
                };
            }
        }
        // never reached in practice; the honest fallback is the
        // dumb7fill oracle at query time, not a wrong table
        tables.extend(std::iter::repeat(attack_of(sq, 0, piece)).take(len.max(1)));
        Entry {
            magic: 0,
            shift: 0,
            offset,
            len: len.max(1),
            brute: true,
        }
    }

    /// Attack set for `piece` standing on `sq_idx` (0 = a1) with
    /// full occupancy `occ`.
    pub fn attacks(&self, piece: Piece, sq_idx: u32, occ: u64) -> u64 {
        let (entries, tables) = match piece {
            Piece::Rook => (&self.rook, &self.rook_tables),
            Piece::Bishop => (&self.bishop, &self.bishop_tables),
        };
        let sq_idx = sq_idx.min(63) as usize;
        let e = &entries[sq_idx];
        let sq = bitboard::sq(sq_idx as u32 % 8, sq_idx as u32 / 8);
        let m = relevant_mask(sq, piece);
        if e.brute {
            return attack_of(sq, occ & m, piece);
        }
        let idx = ((occ & m).wrapping_mul(e.magic) >> e.shift) as usize;
        if idx < e.len {
            tables[e.offset + idx]
        } else {
            0
        }
    }

    /// Queen attacks = rook table ∪ bishop table.
    pub fn queen_attacks(&self, sq_idx: u32, occ: u64) -> u64 {
        self.attacks(Piece::Rook, sq_idx, occ) | self.attacks(Piece::Bishop, sq_idx, occ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_dumb7fill_everywhere() {
        let t = MagicTables::new(0x9E37_79B9);
        let mut r = SplitMix64::new(0x000A_661C);
        for sq in 0..64u32 {
            // empty board + ~120 random occupancies per square
            for trial in 0..120u32 {
                let occ = if trial == 0 { 0 } else { r.next_u64() };
                assert_eq!(
                    t.attacks(Piece::Rook, sq, occ),
                    bitboard::rook_attacks(bitboard::sq(sq % 8, sq / 8), occ),
                    "rook sq={sq} occ={occ:x}"
                );
                assert_eq!(
                    t.attacks(Piece::Bishop, sq, occ),
                    bitboard::bishop_attacks(bitboard::sq(sq % 8, sq / 8), occ),
                    "bishop sq={sq}"
                );
                assert_eq!(
                    t.queen_attacks(sq, occ),
                    bitboard::queen_attacks(bitboard::sq(sq % 8, sq / 8), occ),
                );
            }
        }
    }

    #[test]
    fn deterministic_tables() {
        let a = MagicTables::new(7);
        let b = MagicTables::new(7);
        assert_eq!(a.rook_tables, b.rook_tables);
        assert_eq!(a.bishop_tables, b.bishop_tables);
        for i in 0..64 {
            assert_eq!(a.rook[i].magic, b.rook[i].magic);
        }
    }

    #[test]
    fn occupancy_outside_mask_is_ignored() {
        let t = MagicTables::new(1);
        // bit 63 is an edge square: excluded from every mask
        let sq = 28u32; // e4
        let a = t.attacks(Piece::Rook, sq, 0);
        let b = t.attacks(Piece::Rook, sq, 1u64 << 63);
        assert_eq!(a, b);
    }
}
