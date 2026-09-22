//! Deterministic negamax with alpha-beta pruning over a caller-defined
//! `Game` trait — the adversarial-search layer for board/tactics AI:
//! same position, same depth, same move, every time (moves are tried
//! in the canonical order `Game::moves` returns, and ties keep the
//! *first* candidate). Where a sampling search would randomize, this
//! exhausts to a fixed depth with no randomness at all.
//!
//! ```
//! use izanagi_kit::minimax::Game;
//! ```
//!
//! A game is `Clone + moves + apply + evaluate + terminal`; terminal
//! scores should fold in ply (`WIN − depth`) so faster wins rank
//! higher — see tests for a complete Tic-Tac-Toe oracle.

/// A finite zero-sum game with perfect information.
pub trait Game: Clone {
    /// Move token — any `Copy + Ord` type; `moves` order is the
    /// canonical tie-break order.
    type Move: Copy + Eq;

    /// Legal moves from this position, in canonical order.
    fn moves(&self) -> Vec<Self::Move>;

    /// Position after `m` (functional — the receiver is unchanged).
    fn apply(&self, m: Self::Move) -> Self;

    /// Static evaluation, side-to-move perspective. Only consulted at
    /// the depth horizon (non-terminal positions).
    fn evaluate(&self) -> i64;

    /// `Some(score)` when the game is over — side-to-move
    /// perspective, e.g. `Some(WIN - ply)` on a win for the player
    /// *not* to move, expressed as the negated value for the mover.
    fn terminal(&self) -> Option<i64>;
}

/// Sentinel-scale win score for [`Game::terminal`] implementations.
pub const WIN: i64 = 1_000_000;

fn negamax<G: Game>(g: &G, depth: u32, mut alpha: i64, beta: i64) -> i64 {
    if let Some(v) = g.terminal() {
        return v;
    }
    if depth == 0 {
        return g.evaluate();
    }
    let mut best = i64::MIN / 2;
    for m in g.moves() {
        let v = -negamax(&g.apply(m), depth - 1, -beta, -alpha);
        if v > best {
            best = v;
        }
        if v > alpha {
            alpha = v;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

/// Exact minimax value of `g` to `depth` plies (alpha-beta inside —
/// the score equals an exhaustive negamax; pruning never changes the
/// value, only the work).
pub fn score<G: Game>(g: &G, depth: u32) -> i64 {
    negamax(g, depth, i64::MIN / 2, i64::MAX / 2)
}

/// Best first move: `(move, value)` — the first canonical-order move
/// attaining the maximum. `None` when the position is terminal or has
/// no moves.
pub fn best_move<G: Game>(g: &G, depth: u32) -> Option<(G::Move, i64)> {
    if g.terminal().is_some() {
        return None;
    }
    let mut best: Option<(G::Move, i64)> = None;
    let mut alpha = i64::MIN / 2;
    for m in g.moves() {
        let v = -negamax(&g.apply(m), depth.saturating_sub(1), i64::MIN / 2, -alpha);
        match best {
            Some((_, bv)) if v <= bv => {}
            _ => best = Some((m, v)),
        }
        if v > alpha {
            alpha = v;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tic-Tac-Toe — compact known-answer game. `board[i]` is 0 empty,
    /// 1 X, 2 O; `turn` is the side to move (1 or 2).
    #[derive(Clone)]
    struct Ttt {
        board: [u8; 9],
        turn: u8,
        ply: u32,
    }

    const LINES: [[usize; 3]; 8] = [
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8],
        [0, 4, 8],
        [2, 4, 6],
    ];

    impl Ttt {
        fn new() -> Self {
            Self {
                board: [0; 9],
                turn: 1,
                ply: 0,
            }
        }

        fn winner(&self) -> Option<u8> {
            for l in LINES {
                let b = self.board[l[0]];
                if b != 0 && b == self.board[l[1]] && b == self.board[l[2]] {
                    return Some(b);
                }
            }
            None
        }
    }

    impl Game for Ttt {
        type Move = u8;

        fn moves(&self) -> Vec<u8> {
            (0..9).filter(|&i| self.board[i as usize] == 0).collect()
        }

        fn apply(&self, m: u8) -> Self {
            let mut g = self.clone();
            g.board[m as usize] = g.turn;
            g.turn = 3 - g.turn;
            g.ply += 1;
            g
        }

        fn evaluate(&self) -> i64 {
            0
        }

        fn terminal(&self) -> Option<i64> {
            if let Some(w) = self.winner() {
                // Winner = side that just moved; mover sees −WIN+ply.
                return Some(if w == self.turn {
                    WIN - self.ply as i64
                } else {
                    -(WIN - self.ply as i64)
                });
            }
            if self.moves().is_empty() {
                return Some(0);
            }
            None
        }
    }

    /// Exhaustive negamax without pruning — the oracle.
    fn naive<G: Game>(g: &G, depth: u32) -> i64 {
        if let Some(v) = g.terminal() {
            return v;
        }
        if depth == 0 {
            return g.evaluate();
        }
        let mut best = i64::MIN / 2;
        for m in g.moves() {
            best = best.max(-naive(&g.apply(m), depth - 1));
        }
        best
    }

    #[test]
    fn ttt_known_answers() {
        // Perfect play draws.
        let g = Ttt::new();
        assert_eq!(score(&g, 9), 0);
        let (m, v) = best_move(&g, 9).unwrap_or((255, i64::MIN));
        assert_eq!(v, 0);
        assert_eq!(m, 0); // canonical order keeps the first
                          // X to move, X wins in one: XX_ / ___ / ___ is not yet
                          // terminal; playing 2 wins. Wait — need a real win-in-1:
        let mut g = Ttt::new();
        g.board = [1, 1, 0, 0, 2, 0, 0, 0, 0];
        let (m, v) = best_move(&g, 9).unwrap_or((255, 0));
        assert_eq!((m, v > 0), (2, true));
        // O to move against XX_: blocking at 2 delays the loss longest
        // (X still forces a win via the 4 center double-threat), so the
        // ply-discounted score is the shallowest possible defeat.
        let mut g = Ttt::new();
        g.board = [1, 1, 0, 0, 0, 0, 0, 0, 0];
        g.turn = 2;
        let (m, v) = best_move(&g, 9).unwrap_or((255, 0));
        assert_eq!(m, 2);
        assert_eq!(v, -(WIN - 4));
    }

    #[test]
    fn alpha_beta_matches_naive() {
        // Every reachable position at depth d must score identically
        // with and without pruning.
        fn walk(g: &Ttt, depth: u32) {
            assert_eq!(score(g, depth), naive(g, depth));
            for m in g.moves() {
                let c = g.apply(m);
                if depth > 0 {
                    walk(&c, depth - 1);
                }
            }
        }
        walk(&Ttt::new(), 4);
        // Self-play under full search: 9-ply perfect game is a draw.
        let mut g = Ttt::new();
        while g.terminal().is_none() {
            let (m, _) = best_move(&g, 9).unwrap_or((0, 0));
            g = g.apply(m);
        }
        assert_eq!(g.winner(), None);
    }
}
