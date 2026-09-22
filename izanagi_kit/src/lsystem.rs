//! Lindenmayer systems (L-systems) — deterministic string rewriting for
//! procedural plants, veins, and branching structures, plus an integer
//! turtle that draws the expanded program onto a grid.
//!
//! An L-system rewrites an axiom by applying a production rule to every
//! symbol in parallel, `iters` times (Lindenmayer 1968; Prusinkiewicz &
//! Lindenmayer, *The Algorithmic Beauty of Plants*). The output is a pure
//! function of `(axiom, rules, iters)` — no RNG — so results replay
//! bit-identically. Stochastic variants exist in the literature; this one
//! is the deterministic context-free form (a `0L` system).
//!
//! [`turtle_cells`] interprets the program as a turtle over a caller-chosen
//! direction set: `F`/`G` draw forward, `f`/`g` move without drawing,
//! `+`/`-` rotate one direction step, `[`/`]` push/pop the turtle. A
//! `dirs` table of 8 vectors gives square-grid plants at 45°; 6 vectors
//! gives hex-grid branching. Drawing walks cells through [`crate::gridcast`]
//! so every entered cell is marked — a fat diagonal never skips a corner.
//!
//! ```
//! use izanagi_kit::lsystem::{expand, turtle_cells};
//! // Fibonacci bush: X → F[+X]F[-X]+X, F → FF.
//! let rules = [(b'X', b"F[+X]F[-X]+X".to_vec()), (b'F', b"FF".to_vec())];
//! let prog = expand(b"X", &rules, 3);
//! let cells = turtle_cells(&prog, &izanagi_kit::lsystem::DIRS_8, (0, 0), 1);
//! assert!(!cells.is_empty());
//! ```

/// The eight square-grid directions (E, NE, N, NW, W, SW, S, SE),
/// clockwise from east — the default `dirs` for [`turtle_cells`].
pub const DIRS_8: [(i32, i32); 8] = [
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// The four orthogonal directions — a tighter turn for Minkowski/Koch
/// curves. `+`/`-` step a quarter turn.
pub const DIRS_4: [(i32, i32); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];

/// The six axial hex directions (`hexgrid::DIRECTIONS` as `(q, r)` pairs)
/// for 60°-turn plants on a hex map.
pub const DIRS_HEX: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

/// Expand `axiom` through `rules` (a symbol → replacement slice mapping)
/// `iters` times, in parallel — every symbol rewritten once per iteration.
/// Symbols with no rule copy through unchanged. `rules` must be slice-
/// sorted by symbol for determinism (a `&[(u8, Vec<u8>)]` built in
/// ascending-symbol order — duplicate symbols are resolved to the first).
pub fn expand(axiom: &[u8], rules: &[(u8, Vec<u8>)], iters: u32) -> Vec<u8> {
    let mut cur = axiom.to_vec();
    for _ in 0..iters {
        let mut next = Vec::with_capacity(cur.len() * 2);
        for &s in &cur {
            match rules.iter().find(|(k, _)| *k == s) {
                Some((_, rep)) => next.extend_from_slice(rep),
                None => next.push(s),
            }
        }
        cur = next;
    }
    cur
}

/// Interpret a program string as a turtle walk and return the cells
/// drawn, in drawing order with duplicates kept (the caller can dedupe).
///
/// - `F`, `G`: move one `dirs` step in the current direction, drawing
///   every entered cell (via [`crate::gridcast::grid_ray`]).
/// - `f`, `g`: same move without drawing.
/// - `+`, `-`: rotate one `dirs` entry (wrap-around).
/// - `[`, `]`: push/pop the turtle state.
/// - Any other symbol: ignored (letters not in this set are usually the
///   rewriting variables).
///
/// `start` is the initial cell; `dir0` the initial `dirs` index.
pub fn turtle_cells(
    program: &[u8],
    dirs: &[(i32, i32)],
    start: (i32, i32),
    dir0: usize,
) -> Vec<(i32, i32)> {
    if dirs.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut stack: Vec<(i32, i32, usize)> = Vec::new();
    let (mut x, mut y) = start;
    let mut d = dir0 % dirs.len();
    let nd = dirs.len();
    for &sym in program {
        match sym {
            b'F' | b'G' | b'f' | b'g' => {
                let (dx, dy) = dirs[d];
                let (nx, ny) = (x + dx, y + dy);
                if sym == b'F' || sym == b'G' {
                    // Draw the step via grid DDA so diagonals never skip.
                    for &(cx, cy) in &crate::gridcast::grid_ray(x, y, nx, ny) {
                        out.push((cx, cy));
                    }
                }
                x = nx;
                y = ny;
            }
            b'+' => d = (d + 1) % nd,
            b'-' => d = (d + nd - 1) % nd,
            b'[' => stack.push((x, y, d)),
            b']' => {
                if let Some((px, py, pd)) = stack.pop() {
                    x = px;
                    y = py;
                    d = pd;
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algae_growth() {
        // The canonical L-system: a→ab, b→a — length = Fibonacci.
        let rules = [(b'a', b"ab".to_vec()), (b'b', b"a".to_vec())];
        let fib = |n: u32| -> usize {
            let mut len = 1usize;
            let (mut a, mut b) = (0usize, 1usize);
            for _ in 0..n {
                len = a + b;
                a = b;
                b = len;
            }
            len
        };
        for i in 0..8 {
            assert_eq!(expand(b"a", &rules, i).len(), fib(i + 1));
        }
        assert_eq!(expand(b"a", &rules, 3), b"abaab".to_vec());
    }

    #[test]
    fn unmatched_symbols_copy_through() {
        let rules = [(b'A', b"AB".to_vec())];
        assert_eq!(expand(b"AzA", &rules, 1), b"ABzAB".to_vec());
        // First rule wins on duplicates (documented semantics).
        let dup = [(b'A', b"X".to_vec()), (b'A', b"Y".to_vec())];
        assert_eq!(expand(b"A", &dup, 1), b"X".to_vec());
        assert_eq!(expand(b"A", &dup, 2), b"X".to_vec()); // X has no rule → copies
    }

    #[test]
    fn turtle_draws_a_square() {
        // F+F+F+F with quarter turns = a 1-cell square outline.
        let cells = turtle_cells(b"F+F+F+F", &DIRS_4, (0, 0), 0);
        let set: std::collections::BTreeSet<_> = cells.iter().copied().collect();
        let expected: std::collections::BTreeSet<_> = [(0, 0), (1, 0), (1, -1), (0, -1), (0, 0)]
            .iter()
            .copied()
            .collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn turtle_branches_restore_state() {
        // F[+F]F — draw two cells forward, branch north, continue east.
        let cells = turtle_cells(b"F[+F]F", &DIRS_4, (0, 0), 0);
        let set: std::collections::BTreeSet<_> = cells.iter().copied().collect();
        let expected: std::collections::BTreeSet<_> =
            [(0, 0), (1, 0), (1, -1), (2, 0)].iter().copied().collect();
        assert_eq!(set, expected, "branch push/pop must restore position");
    }

    #[test]
    fn expanded_tree_draws_connected_cells() {
        // The drawn cell *set* must be 8-connected (the draw sequence
        // legitimately teleports on `]` pops — connectivity, not
        // consecutive-step adjacency, is the invariant).
        let rules = [(b'X', b"F[+X][-X]FX".to_vec()), (b'F', b"FF".to_vec())];
        let prog = expand(b"X", &rules, 3);
        let cells = turtle_cells(&prog, &DIRS_8, (0, 0), 0);
        assert!(cells.len() > 20);
        let set: std::collections::BTreeSet<_> = cells.iter().copied().collect();
        // BFS over 8-neighbours from an arbitrary cell must reach all.
        let mut seen = std::collections::BTreeSet::from([*set.iter().next().unwrap_or(&(0, 0))]);
        let mut queue: std::collections::VecDeque<_> = seen.iter().copied().collect();
        while let Some((x, y)) = queue.pop_front() {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let n = (x + dx, y + dy);
                    if set.contains(&n) && seen.insert(n) {
                        queue.push_back(n);
                    }
                }
            }
        }
        assert_eq!(seen.len(), set.len(), "plant must be one 8-connected blob");
    }

    #[test]
    fn deterministic_and_empty_safe() {
        let rules = [(b'X', b"F[+X]-X".to_vec())];
        let prog = expand(b"X", &rules, 4);
        assert_eq!(
            turtle_cells(&prog, &DIRS_8, (5, 5), 0),
            turtle_cells(&prog, &DIRS_8, (5, 5), 0)
        );
        assert!(turtle_cells(b"", &DIRS_8, (0, 0), 0).is_empty());
        assert!(turtle_cells(b"F", &[], (0, 0), 0).is_empty());
    }
}
