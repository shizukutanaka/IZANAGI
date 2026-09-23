//! Tower of Hanoi — the optimal `2ⁿ−1` move sequence on
//! three pegs, produced both in bulk (classic recursion)
//! and pointwise (`move_at`, `O(n)` per query via the
//! recursive split point `k = 2ⁿ⁻¹−1`), plus `state` for
//! the peg assignment after `k` moves.
//!
//! Pegs are `u8` labels `0..3`; `moves(n, from, to, aux)`
//! moves the whole `n`-disk stack `from → to` via `aux`.
//!
//! ```
//! use izanagi_kit::hanoi::{moves, move_at, state};
//! // n=3 needs 7 moves; the middle one shifts the big disk
//! assert_eq!(moves(3, 0, 2, 1).len(), 7);
//! assert_eq!(move_at(3, 0, 2, 1, 3), Some((2, 0, 2)));
//! assert_eq!(state(3, 0, 2, 1, 7), vec![2, 2, 2]);
//! ```
//!
//! References: the recursive solution is classical
//! (Lucas 1883); the `k = 2ⁿ⁻¹−1` split used by
//! `move_at` is the standard structure of the optimal
//! solution — move `2ⁿ⁻¹−1` always transfers the largest
//! disk, and each side is a smaller Hanoi instance.

/// The full optimal move list: `(disk, from_peg, to_peg)`
/// with `disk` 0-indexed (0 = smallest). `2ⁿ−1` entries —
/// keep `n` modest; `n = 0` gives `[]`, `n ≥ 32` gives
/// `None`-sized output: this function returns `Vec`, so
/// `n` above ~28 is impractical by construction.
pub fn moves(n: u32, from: u8, to: u8, aux: u8) -> Vec<(u32, u8, u8)> {
    let mut out = Vec::new();
    fn rec(n: u32, f: u8, t: u8, a: u8, out: &mut Vec<(u32, u8, u8)>) {
        if n == 0 {
            return;
        }
        rec(n - 1, f, a, t, out);
        out.push((n - 1, f, t));
        rec(n - 1, a, t, f, out);
    }
    rec(n, from, to, aux, &mut out);
    out
}

/// The `k`-th move (0-indexed) of the optimal sequence —
/// `O(n)` descent through the recursive split:
/// move `2ⁿ⁻¹−1` is the largest disk going `from → to`,
/// the left half is the `(n−1)`-problem `from → aux`,
/// the right half `aux → to`. `None` for `k ≥ 2ⁿ−1` or
/// `n ≥ 32`.
pub fn move_at(n: u32, from: u8, to: u8, aux: u8, mut k: u64) -> Option<(u32, u8, u8)> {
    if n == 0 || n >= 32 || k >= (1u64 << n) - 1 {
        return None;
    }
    let (mut f, mut t, mut a) = (from, to, aux);
    let mut disks = n;
    loop {
        let mid = (1u64 << (disks - 1)) - 1;
        if k == mid {
            return Some((disks - 1, f, t));
        }
        if k < mid {
            // left subproblem: from → aux
            std::mem::swap(&mut t, &mut a);
        } else {
            // right subproblem: aux → to
            k -= mid + 1;
            std::mem::swap(&mut f, &mut a);
        }
        disks -= 1;
    }
}

/// The peg of every disk after `k` moves — `state[d]` is
/// the peg holding disk `d`. Replays `moves` — `O(2ⁿ)`,
/// keep `n ≤ 20`.
pub fn state(n: u32, from: u8, to: u8, aux: u8, k: u64) -> Vec<u8> {
    let mut peg = vec![from; n as usize];
    for &(d, f, t) in moves(n, from, to, aux).iter().take(k as usize) {
        peg[d as usize] = t;
        let _ = f;
    }
    peg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let mv = moves(3, 0, 2, 1);
        assert_eq!(mv.len(), 7);
        assert_eq!(mv[3], (2, 0, 2)); // largest disk crosses mid-sequence
        assert_eq!(move_at(3, 0, 2, 1, 3), Some((2, 0, 2)));
        assert_eq!(move_at(3, 0, 2, 1, 7), None);
        assert_eq!(move_at(0, 0, 2, 1, 0), None);
        assert_eq!(state(3, 0, 2, 1, 7), vec![2, 2, 2]);
        assert_eq!(state(3, 0, 2, 1, 0), vec![0, 0, 0]);
        assert_eq!(moves(0, 0, 2, 1), Vec::<(u32, u8, u8)>::new());
    }

    /// Full simulation oracle: every move is legal (the
    /// moved disk is on top of its source and smaller than
    /// anything on the destination), and the end state has
    /// the whole stack on `to`.
    #[test]
    fn legality_oracle() {
        let mut rng = SplitMix64::new(0x4A01);
        for _ in 0..60 {
            let n = 1 + rng.below(10);
            let mv = moves(n, 0, 2, 1);
            assert_eq!(mv.len() as u64, (1u64 << n) - 1);
            // stacks[peg] = bottom→top
            let mut stacks: Vec<Vec<u32>> = vec![(0..n).rev().collect(), vec![], vec![]];
            for &(d, f, t) in &mv {
                let src = &mut stacks[f as usize];
                assert_eq!(src.last(), Some(&d), "disk {d} not on top of peg {f}");
                src.pop();
                let dst = &mut stacks[t as usize];
                assert!(
                    dst.last().map_or(true, |&top| d < top),
                    "disk {d} onto smaller"
                );
                dst.push(d);
            }
            assert_eq!(stacks[2].len(), n as usize);
            assert!(stacks[0].is_empty() && stacks[1].is_empty());
            // move_at matches the materialized sequence
            for k in 0..mv.len() as u64 {
                assert_eq!(move_at(n, 0, 2, 1, k), Some(mv[k as usize]));
            }
            // state(n,k) equals replaying the first k moves
            for _ in 0..6 {
                let k = u64::from(rng.below(mv.len() as u32));
                let mut peg = vec![0u8; n as usize];
                for &(d, _f, t) in mv.iter().take(k as usize) {
                    peg[d as usize] = t;
                }
                assert_eq!(state(n, 0, 2, 1, k), peg);
            }
        }
    }

    /// The `ctz` characterization: move `k` always moves
    /// disk `ctz(k+1)` — the classic bitwise
    /// characterization of the optimal sequence.
    #[test]
    fn ctz_disk_oracle() {
        for n in 1..12u32 {
            for k in 0..(1u64 << n) - 1 {
                let (d, _, _) = move_at(n, 0, 2, 1, k).unwrap();
                assert_eq!(d, (k + 1).trailing_zeros());
            }
        }
    }
}
