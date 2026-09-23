//! Sprague–Grundy theory for finite impartial games: `mex`,
//! Grundy numbers for subtraction (take-away) games, disjunctive
//! sums, and move search — all integer, all deterministic.
//!
//! A position's Grundy number is `mex{ g(next) | next reachable }`;
//! a position is losing (P-position) iff its Grundy is `0`. In a
//! disjunctive sum of games the Grundy numbers combine by XOR —
//! `nim_sum` — so a position of heaps is losing iff the xor of
//! every component's Grundy is `0`.
//!
//! ```
//! use izanagi_kit::grundy::{mex, take_away, nim_sum, winning_move};
//! assert_eq!(mex(&[0, 1, 3]), 2);
//! // heap game: a move removes 1, 2, or 3 stones
//! let g = take_away(&[1, 2, 3], 10);
//! assert_eq!(g[5], 1); // period-4 cycle: 5 mod 4 = 1
//! assert_eq!(nim_sum(&[g[7], g[3]]), 0); // losing
//! assert_eq!(winning_move(&[1, 2, 3], &g, 5), Some(1));
//! ```

/// Minimum excludant of a set of non-negative integers: the
/// least `g ≥ 0` not present. `mex(∅) = 0`.
pub fn mex(set: &[u64]) -> u64 {
    let mut present = std::collections::BTreeSet::new();
    for &g in set {
        present.insert(g);
    }
    let mut g = 0u64;
    while present.contains(&g) {
        g += 1;
    }
    g
}

/// Grundy numbers `g[0..=n]` for the subtraction game with move
/// set `moves` (each move removes `m` stones, `m ≤ heap`). Moves
/// are canonicalized to a sorted deduped list, so the table is a
/// pure function of `(moves, n)`.
pub fn take_away(moves: &[u64], n: usize) -> Vec<u64> {
    let mut mv: Vec<u64> = moves.iter().copied().filter(|&m| m > 0).collect();
    mv.sort_unstable();
    mv.dedup();
    let mut g = vec![0u64; n + 1];
    for heap in 1..=n {
        let mut opts: Vec<u64> = Vec::new();
        for &m in &mv {
            if m as usize <= heap {
                opts.push(g[heap - m as usize]);
            } else {
                break;
            }
        }
        g[heap] = mex(&opts);
    }
    g
}

/// Grundy table for a *take-from-any-one-heap* multiset game:
/// same as `take_away` — kept separate for readability.
/// `heaps` is a list of pile sizes; the position loses iff
/// `nim_sum(g[heap_i]) == 0`.
pub fn position_grundy(moves: &[u64], heaps: &[u64]) -> u64 {
    let Some(&max) = heaps.iter().max() else {
        return 0;
    };
    let g = take_away(moves, max as usize);
    nim_sum(&heaps.iter().map(|&h| g[h as usize]).collect::<Vec<_>>())
}

/// Nim-sum: xor of Grundy numbers — the disjunctive-sum rule.
pub fn nim_sum(grundies: &[u64]) -> u64 {
    grundies.iter().fold(0u64, |a, &g| a ^ g)
}

/// A move leading to a P-position: the least `m` (sorted order)
/// such that `g[heap − m]` is `0`, or `None` when losing.
pub fn winning_move(moves: &[u64], g: &[u64], heap: usize) -> Option<u64> {
    if heap >= g.len() {
        return None;
    }
    let mut mv: Vec<u64> = moves.iter().copied().filter(|&m| m > 0).collect();
    mv.sort_unstable();
    mv.dedup();
    mv.iter()
        .find(|&&m| m as usize <= heap && g[heap - m as usize] == 0)
        .copied()
}

/// `true` iff the single-heap position is a P-position.
pub fn losing(moves: &[u64], heap: usize) -> bool {
    take_away(moves, heap)[heap] == 0
}

/// Period detection on a Grundy table: finds the least
/// `(start, period)` such that `g[i] = g[i − period]` holds for
/// every `i ≥ start + period` inside the table — AND, crucially,
/// the verified periodic stretch is long enough to *guarantee*
/// continuation: when the last `memory` positions all satisfy
/// `g[i] = g[i−p]`, any recurrence whose lookback is ≤ `memory`
/// (e.g. `max(moves)` for `take_away`) provably repeats forever
/// — the mex of identical inputs is identical. Pass
/// `memory = max(moves)`; otherwise the reported period is only
/// a tail coincidence and extending the table can break it.
pub fn detect_period(g: &[u64], memory: usize) -> Option<(usize, usize)> {
    let n = g.len();
    for p in 1..=n / 2 {
        // last index where p-periodicity is violated
        let mut bad: Option<usize> = None;
        for i in p..n {
            if g[i] != g[i - p] {
                bad = Some(i);
            }
        }
        let s = match bad {
            None => 0,
            Some(i) => i + 1 - p,
        };
        // the verified periodic run is n − s − p long; it must
        // cover a full recurrence memory to be a theorem, and
        // even without memory needs ≥1 verified pair
        if n >= s + p + memory.max(1) {
            return Some((s, p));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force outcome oracle: a heap loses iff every legal
    /// move reaches a winning position.
    fn losing_brute(moves: &[u64], heap: u64) -> bool {
        let mut memo: Vec<i8> = vec![-1; heap as usize + 1]; // -1 unknown, 0 lose, 1 win
        fn go(moves: &[u64], h: usize, memo: &mut [i8]) -> i8 {
            if memo[h] >= 0 {
                return memo[h];
            }
            let mut r = 0i8;
            for &m in moves {
                if m > 0 && m as usize <= h && go(moves, h - m as usize, memo) == 0 {
                    r = 1;
                    break;
                }
            }
            memo[h] = r;
            r
        }
        go(moves, heap as usize, &mut memo) == 0
    }

    #[test]
    fn basics() {
        assert_eq!(mex(&[0, 1, 3]), 2);
        assert_eq!(mex(&[]), 0);
        assert_eq!(mex(&[1, 2]), 0);
        let g = take_away(&[1, 2, 3], 10);
        assert_eq!(g[0], 0);
        assert_eq!(g[4], 0); // period-4
        assert_eq!(g[5], 1);
        assert_eq!(nim_sum(&[3, 5]), 6);
        assert_eq!(winning_move(&[1, 2, 3], &g, 5), Some(1));
        assert_eq!(winning_move(&[1, 2, 3], &g, 4), None);
        assert_eq!(winning_move(&[1, 2, 3], &g, 99), None); // out of table
        assert!(losing(&[1, 2, 3], 8));
        assert_eq!(position_grundy(&[1, 2, 3], &[5, 5, 5]), 1);
        assert_eq!(position_grundy(&[1, 2, 3], &[]), 0);
        // {1,2} moves → period 3 starting at 0
        let g2 = take_away(&[1, 2], 30);
        assert_eq!(detect_period(&g2, 2), Some((0, 3)));
        // memory = 0 asks only for a non-vacuous tail repeat
        assert_eq!(detect_period(&g2, 0), Some((0, 3)));
    }

    #[test]
    fn oracle_win_lose() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..80 {
            let k = 1 + rng.below(4);
            let mut moves: Vec<u64> = (0..k).map(|_| 1 + rng.below(9) as u64).collect();
            moves.sort_unstable();
            moves.dedup();
            let n = 60 + rng.below(60) as usize;
            let g = take_away(&moves, n);
            for heap in 0..=n {
                assert_eq!(
                    g[heap] == 0,
                    losing_brute(&moves, heap as u64),
                    "moves={moves:?} heap={heap}"
                );
                // winning_move consistency
                if g[heap] != 0 {
                    let m = winning_move(&moves, &g, heap)
                        .unwrap_or_else(|| unreachable!("winning position lacks a move"));
                    assert_eq!(g[heap - m as usize], 0);
                } else {
                    assert_eq!(winning_move(&moves, &g, heap), None);
                }
            }
        }
    }

    #[test]
    fn oracle_disjunctive_sum() {
        // multi-heap positions: outcome = xor of component grundies —
        // oracle plays the full game on the heap vector directly
        fn multi_losing(moves: &[u64], heaps: &mut [u64]) -> bool {
            // memoize on sorted heap vector
            fn go(moves: &[u64], h: &mut Vec<u64>) -> bool {
                // true = losing for player to move
                for i in 0..h.len() {
                    for &m in moves {
                        if m <= h[i] {
                            h[i] -= m;
                            let child = go(moves, h);
                            h[i] += m;
                            if child {
                                return false;
                            }
                        }
                    }
                }
                true
            }
            let mut h = heaps.to_vec();
            h.sort_unstable();
            go(moves, &mut h)
        }
        let mut rng = SplitMix64::new(23);
        for _ in 0..60 {
            let moves = vec![1 + rng.below(3) as u64, 1 + rng.below(5) as u64];
            let heaps: Vec<u64> = (0..1 + rng.below(4))
                .map(|_| rng.below(12) as u64)
                .collect();
            let g_max = *heaps.iter().max().unwrap_or(&0);
            let table = take_away(&moves, g_max as usize);
            let xor = nim_sum(&heaps.iter().map(|&h| table[h as usize]).collect::<Vec<_>>());
            assert_eq!(
                xor == 0,
                multi_losing(&moves, &mut heaps.clone()),
                "moves={moves:?} heaps={heaps:?}"
            );
        }
    }

    #[test]
    fn period_of_subtraction_games() {
        // any finite subtraction game is eventually periodic — verify
        // the detector agrees the table's tail truly repeats
        let mut rng = SplitMix64::new(41);
        for _ in 0..40 {
            let k = 1 + rng.below(4);
            let mut moves: Vec<u64> = (0..k).map(|_| 1 + rng.below(8) as u64).collect();
            moves.sort_unstable();
            moves.dedup();
            let n = 400usize;
            let g = take_away(&moves, n);
            let mem = *moves.iter().max().unwrap_or(&0) as usize;
            if let Some((s, p)) = detect_period(&g, mem) {
                for i in s + p..n {
                    assert_eq!(g[i], g[i - p], "period {p} start {s} i {i}");
                }
                // prediction: the extended table keeps the cycle
                let g2 = take_away(&moves, n + 20);
                for i in s + p..n + 20 {
                    assert_eq!(g2[i], g2[i - p], "i {i}");
                }
            }
        }
    }
}
