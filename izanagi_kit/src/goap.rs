//! Goal-Oriented Action Planning over bitmask world states — the
//! classic F.E.A.R.-style planner.
//!
//! A world state is a `u64` fact mask (≤ 64 boolean facts). Each
//! [`Action`] names the facts it requires (`pre`), the facts it sets
//! (`set`), and the facts it clears (`clr`), plus a `u64` cost:
//! `state' = (state & !clr) | set` when `state & pre == pre`.
//!
//! [`plan`] runs a canonical best-first search over states — the
//! priority order is `(cost, plan-so-far)` so among minimum-cost plans
//! the lexicographically smallest action-index sequence wins, and the
//! result is a pure function of the inputs.
//!
//! ```
//! use izanagi_kit::goap::{self, Action};
//! // Facts: bit0 = has_wood, bit1 = has_axe, bit2 = fire_lit.
//! let actions = [
//!     Action { name: 0, pre: 0,     set: 0b010, clr: 0, cost: 3 }, // get axe
//!     Action { name: 1, pre: 0b010, set: 0b001, clr: 0, cost: 2 }, // chop wood
//!     Action { name: 2, pre: 0b001, set: 0b100, clr: 0, cost: 5 }, // light fire
//! ];
//! let plan = goap::plan(&actions, 0, 0b100).unwrap();
//! assert_eq!(plan, vec![0, 1, 2]);
//! ```
//!
//! Reference: Orkin, "Applying Goal-Oriented Action Planning to Games"
//! (AI Game Programming Wisdom 2).

/// One planner action: precondition mask, effect masks, cost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action {
    /// Opaque user tag reported in the plan instead of the index —
    /// keep it unique; the plan vector stores indices, use
    /// [`named`] to map them.
    pub name: u32,
    /// Facts that must already hold.
    pub pre: u64,
    /// Facts the action sets.
    pub set: u64,
    /// Facts the action clears.
    pub clr: u64,
    /// Non-negative action cost.
    pub cost: u64,
}

/// Minimum-cost plan from `start` to any state covering `goal`, as a
/// canonical vector of action indices (`Vec<u32>`). `None` when no
/// action sequence reaches the goal.
///
/// The search is Dijkstra-ordered on `(cost, sequence)`: the first
/// time a goal state pops, its plan is optimal *and* canonical.
/// States are keyed by mask — `2^k` reachable masks are visited once.
pub fn plan(actions: &[Action], start: u64, goal: u64) -> Option<Vec<u32>> {
    if start & goal == goal {
        return Some(Vec::new());
    }
    use std::collections::{BTreeMap, BTreeSet};
    // Open set ordered by (cost, plan, state); plan inside the key
    // makes the expansion deterministic and canonical.
    let mut open: BTreeSet<(u64, Vec<u32>, u64)> = BTreeSet::new();
    let mut best: BTreeMap<u64, u64> = BTreeMap::new();
    open.insert((0, Vec::new(), start));
    best.insert(start, 0);
    while let Some((cost, seq, state)) = {
        let v = open.iter().next().cloned();
        if let Some(v) = &v {
            open.remove(v);
        }
        v
    } {
        if state & goal == goal {
            return Some(seq);
        }
        if cost > *best.get(&state).unwrap_or(&u64::MAX) {
            continue; // stale entry
        }
        for (i, a) in actions.iter().enumerate() {
            if state & a.pre != a.pre {
                continue;
            }
            let next = (state & !a.clr) | a.set;
            if next == state {
                continue;
            }
            let nc = cost + a.cost;
            if nc < *best.get(&next).unwrap_or(&u64::MAX) {
                best.insert(next, nc);
                let mut nseq = seq.clone();
                nseq.push(i as u32);
                open.insert((nc, nseq, next));
            }
        }
    }
    None
}

/// Map a plan's action indices through [`Action::name`].
pub fn named(actions: &[Action], plan: &[u32]) -> Vec<u32> {
    plan.iter()
        .filter_map(|&i| actions.get(i as usize).map(|a| a.name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Exhaustive DFS oracle over plans of bounded length — compares
    /// optimal cost AND the canonical (lex-min index) sequence.
    fn oracle(actions: &[Action], start: u64, goal: u64, max_len: usize) -> Option<Vec<u32>> {
        let mut best: Option<(u64, Vec<u32>)> = None;
        let mut stack: Vec<(u64, Vec<u32>, u64)> = vec![(start, Vec::new(), 0)];
        while let Some((state, seq, cost)) = stack.pop() {
            if state & goal == goal {
                let better = match &best {
                    None => true,
                    Some((bc, bseq)) => (cost, seq.clone()) < (*bc, bseq.clone()),
                };
                if better {
                    best = Some((cost, seq.clone()));
                }
                continue;
            }
            if seq.len() >= max_len {
                continue;
            }
            for (i, a) in actions.iter().enumerate() {
                if state & a.pre != a.pre {
                    continue;
                }
                let next = (state & !a.clr) | a.set;
                if next == state {
                    continue;
                }
                let mut nseq = seq.clone();
                nseq.push(i as u32);
                stack.push((next, nseq, cost + a.cost));
            }
        }
        best.map(|(_, s)| s)
    }

    #[test]
    fn basics() {
        // Doc example.
        let actions = [
            Action {
                name: 10,
                pre: 0,
                set: 0b010,
                clr: 0,
                cost: 3,
            },
            Action {
                name: 11,
                pre: 0b010,
                set: 0b001,
                clr: 0,
                cost: 2,
            },
            Action {
                name: 12,
                pre: 0b001,
                set: 0b100,
                clr: 0,
                cost: 5,
            },
        ];
        let plan = plan(&actions, 0, 0b100).unwrap();
        assert_eq!(plan, vec![0, 1, 2]);
        assert_eq!(named(&actions, &plan), vec![10, 11, 12]);
        // Already satisfied.
        assert_eq!(plan_fn(&actions, 0b111, 0b100), Some(vec![]));
        // Unreachable.
        assert!(plan_fn(&actions, 0, 0b1000).is_none());
    }

    fn plan_fn(actions: &[Action], start: u64, goal: u64) -> Option<Vec<u32>> {
        plan(actions, start, goal)
    }

    #[test]
    fn cheaper_beats_shorter() {
        // Two routes: a→goal direct cost 9, or b→goal via cheap chain.
        let actions = [
            Action {
                name: 0,
                pre: 0,
                set: 0b01,
                clr: 0,
                cost: 1,
            },
            Action {
                name: 1,
                pre: 0b01,
                set: 0b10,
                clr: 0,
                cost: 1,
            },
            Action {
                name: 2,
                pre: 0,
                set: 0b10,
                clr: 0,
                cost: 9,
            },
        ];
        assert_eq!(plan(&actions, 0, 0b10), Some(vec![0, 1]));
    }

    #[test]
    fn oracle_small() {
        let mut rng = SplitMix64::new(0x604a_c0de_d00d_f00d);
        for _case in 0..40 {
            let n = 1 + rng.below(5) as usize;
            let bits = rng.below(4) as usize + 1;
            let mut actions = Vec::with_capacity(n);
            for i in 0..n {
                let pick = |rng: &mut SplitMix64| -> u64 {
                    let mut m = 0;
                    for b in 0..bits {
                        if rng.below(3) == 0 {
                            m |= 1u64 << b;
                        }
                    }
                    m
                };
                actions.push(Action {
                    name: i as u32,
                    pre: pick(&mut rng),
                    set: pick(&mut rng),
                    clr: pick(&mut rng),
                    cost: 1 + rng.below(9) as u64,
                });
            }
            let start = rng.next_u64() & ((1u64 << bits) - 1);
            let goal = rng.next_u64() & ((1u64 << bits) - 1);
            let mine = plan(&actions, start, goal);
            let brute = oracle(&actions, start, goal, 8);
            assert_eq!(mine, brute);
        }
    }
}
