//! Planning-based test synthesis: given a goal predicate, search for an
//! input sequence that drives a deterministic simulation from its start
//! state to a state satisfying the goal — "can the player reach the
//! treasure room" becomes an executable replay instead of a hand-authored
//! test script.
//!
//! [`plan_inputs`] is breadth-first search over the state space, so the
//! returned sequence is a **shortest** one (fewest inputs) that reaches a
//! goal state — not just *a* solution. States are deduplicated by their
//! [`DetHash`] digest ([`hash_state`]), so cycles and converging paths don't
//! blow up the search; this is what keeps planning tractable on the small,
//! finite state spaces typical of game logic (grid position plus a few
//! flags) without needing `S: Eq + Hash`.
//!
//! Because [`step`](plan_inputs)'s signature is the same
//! `Fn(&S, &I) -> S` shape used throughout the kit ([`resimulate`](crate::replay::resimulate),
//! [`dst_sweep`](crate::dst::dst_sweep)), a plan found here is directly
//! replayable through those same harnesses — the synthesized input sequence
//! *is* the test.
//!
//! (Using Planning for Automated Testing of Video Games, IJCAI 2025.)

use std::collections::{HashSet, VecDeque};

use crate::world_hash::{hash_state, DetHash};

/// Search breadth-first for a shortest sequence of inputs (drawn from
/// `inputs`) that drives `start` through `step` to a state satisfying `goal`.
/// Returns `Some(vec![])` if `start` already satisfies `goal`. Returns `None`
/// if no such sequence is found within `max_states` explored states — a
/// safety bound against runaway search when the goal is unreachable (or the
/// state space is effectively unbounded).
///
/// - `step(state, input) -> S` is a pure transition function.
/// - `goal(state) -> bool` is the goal predicate.
/// - Ties (multiple shortest sequences) are broken by `inputs`' order: at
///   each expanded state, inputs are tried in the order given, so the result
///   is deterministic across runs for a fixed `inputs` order and search
///   history — matching the kit's replay-determinism convention.
pub fn plan_inputs<S, I, F, G>(
    start: S,
    inputs: &[I],
    step: F,
    goal: G,
    max_states: usize,
) -> Option<Vec<I>>
where
    S: DetHash + Clone,
    I: Clone,
    F: Fn(&S, &I) -> S,
    G: Fn(&S) -> bool,
{
    if goal(&start) {
        return Some(Vec::new());
    }

    let mut visited: HashSet<u64> = HashSet::new();
    visited.insert(hash_state(&start));
    let mut queue: VecDeque<(S, Vec<I>)> = VecDeque::new();
    queue.push_back((start, Vec::new()));
    let mut explored = 1usize;

    while let Some((state, path)) = queue.pop_front() {
        for input in inputs {
            let next = step(&state, input);
            if !visited.insert(hash_state(&next)) {
                continue; // already reached by an equal-or-shorter path
            }
            let mut next_path = path.clone();
            next_path.push(input.clone());
            if goal(&next) {
                return Some(next_path);
            }
            explored += 1;
            if explored > max_states {
                return None;
            }
            queue.push_back((next, next_path));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_start_already_at_goal_is_empty_plan() {
        let plan = plan_inputs(5i32, &[1, -1], |s, d| s + d, |s| *s == 5, 1000);
        assert_eq!(plan, Some(Vec::new()));
    }

    #[test]
    fn test_plan_counter_finds_shortest_sequence() {
        // Only one length-5 composition of {+1, -1} sums to +5: five +1s.
        // Any path mixing in a -1 needs more than 5 net steps, so BFS must
        // return exactly this.
        let plan = plan_inputs(0i32, &[1, -1], |s, d| s + d, |s| *s == 5, 10_000).unwrap();
        assert_eq!(plan, vec![1, 1, 1, 1, 1]);
    }

    #[test]
    fn test_plan_replaying_the_sequence_actually_reaches_the_goal() {
        // The returned plan must be a genuine executable replay: folding
        // `step` over it from `start` lands on a goal-satisfying state.
        let step = |s: &i32, d: &i32| s + d;
        let goal = |s: &i32| *s == 7;
        let plan = plan_inputs(0i32, &[3, -1], step, goal, 10_000).unwrap();
        let end = plan.iter().fold(0i32, |s, d| step(&s, d));
        assert!(goal(&end), "replaying the plan must satisfy the goal");
    }

    #[test]
    fn test_plan_unreachable_goal_returns_none_within_bound() {
        // Goal is never true; a tight max_states bound must terminate
        // promptly instead of hanging on an unbounded counter walk.
        let plan = plan_inputs(0i32, &[1, -1], |s, d| s + d, |_| false, 50);
        assert_eq!(plan, None);
    }

    #[test]
    fn test_plan_is_deterministic() {
        let step = |s: &i32, d: &i32| s + d;
        let goal = |s: &i32| *s == 9;
        let a = plan_inputs(0i32, &[2, 3, -1], step, goal, 10_000);
        let b = plan_inputs(0i32, &[2, 3, -1], step, goal, 10_000);
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    // --- grid maze: a genuine "can the player reach X" scenario ---

    const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    fn grid_step(
        walls: &HashSet<(i32, i32)>,
        w: i32,
        h: i32,
    ) -> impl Fn(&(i32, i32), &usize) -> (i32, i32) + '_ {
        move |&(x, y), &dir| {
            let (dx, dy) = DIRS[dir];
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h || walls.contains(&(nx, ny)) {
                (x, y) // blocked move: stay in place (a no-op, still a valid state to revisit-check)
            } else {
                (nx, ny)
            }
        }
    }

    #[test]
    fn test_plan_finds_path_through_a_maze() {
        // A corridor with a single wall stub forcing a detour.
        let walls: HashSet<(i32, i32)> = [(3, 1), (3, 2), (3, 3)].into_iter().collect();
        let (w, h) = (8, 8);
        let step = grid_step(&walls, w, h);
        let goal_cell = (6, 2);
        let plan = plan_inputs(
            (0i32, 2i32),
            &[0usize, 1, 2, 3],
            &step,
            |&s| s == goal_cell,
            100_000,
        )
        .unwrap();

        // Replay it and confirm it actually lands on the goal cell, stepping
        // only onto in-bounds, unwalled cells.
        let mut cur = (0i32, 2i32);
        for &dir in &plan {
            let next = step(&cur, &dir);
            assert!(!walls.contains(&next), "plan must not step onto a wall");
            assert!(next.0 >= 0 && next.1 >= 0 && next.0 < w && next.1 < h);
            cur = next;
        }
        assert_eq!(cur, goal_cell);
    }

    #[test]
    fn test_plan_matches_bfs_shortest_distance_on_open_grid() {
        // On an open grid the plan's length must equal the Manhattan
        // distance — the true BFS-optimal step count for 4-connected
        // unit-cost movement (same oracle argument as jps4's tests).
        let walls = HashSet::new();
        let (w, h) = (10, 10);
        let step = grid_step(&walls, w, h);
        let start = (1i32, 1i32);
        let goal_cell = (6i32, 4i32);
        let plan = plan_inputs(
            start,
            &[0usize, 1, 2, 3],
            step,
            |&s| s == goal_cell,
            100_000,
        )
        .unwrap();
        let manhattan = (goal_cell.0 - start.0).abs() + (goal_cell.1 - start.1).abs();
        assert_eq!(plan.len() as i32, manhattan);
    }

    #[test]
    fn test_plan_unreachable_cell_behind_full_wall_returns_none() {
        // Fully enclose the goal cell — no plan can reach it.
        let mut walls = HashSet::new();
        for (dx, dy) in DIRS {
            walls.insert((5 + dx, 5 + dy));
        }
        let (w, h) = (10, 10);
        let step = grid_step(&walls, w, h);
        let plan = plan_inputs(
            (0i32, 0i32),
            &[0usize, 1, 2, 3],
            step,
            |&s| s == (5, 5),
            100_000,
        );
        assert_eq!(plan, None);
    }
}
