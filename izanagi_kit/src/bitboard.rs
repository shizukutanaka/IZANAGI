//! 8×8 bitboards — a `u64` per piece set with bit `sq` naming the
//! square `rank*8 + file` (`a1 = 0`, `h8 = 63`). Shifts masked by
//! the wrap-around files give every direction in one instruction,
//! and sliding attacks come from a dumb7fill occluded fill: the
//! integer dataflow is a pure function of `(set, occupancy)` — the
//! same position always produces the same attack map.
//!
//! ```
//! use izanagi_kit::bitboard as bb;
//!
//! let knights = bb::sq(1, 0) | bb::sq(6, 0); // b1, g1
//! let attacks = bb::knight_attacks(knights);
//! assert!(attacks & bb::sq(2, 2) != 0); // c3
//! let rooks = bb::sq(0, 0); // a1
//! let occ = rooks | bb::sq(0, 2); // blocker on a3
//! // North ray stops at (and captures) the a3 blocker; the a-file
//! // squares beyond stay unattacked, while rank 1 is fully open.
//! assert_eq!(
//!     bb::rook_attacks(rooks, occ),
//!     (bb::RANKS[0] & !rooks) | bb::sq(0, 1) | bb::sq(0, 2)
//! );
//! ```
//!
//! References: Chess Programming Wiki — bitboards, dumb7fill,
//! kindergarten-style attack generation.

/// Bit for the square `file + 8*rank`; both are 0-based (`a1`=0,0).
pub fn sq(file: u32, rank: u32) -> u64 {
    1u64 << (rank * 8 + file)
}

/// File masks `A..=H`.
pub const FILES: [u64; 8] = [
    0x0101_0101_0101_0101,
    0x0202_0202_0202_0202,
    0x0404_0404_0404_0404,
    0x0808_0808_0808_0808,
    0x1010_1010_1010_1010,
    0x2020_2020_2020_2020,
    0x4040_4040_4040_4040,
    0x8080_8080_8080_8080,
];
/// Rank masks `1..=8`.
pub const RANKS: [u64; 8] = [
    0x0000_0000_0000_00ff,
    0x0000_0000_0000_ff00,
    0x0000_0000_00ff_0000,
    0x0000_0000_ff00_0000,
    0x0000_00ff_0000_0000,
    0x0000_ff00_0000_0000,
    0x00ff_0000_0000_0000,
    0xff00_0000_0000_0000,
];

const NOT_A: u64 = !FILES[0];
const NOT_H: u64 = !FILES[7];
const NOT_AB: u64 = !(FILES[0] | FILES[1]);
const NOT_GH: u64 = !(FILES[6] | FILES[7]);

/// Number of set bits.
pub fn popcnt(bb: u64) -> u32 {
    bb.count_ones()
}

/// Index of the lowest set bit (`a1`=0), or `None` when empty.
pub fn lsb(bb: u64) -> Option<u32> {
    if bb == 0 {
        None
    } else {
        Some(bb.trailing_zeros())
    }
}

/// Set bits as square indices in ascending order.
pub fn squares(bb: u64) -> Vec<u32> {
    let mut v = Vec::with_capacity(bb.count_ones() as usize);
    let mut b = bb;
    while b != 0 {
        let s = b.trailing_zeros();
        v.push(s);
        b &= b - 1;
    }
    v
}

/// One step north (`+rank`); the top rank falls off.
pub fn north(bb: u64) -> u64 {
    bb << 8
}
/// One step south.
pub fn south(bb: u64) -> u64 {
    bb >> 8
}
/// One step east (`+file`); file H wraps off.
pub fn east(bb: u64) -> u64 {
    (bb & NOT_H) << 1
}
/// One step west.
pub fn west(bb: u64) -> u64 {
    (bb & NOT_A) >> 1
}
/// One step north-east.
pub fn north_east(bb: u64) -> u64 {
    (bb & NOT_H) << 9
}
/// One step north-west.
pub fn north_west(bb: u64) -> u64 {
    (bb & NOT_A) << 7
}
/// One step south-east.
pub fn south_east(bb: u64) -> u64 {
    (bb & NOT_H) >> 7
}
/// One step south-west.
pub fn south_west(bb: u64) -> u64 {
    (bb & NOT_A) >> 9
}

/// Squares a knight on any set square attacks.
pub fn knight_attacks(bb: u64) -> u64 {
    let a = (bb & NOT_GH) << 10 | (bb & NOT_AB) << 6; // +2f ±1r ... via file-clamped shifts
    let b = (bb & NOT_GH) >> 6 | (bb & NOT_AB) >> 10;
    let c = (bb & NOT_H) << 17 | (bb & NOT_A) << 15;
    let d = (bb & NOT_H) >> 15 | (bb & NOT_A) >> 17;
    a | b | c | d
}

/// Squares a king on any set square attacks (one step, all dirs).
pub fn king_attacks(bb: u64) -> u64 {
    let e = east(bb);
    let w = west(bb);
    e | w | north(bb | e | w) | south(bb | e | w)
}

/// Squares white pawns on set squares attack (north-east/north-west);
/// pass `false` for black's south-going attacks.
pub fn pawn_attacks(bb: u64, white: bool) -> u64 {
    if white {
        north_east(bb) | north_west(bb)
    } else {
        south_east(bb) | south_west(bb)
    }
}

/// Occluded fill along `dir` starting from `gen`, blocked by
/// `blockers`: `gen` propagates through empty squares only, and the
/// final shift lands on the first blocker (capture) or off-board.
fn fill(mut gen: u64, blockers: u64, dir: fn(u64) -> u64) -> u64 {
    let empty = !blockers;
    let mut flood = gen;
    for _ in 0..6 {
        gen = dir(gen) & empty;
        flood |= gen;
    }
    dir(flood)
}

/// Sliding attacks along one direction from every set square.
fn ray_attacks(bb: u64, occ: u64, dir: fn(u64) -> u64) -> u64 {
    fill(bb, occ, dir)
}

/// Rook attacks from every set square, blocked by `occ`
/// (the first blocker along each ray is included as a capture).
pub fn rook_attacks(bb: u64, occ: u64) -> u64 {
    ray_attacks(bb, occ, north)
        | ray_attacks(bb, occ, south)
        | ray_attacks(bb, occ, east)
        | ray_attacks(bb, occ, west)
}

/// Bishop attacks from every set square, blocked by `occ`.
pub fn bishop_attacks(bb: u64, occ: u64) -> u64 {
    ray_attacks(bb, occ, north_east)
        | ray_attacks(bb, occ, north_west)
        | ray_attacks(bb, occ, south_east)
        | ray_attacks(bb, occ, south_west)
}

/// Queen attacks — rook and bishop rays combined.
pub fn queen_attacks(bb: u64, occ: u64) -> u64 {
    rook_attacks(bb, occ) | bishop_attacks(bb, occ)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq_idx(file: u32, rank: u32) -> u32 {
        rank * 8 + file
    }

    /// Slow ray walk from `s` in direction (df, dr); returns attacked
    /// squares including the first occupied blocker, stopping at it.
    fn walk(s: u32, occ: u64, df: i32, dr: i32) -> u64 {
        let mut out = 0u64;
        let (mut f, mut r) = ((s % 8) as i32, (s / 8) as i32);
        loop {
            f += df;
            r += dr;
            if !(0..8).contains(&f) || !(0..8).contains(&r) {
                break;
            }
            let bit = 1u64 << sq_idx(f as u32, r as u32);
            out |= bit;
            if occ & bit != 0 {
                break;
            }
        }
        out
    }

    /// Per-square attack oracle: union over sources of walked rays.
    fn oracle_attacks(bb: u64, occ: u64, dirs: &[(i32, i32)]) -> u64 {
        let mut out = 0u64;
        for s in squares(bb) {
            for &(df, dr) in dirs {
                out |= walk(s, occ, df, dr);
            }
        }
        out
    }

    /// Leaper oracle: a single step per direction, no ray.
    fn leaper_attacks(bb: u64, dirs: &[(i32, i32)]) -> u64 {
        let mut out = 0u64;
        for s in squares(bb) {
            let (f, r) = (s % 8, s / 8);
            for &(df, dr) in dirs {
                let (nf, nr) = (f as i32 + df, r as i32 + dr);
                if (0..8).contains(&nf) && (0..8).contains(&nr) {
                    out |= 1u64 << (nr * 8 + nf) as u32;
                }
            }
        }
        out
    }

    #[test]
    fn shifts_and_masks() {
        assert_eq!(north(sq(0, 0)), sq(0, 1));
        assert_eq!(south(sq(0, 1)), sq(0, 0));
        assert_eq!(east(sq(7, 0)), 0); // wraps off the board
        assert_eq!(west(sq(0, 0)), 0);
        assert_eq!(north_east(sq(0, 0)), sq(1, 1));
        assert_eq!(south_west(sq(1, 1)), sq(0, 0));
        assert_eq!(north_west(sq(7, 0)), sq(6, 1));
        assert_eq!(south_east(sq(6, 1)), sq(7, 0));
        assert_eq!(north_west(sq(0, 0)), 0); // wraps off the board
        assert_eq!(south_east(sq(7, 7)), 0);
        for f in 0..8 {
            assert_eq!(popcnt(FILES[f as usize]), 8);
            assert_eq!(popcnt(RANKS[f as usize]), 8);
        }
        assert_eq!(lsb(0), None);
        assert_eq!(lsb(sq(3, 2)), Some(sq_idx(3, 2)));
        assert_eq!(squares(sq(0, 0) | sq(7, 7)), vec![0, 63]);
    }

    #[test]
    fn knight_king_pawn_oracle() {
        let knight_dirs = [
            (1, 2),
            (2, 1),
            (-1, 2),
            (-2, 1),
            (1, -2),
            (2, -1),
            (-1, -2),
            (-2, -1),
        ];
        let king_dirs = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ];
        for f in 0..8 {
            for r in 0..8 {
                let b = sq(f, r);
                assert_eq!(knight_attacks(b), leaper_attacks(b, &knight_dirs));
                assert_eq!(king_attacks(b), leaper_attacks(b, &king_dirs));
            }
        }
        assert_eq!(pawn_attacks(sq(3, 3), true), sq(2, 4) | sq(4, 4));
        assert_eq!(pawn_attacks(sq(3, 3), false), sq(2, 2) | sq(4, 2));
        assert_eq!(pawn_attacks(sq(0, 0), true), sq(1, 1));
        assert_eq!(pawn_attacks(sq(7, 7), false), sq(6, 6));
    }

    #[test]
    fn sliding_oracle() {
        use crate::rng::SplitMix64;
        let rook_dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        let bishop_dirs = [(1, 1), (-1, 1), (1, -1), (-1, -1)];
        let mut rng = SplitMix64::new(0xB17B_0AED);
        for _ in 0..4000 {
            let b = rng.next_u64();
            let occ = rng.next_u64();
            assert_eq!(rook_attacks(b, occ), oracle_attacks(b, occ, &rook_dirs));
            assert_eq!(bishop_attacks(b, occ), oracle_attacks(b, occ, &bishop_dirs));
            assert_eq!(
                queen_attacks(b, occ),
                rook_attacks(b, occ) | bishop_attacks(b, occ)
            );
        }
    }

    #[test]
    fn edge_of_board() {
        // Corner rook with no blockers: full rank+file.
        let a1 = sq(0, 0);
        assert_eq!(rook_attacks(a1, a1), (RANKS[0] | FILES[0]) & !a1);
        // Attacks include own-occupied adjacent squares as captures.
        let occ = a1 | sq(0, 1) | sq(1, 0);
        assert_eq!(rook_attacks(a1, occ), sq(0, 1) | sq(1, 0));
        // Bishop through a diagonal blocker.
        let occ2 = sq(0, 0) | sq(3, 3);
        let expect = sq(1, 1) | sq(2, 2) | sq(3, 3);
        assert_eq!(bishop_attacks(sq(0, 0), occ2), expect);
    }
}
