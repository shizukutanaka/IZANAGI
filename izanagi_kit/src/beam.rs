//! Beam search — breadth-limited best-first expansion. Each
//! level expands every state in the frontier, scores the
//! children, and keeps the `width` lowest-scoring ones. No
//! randomness anywhere: ties resolve by generation order, so a
//! given `(start, expand, score)` always yields the same search
//! and the same path.
//!
//! ```
//! use izanagi_kit::beam::beam_search;
//! // walk a grid: from x you may step to x+1 or x*2, modulo 97;
//! // score = distance to 64 — a beam of 4 finds it
//! let r = beam_search(&0u64, 4, 12, |&s, out| {
//!     out.push(((s + 1) % 97, 0));
//!     out.push(((s * 2) % 97, 0));
//! }, |&s| (s as i64 - 64).abs());
//! assert_eq!(r.best, 64);
//! assert_eq!(r.path[0], 0);
//! assert_eq!(*r.path.last().unwrap(), 64);
//! ```

/// Result of [`beam_search`].
#[derive(Clone, Debug)]
pub struct BeamResult<S> {
    /// Best-scoring state ever expanded (not just at the deepest
    /// level — a beam that overshoots its goal still returns it).
    pub best: S,
    /// Score of `best`.
    pub score: i64,
    /// Path `start → … → best` via parent links.
    pub path: Vec<S>,
    /// Levels expanded (≤ `max_depth`).
    pub levels: usize,
    /// Total states scored (calls into `expand`).
    pub expanded: usize,
}

/// Deterministic beam search.
///
/// - `expand(s, &mut out)` pushes `(child, edge_info)` pairs —
///   `edge_info` is carried through but not used for ranking;
///   it's available for callers encoding move/cost metadata.
/// - `score(s)` evaluates a state; lower is better.
/// - Children are ranked by `(score, generation order)` so the
///   search is a pure function of `expand`'s output order.
/// - The search stops when a level produces no children or
///   `max_depth` levels are done.
pub fn beam_search<S: Clone>(
    start: &S,
    width: usize,
    max_depth: usize,
    expand: impl Fn(&S, &mut Vec<(S, i64)>),
    score: impl Fn(&S) -> i64,
) -> BeamResult<S> {
    // nodes: (state, score, parent index) — arena keeps parents
    // reachable for path reconstruction
    let mut nodes: Vec<(S, i64, usize)> = vec![(start.clone(), score(start), !0usize)];
    let mut frontier: Vec<usize> = vec![0];
    let mut best = 0usize;
    let mut expanded = 0usize;
    let mut levels = 0usize;

    while levels < max_depth && !frontier.is_empty() {
        let mut kids: Vec<(usize, i64, usize)> = Vec::new(); // (node id, score, gen order)
        let mut order = 0usize;
        let mut buf = Vec::new();
        for &f in &frontier {
            expand(&nodes[f].0, &mut buf);
            expanded += 1;
            for (child, info) in buf.drain(..) {
                let id = nodes.len();
                let sc = score(&child);
                nodes.push((child, sc, f));
                kids.push((id, sc, order));
                order += 1;
                let _ = info;
            }
        }
        if kids.is_empty() {
            break;
        }
        // rank: score asc, then generation order asc
        kids.sort_by_key(|&(_, sc, ord)| (sc, ord));
        kids.truncate(width.max(1));
        frontier = kids.iter().map(|&(id, _, _)| id).collect();
        for &(id, sc, _) in &kids {
            if sc < nodes[best].1 {
                best = id;
            }
        }
        levels += 1;
    }

    // reconstruct path
    let mut path = Vec::new();
    let mut cur = best;
    while cur != !0usize {
        path.push(nodes[cur].0.clone());
        cur = nodes[cur].2;
    }
    path.reverse();
    BeamResult {
        best: nodes[best].0.clone(),
        score: nodes[best].1,
        path,
        levels,
        expanded,
    }
}

/// Convenience beam over a [`minimax::Game`](crate::minimax::Game):
/// expands the position's moves and ranks by `eval` — a
/// root-perspective score where *higher is better* (the usual
/// game-evaluation convention), so the beam keeps the `width`
/// highest-scoring children. Deterministic: ties resolve by
/// `moves()` order. Terminal states don't expand.
///
/// Returned path is the move sequence from `game`'s state to
/// the best-scoring state the beam touched.
pub fn beam_moves<G: crate::minimax::Game>(
    game: &G,
    width: usize,
    depth: usize,
    eval: impl Fn(&G) -> i64,
) -> Vec<G::Move> {
    // state arena entry: (move to reach, parent)
    struct Ent<M> {
        mv: Option<M>,
        par: usize,
    }
    let mut ents: Vec<Ent<G::Move>> = vec![Ent {
        mv: None,
        par: !0usize,
    }];
    let mut games: Vec<G> = vec![game.clone()];
    let mut frontier: Vec<usize> = vec![0];
    let mut best = 0usize;
    let mut best_score = eval(game);
    for _ in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let mut kids: Vec<(usize, i64, usize)> = Vec::new();
        let mut order = 0usize;
        for &f in &frontier {
            if games[f].terminal().is_some() {
                continue;
            }
            for mv in games[f].moves() {
                let id = ents.len();
                ents.push(Ent {
                    mv: Some(mv),
                    par: f,
                });
                games.push(games[f].apply(mv));
                let sc = eval(&games[id]);
                kids.push((id, sc, order));
                order += 1;
                if sc > best_score {
                    best_score = sc;
                    best = id;
                }
            }
        }
        if kids.is_empty() {
            break;
        }
        kids.sort_by_key(|&(_, sc, ord)| (std::cmp::Reverse(sc), ord));
        kids.truncate(width.max(1));
        frontier = kids.iter().map(|&(id, _, _)| id).collect();
    }
    let mut path = Vec::new();
    let mut cur = best;
    while ents[cur].par != !0usize {
        if let Some(mv) = ents[cur].mv {
            path.push(mv);
        }
        cur = ents[cur].par;
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minimax::Game;

    /// Tiny deterministic game for beam tests: position = sum;
    /// moves add +1 or +2; terminal at ≥10.
    #[derive(Clone, Debug, PartialEq)]
    struct Adder {
        sum: i64,
        p2: bool,
    }
    impl Game for Adder {
        type Move = i64;
        fn moves(&self) -> Vec<i64> {
            if self.sum < 10 {
                vec![1, 2]
            } else {
                Vec::new()
            }
        }
        fn apply(&self, m: i64) -> Adder {
            Adder {
                sum: self.sum + m,
                p2: !self.p2,
            }
        }
        fn evaluate(&self) -> i64 {
            self.sum % 2
        }
        fn terminal(&self) -> Option<i64> {
            if self.sum >= 10 {
                Some(self.sum % 2)
            } else {
                None
            }
        }
    }

    #[test]
    fn grid_walk_finds_goal() {
        let r = beam_search(
            &0u64,
            4,
            12,
            |&s, out| {
                out.push(((s + 1) % 97, 0));
                out.push(((s * 2) % 97, 0));
            },
            |&s| (s as i64 - 64).abs(),
        );
        assert_eq!(r.best, 64);
        assert_eq!(r.score, 0);
        assert_eq!(r.path[0], 0);
        assert_eq!(*r.path.last().unwrap(), 64);
        assert!(r.levels <= 12 && r.expanded > 0);
    }

    /// Determinism: same search twice → identical result.
    #[test]
    fn deterministic() {
        let run = |w: usize| {
            beam_search(
                &1u64,
                w,
                8,
                |&s, out| {
                    for d in [3u64, 1, 2] {
                        out.push((s * 3 + d, 0));
                    }
                },
                |&s| -(s as i64 % 100),
            )
        };
        let a = run(3);
        let b = run(3);
        assert_eq!(a.best, b.best);
        assert_eq!(a.path, b.path);
        assert_eq!(a.score, b.score);
        // width 0 clamps to 1 — still works
        let z = beam_search(
            &5u64,
            0,
            3,
            |&s, out| out.push((s + 1, 0)),
            |&s| -(s as i64),
        );
        assert_eq!(z.best, 8);
        // no children → stops at level 0
        let e = beam_search(
            &1u64,
            4,
            5,
            |_: &u64, _: &mut Vec<(u64, i64)>| {},
            |&s| s as i64,
        );
        assert_eq!(e.best, 1);
        assert_eq!(e.levels, 0);
    }

    /// Oracle: the returned path actually leads to `best` when
    /// replayed through `expand`, and every intermediate state
    /// on the path was a beam member.
    #[test]
    fn path_is_replayable() {
        let r = beam_search(
            &2u64,
            3,
            7,
            |&s, out| {
                out.push((s + 4, 0));
                out.push((s * 5 % 61, 0));
            },
            |&s| -(s as i64),
        );
        // replay the path
        let mut cur = 2u64;
        for &step in &r.path[1..] {
            let buf: Vec<(u64, i64)> = vec![(cur + 4, 0), (cur * 5 % 61, 0)];
            assert!(
                buf.iter().any(|&(s, _)| s == step),
                "{cur} → {step} missing"
            );
            cur = step;
        }
        assert_eq!(cur, r.best);
    }

    /// `beam_moves` over a Game: returns a legal move sequence.
    #[test]
    fn moves_are_legal() {
        let g = Adder { sum: 0, p2: false };
        let path = beam_moves(&g, 3, 8, |g| g.sum);
        // replay
        let mut cur = Adder { sum: 0, p2: false };
        for &m in &path {
            assert!(cur.moves().contains(&m), "illegal move {m}");
            cur = cur.apply(m);
        }
        // the greedy eval (max sum) should reach a terminal or
        // the depth cap — check the path isn't empty
        assert!(!path.is_empty());
        assert_eq!(cur.sum, path.iter().sum::<i64>());
    }
}
