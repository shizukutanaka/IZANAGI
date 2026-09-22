//! Monte-Carlo tree search — seeded UCB1 over the
//! [`crate::minimax::Game`] trait. Where `minimax` is exhaustive
//! (small trees), MCTS is the scalable counterpart: `iterations`
//! bounded playouts through a growing arena tree, selection by an
//! integer UCB, rollouts via [`crate::rng::SplitMix64`] — the whole
//! search is a pure function of `(position, budget, seed)`.
//!
//! Integer UCB: `score = winrate_permille + c·√(⌈log2 N⌉/n)` —
//! `log2` in place of `ln` only rescales the exploration constant
//! (`ln N = ln2·log2 N`), so no floats appear anywhere. Playout
//! results map by sign to `{0, 500, 1000}` permille; a rollout that
//! exceeds `rollout_cap` plies scores by the sign of `evaluate()`.
//! Children expand in canonical `moves()` order and unvisited
//! children are tried first, so the returned move is deterministic.
//!
//! ```
//! use izanagi_kit::mcts::mcts;
//! // (see tests for a Game impl) — returns None at a terminal root.
//! ```

use crate::minimax::Game;
use crate::rng::SplitMix64;

/// One recommended move plus its search statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MctsResult<M> {
    /// The move — most-visited root child, canonical order on ties.
    pub mov: M,
    /// Playouts routed through the move.
    pub visits: u32,
    /// Estimated win rate for the side to move at the root (0–1000).
    pub win_permille: u32,
}

#[derive(Clone, Debug)]
struct Node<M> {
    /// child edges: (move, node index) in canonical moves() order.
    children: Vec<(M, u32)>,
    /// moves not yet expanded — also canonical order.
    unexpanded: Vec<M>,
    visits: u32,
    /// accumulated wins_permille, side-to-move-at-this-node view.
    wins: u64,
}

fn log2_ceil(n: u32) -> u64 {
    // ⌈log2 n⌉ with log2_ceil(0)=log2_ceil(1)=0
    (32 - (n.max(1) - 1).leading_zeros()) as u64
}

fn isqrt(n: u64) -> u64 {
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Integer UCB in permille, *from the parent's perspective*:
/// `wins` is stored in the child's side-to-move view, so the
/// parent sees `1000 − child_mean` plus
/// `c·√(⌈log2(N)⌉·10⁶/n)/1000`. Unvisited children sort first.
fn ucb<M>(n: &Node<M>, log_n: u64, c_permille: u32) -> u64 {
    if n.visits == 0 {
        return u64::MAX;
    }
    let mean = 1000 - n.wins / n.visits as u64;
    let bonus = (c_permille as u64 * isqrt(log_n * 1_000_000 / n.visits as u64)) / 1000;
    mean + bonus
}

/// Run `iterations` playouts from `root` — `None` when the root is
/// terminal or has no moves. `c_permille` ~1414 is the classic √2;
/// `rollout_cap` bounds simulation length (`evaluate()` sign
/// breaks the tie when reached — `0` also draws instantly).
pub fn mcts<G: Game>(
    root: &G,
    iterations: u32,
    c_permille: u32,
    rollout_cap: u32,
    rng: &mut SplitMix64,
) -> Option<MctsResult<G::Move>> {
    if root.terminal().is_some() {
        return None;
    }
    let mut nodes: Vec<Node<G::Move>> = Vec::new();
    nodes.push(Node {
        children: Vec::new(),
        unexpanded: root.moves(),
        visits: 0,
        wins: 0,
    });
    if nodes[0].unexpanded.is_empty() {
        return None;
    }
    for _ in 0..iterations {
        // SELECTION — descend by UCB until a node with unexpanded
        // moves (or a terminal leaf) is reached, recording the path.
        let mut path: Vec<usize> = vec![0];
        let mut cur = 0usize;
        let mut g = root.clone();
        let mut leaf_terminal: Option<i64> = None;
        loop {
            if let Some(t) = g.terminal() {
                leaf_terminal = Some(t);
                break;
            }
            if !nodes[cur].unexpanded.is_empty() {
                break;
            }
            let log_n = log2_ceil(nodes[cur].visits.max(1));
            let mut best = 0usize;
            let mut best_score = 0u64;
            for (ci, &(_, child)) in nodes[cur].children.iter().enumerate() {
                let s = ucb(&nodes[child as usize], log_n, c_permille);
                if ci == 0 || s > best_score {
                    best = ci;
                    best_score = s;
                }
            }
            let (m, child) = nodes[cur].children[best];
            g = g.apply(m);
            cur = child as usize;
            path.push(cur);
        }
        // EXPANSION — one new child per playout, canonical order.
        let reward_mover_sign: i64 = if leaf_terminal.is_none() {
            let m = nodes[cur].unexpanded.remove(0);
            g = g.apply(m);
            let child = nodes.len() as u32;
            nodes.push(Node {
                children: Vec::new(),
                unexpanded: g.moves(),
                visits: 0,
                wins: 0,
            });
            nodes[cur].children.push((m, child));
            cur = child as usize;
            path.push(cur);
            rollout(&g, rollout_cap, rng)
        } else {
            leaf_terminal.unwrap_or(0).signum()
        };
        // BACKPROP — credit every node on the path; the permille
        // reward is recorded from that node's side-to-move view,
        // so the sign alternates each level going up.
        let mut v = reward_mover_sign;
        for &ni in path.iter().rev() {
            let permille: u64 = match v {
                x if x > 0 => 1000,
                x if x < 0 => 0,
                _ => 500,
            };
            nodes[ni].visits += 1;
            nodes[ni].wins += permille;
            v = -v;
        }
    }
    // most-visited root child; canonical order on ties
    let mut best: Option<&(G::Move, u32)> = None;
    for e in nodes[0].children.iter() {
        match best {
            None => best = Some(e),
            Some(b) if nodes[e.1 as usize].visits > nodes[b.1 as usize].visits => best = Some(e),
            _ => {}
        }
    }
    let &(m, ci) = best?;
    let n = &nodes[ci as usize];
    Some(MctsResult {
        mov: m,
        visits: n.visits,
        // child wins are stored in the *opponent's* perspective —
        // flip for the root mover.
        win_permille: (1000 - (n.wins / n.visits.max(1) as u64)) as u32,
    })
}

/// Random playout — sign of `terminal()` for the mover at `g`,
/// or sign of `evaluate()` at the cap. Returns `−1, 0, +1`.
fn rollout<G: Game>(g: &G, cap: u32, rng: &mut SplitMix64) -> i64 {
    let mut g = g.clone();
    for _ in 0..cap {
        if let Some(t) = g.terminal() {
            return t.signum();
        }
        let mv = g.moves();
        if mv.is_empty() {
            return 0;
        }
        let i = rng.below(mv.len() as u32) as usize;
        g = g.apply(mv[i]);
    }
    if let Some(t) = g.terminal() {
        return t.signum();
    }
    g.evaluate().signum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tic-Tac-Toe — same fixture style as `minimax` tests.
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
        fn winner(&self) -> Option<u8> {
            for l in LINES {
                let b = self.board[l[0]];
                if b != 0 && b == self.board[l[1]] && b == self.board[l[2]] {
                    return Some(b);
                }
            }
            None
        }
        fn play(&self, seq: &[u8]) -> Self {
            let mut g = self.clone();
            for &m in seq {
                g = g.apply(m);
            }
            g
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
            if self.winner().is_some() {
                return Some(-1);
            }
            if self.moves().is_empty() {
                return Some(0);
            }
            None
        }
    }

    #[test]
    fn finds_forced_wins_and_never_blunders_from_start() {
        let g0 = Ttt {
            board: [0; 9],
            turn: 1,
            ply: 0,
        };
        // X: 0,1 | O: 3,4 — X wins by playing 2
        let win = g0.play(&[0, 3, 1, 4]);
        let mut rng = SplitMix64::new(0x11C7);
        let r = mcts(&win, 400, 1414, 32, &mut rng).unwrap();
        assert_eq!(r.mov, 2);
        assert!(r.visits > 0);

        // immediate win beats everything: X={3,4} wins by 5
        let wins = g0.play(&[3, 0, 4, 1]);
        let r = mcts(&wins, 400, 1414, 32, &mut rng).unwrap();
        assert_eq!(r.mov, 5);
        // must-block: O threatens 0-1-2; only cell 2 avoids an
        // immediate loss — every other child is refuted by rollouts
        let block = g0.play(&[6, 0, 4, 1]);
        let r = mcts(&block, 500, 1414, 32, &mut rng).unwrap();
        assert_eq!(r.mov, 2);
    }

    #[test]
    fn deterministic_for_same_seed_and_edges() {
        let g = Ttt {
            board: [0; 9],
            turn: 1,
            ply: 0,
        };
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        let ra = mcts(&g, 300, 1000, 16, &mut a).unwrap();
        let rb = mcts(&g, 300, 1000, 16, &mut b).unwrap();
        assert_eq!(ra, rb);
        // terminal root → None
        let lost = g.play(&[0, 3, 1, 4, 2]);
        assert!(mcts(&lost, 100, 1000, 16, &mut a).is_none());
    }
}
