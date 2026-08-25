//! Archive-based state-space exploration: remember where you have been, go
//! back there deliberately, and explore onward from the frontier.
//!
//! ## The gap this fills
//!
//! The kit already has two ways to search a simulation's state space, and
//! they sit at opposite extremes:
//!
//! - [`plan::plan_inputs`](crate::plan::plan_inputs) is breadth-first search —
//!   **complete** (it finds a shortest input sequence if one exists) but its
//!   frontier grows exponentially with depth, so it gives up at `max_states`
//!   with nothing to show for the work.
//! - [`prop::forall_states`](crate::prop::forall_states) and the
//!   [`dst`](crate::dst) sweeps are random play — cheap and unbounded in
//!   depth, but **memoryless**: every seed starts over from the initial state,
//!   so reaching a state that needs a long specific prefix is exponentially
//!   unlikely.
//!
//! Ecoffet, Huizinga, Lehman, Stanley & Clune diagnosed exactly this failure
//! in random exploration (*First return, then explore*, Nature 590:580–586,
//! 2021; arXiv:1901.10995) and named its two causes:
//!
//! - **detachment** — the searcher forgets the promising places it already
//!   reached, and re-derives them (or never does) instead of pushing past them;
//!   and
//! - **derailment** — even when a promising state is known, exploratory
//!   randomness knocks the searcher off course on the way back to it.
//!
//! Their fix is *Go-Explore*: keep an **archive** of the distinct states seen,
//! each with the shortest path known to reach it; then repeatedly (1) pick an
//! archived state, (2) **return** to it *without* exploring — deterministically
//! — and only then (3) explore randomly from there, filing any newly reached
//! states back into the archive.
//!
//! ## Why it belongs in this crate specifically
//!
//! Step (2) is the hard part in general, and the paper is explicit about it:
//! returning without derailment needs an environment you can *reset* to a
//! previous state, which is why their headline results use emulator state
//! restore, and why stochastic environments need a whole extra
//! "robustification" phase to convert the found trajectory into a robust
//! policy.
//!
//! Here that precondition is the crate's founding guarantee. A
//! [`Simulation`] is a pure function of `(state, input)`, so returning to an
//! archived state is a `Clone` — and the path stored alongside it *is* a
//! replayable test, valid in [`replay::resimulate`](crate::replay::resimulate)
//! and every other harness. No robustification phase exists because there is
//! no stochasticity to robustify against.
//!
//! ## Cells: the one parameter that matters
//!
//! An archive keyed by the exact state can only ever be as small as the state
//! space, so Go-Explore keys it by a **cell** — a deliberate down-sampling of
//! the state (the paper down-samples game frames to a coarse grid). Here the
//! cell function is `Fn(&S) -> u64`, and the two useful ends are:
//!
//! - [`hash_state`](crate::world_hash::hash_state) — every distinct state is
//!   its own cell. Exact, and the
//!   right choice for small state spaces.
//! - a coarse projection, e.g. hashing only the player's position and
//!   ignoring inventory. Fewer cells, so exploration pushes *outward* instead
//!   of re-cataloguing minor variations — which is the whole point of the
//!   down-sampling.
//!
//! When several states share a cell, the archive keeps one representative:
//! the one reached by the shortest known path — so the path it hands back is
//! the most useful one as a regression test.
//!
//! ## Sizing `steps_per_iteration` against the cell
//!
//! That choice has a consequence worth stating plainly, because it decides
//! whether coarse cells work at all. Keeping the *shortest*-path
//! representative means a return lands at the cell's **entry edge** — the
//! first state of the cell that exploration happened to touch. An exploration
//! walk of `steps_per_iteration` inputs therefore has to cross the whole cell
//! before it can discover the next one, and if the cell is wider than the walk
//! is long, it never escapes: exploration stalls with a full selection budget
//! spent inside one cell.
//!
//! Measured on a 90-long corridor, 300 iterations, `+1`/`-1` inputs (cells =
//! position bucketed by width):
//!
//! | cell width | walk 8 | walk 12 | walk 24 |
//! |---|---|---|---|
//! | 1 (exact) | 75 cells | 91 | 90 |
//! | 3 | 28 | 30 | 31 |
//! | 5 | 9 | 14 | 19 |
//! | 10 | **1 (stalled)** | **1 (stalled)** | 7 |
//!
//! So: **make `steps_per_iteration` comfortably larger than the diameter of a
//! cell**. The symptom of getting it wrong is unmistakable once you know to
//! look — [`Archive::len`] near 1 with the whole selection budget recorded in
//! [`Archive::selections`] on that one cell.
//!
//! ```
//! use izanagi_kit::explore::{explore_until, ExploreConfig};
//! use izanagi_kit::world_hash::{hash_state, DetHash, Fnv1a};
//!
//! // A corridor: to reach the far end, a random walk must guess right ~40
//! // times in a row. Archived exploration walks it in a few hundred steps.
//! #[derive(Clone)]
//! struct Corridor {
//!     at: i32,
//! }
//! impl DetHash for Corridor {
//!     fn det_hash(&self, h: &mut Fnv1a) {
//!         h.write_i32(self.at);
//!     }
//! }
//!
//! let path = explore_until(
//!     &Corridor { at: 0 },
//!     &[1i32, -1],
//!     |s: &Corridor, i: &i32| Corridor {
//!         at: (s.at + i).clamp(0, 40),
//!     },
//!     hash_state,
//!     |s: &Corridor| s.at == 40,
//!     &ExploreConfig {
//!         seed: 7,
//!         iterations: 400,
//!         steps_per_iteration: 8,
//!         max_cells: 1000,
//!     },
//! )
//! .expect("the far end is reachable");
//! assert!(!path.is_empty());
//! ```

use std::collections::HashMap;

use crate::rng::SplitMix64;
use crate::sim::Simulation;

/// Named sub-stream for exploration, so cell selection and input sampling
/// neither consume from nor correlate with a simulation's own seeded streams.
const EXPLORE_STREAM: u64 = 0x0045_5850_4C4F_5245; // "EXPLORE"

/// Selection weight of a never-explored cell. A cell explored from `n` times
/// gets `SELECT_BASE / (n + 1)`, floored at 1.
const SELECT_BASE: u32 = 1024;

/// Knobs for [`explore`] and [`explore_until`].
///
/// `iterations` × `steps_per_iteration` bounds the total simulation steps;
/// `max_cells` bounds memory, since the archive holds one state clone per
/// cell. Deeper `steps_per_iteration` pushes further per return, more
/// `iterations` spreads effort across more of the frontier — the paper's
/// exploration/exploitation dial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExploreConfig {
    /// Seed for cell selection and input sampling. The seed alone reproduces
    /// the entire exploration exactly.
    pub seed: u64,
    /// How many times to return to an archived cell and explore from it.
    pub iterations: usize,
    /// How many random inputs to apply per return.
    pub steps_per_iteration: usize,
    /// Upper bound on archived cells. Exploration stops once reached; the
    /// start cell is always archived regardless.
    pub max_cells: usize,
}

impl Default for ExploreConfig {
    /// Seed 0, 256 iterations × 16 steps, 4096 cells — a few thousand steps,
    /// sized for a unit test rather than an overnight run.
    fn default() -> Self {
        ExploreConfig {
            seed: 0,
            iterations: 256,
            steps_per_iteration: 16,
            max_cells: 4096,
        }
    }
}

struct Entry<S, I> {
    cell: u64,
    path: Vec<I>,
    state: S,
    selected: u32,
}

/// The set of cells discovered by [`explore`], each with a representative
/// state and the shortest input sequence found to reach it.
///
/// Every stored path is a genuine replay: applying it to the original start
/// state reproduces the stored state exactly. Cells are held in **discovery
/// order** — the internal hash map is only ever used for lookup, never
/// iterated, so nothing here depends on hash-map ordering.
pub struct Archive<S, I> {
    entries: Vec<Entry<S, I>>,
    index: HashMap<u64, usize>,
}

impl<S, I> Archive<S, I> {
    fn new() -> Self {
        Archive {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// File `(cell, path, state)`: insert it if the cell is new, or replace
    /// the stored representative if this path is strictly shorter than the
    /// one already recorded. Returns whether the cell was newly discovered.
    ///
    /// The selection counter survives replacement — it counts how often the
    /// *cell* has been explored from, which is a property of the cell, not of
    /// whichever representative currently stands for it.
    fn offer(&mut self, cell: u64, path: Vec<I>, state: S) -> bool {
        match self.index.get(&cell) {
            Some(&idx) => {
                if path.len() < self.entries[idx].path.len() {
                    self.entries[idx].path = path;
                    self.entries[idx].state = state;
                }
                false
            }
            None => {
                self.index.insert(cell, self.entries.len());
                self.entries.push(Entry {
                    cell,
                    path,
                    state,
                    selected: 0,
                });
                true
            }
        }
    }

    /// Number of distinct cells discovered.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no cells are archived. Never true for an archive returned by
    /// [`explore`], which always files the start cell.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether `cell` was discovered.
    pub fn contains(&self, cell: u64) -> bool {
        self.index.contains_key(&cell)
    }

    /// The shortest input sequence found that reaches `cell`, or `None` if it
    /// was never discovered. Replaying it from the start state reproduces the
    /// corresponding [`state_at`](Self::state_at).
    pub fn path_to(&self, cell: u64) -> Option<&[I]> {
        self.index
            .get(&cell)
            .map(|&idx| self.entries[idx].path.as_slice())
    }

    /// The representative state stored for `cell`.
    pub fn state_at(&self, cell: u64) -> Option<&S> {
        self.index.get(&cell).map(|&idx| &self.entries[idx].state)
    }

    /// How many times exploration returned to `cell`. Low counts on a
    /// long-lived archive mark the parts of the space that were sampled least.
    pub fn selections(&self, cell: u64) -> Option<u32> {
        self.index.get(&cell).map(|&idx| self.entries[idx].selected)
    }

    /// Every `(cell, shortest path)` pair, in discovery order.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &[I])> + '_ {
        self.entries.iter().map(|e| (e.cell, e.path.as_slice()))
    }

    /// Every cell key, in discovery order.
    pub fn cells(&self) -> impl Iterator<Item = u64> + '_ {
        self.entries.iter().map(|e| e.cell)
    }

    /// The cell whose shortest known path is longest — the deepest point
    /// exploration reached. Ties break toward the earlier discovery.
    pub fn deepest(&self) -> Option<(u64, &[I])> {
        self.entries
            .iter()
            .enumerate()
            .max_by_key(|(i, e)| (e.path.len(), std::cmp::Reverse(*i)))
            .map(|(_, e)| (e.cell, e.path.as_slice()))
    }
}

/// Explore the state space reachable from `start`, returning the [`Archive`]
/// of everything found — the Go-Explore loop described in the module docs.
///
/// - `step(state, input) -> S` is the pure transition function, the same
///   shape [`plan_inputs`](crate::plan::plan_inputs) and
///   [`resimulate`](crate::replay::resimulate) use.
/// - `cell_of(state) -> u64` assigns a state to a cell; pass
///   [`hash_state`](crate::world_hash::hash_state) for one cell per distinct
///   state.
///
/// Cell selection prefers cells that have been explored from least often
/// (weight `SELECT_BASE / (selections + 1)`), so effort spreads over the
/// frontier instead of pooling in the first-found region — but the weight is
/// floored at 1, so no cell is ever permanently abandoned. That floor is what
/// keeps the algorithm free of the detachment it exists to avoid.
///
/// Exploration is fully determined by `config.seed`: the same arguments
/// always produce the same archive, and every path in it is replayable.
pub fn explore<S, I, F, C>(
    start: &S,
    inputs: &[I],
    step: F,
    cell_of: C,
    config: &ExploreConfig,
) -> Archive<S, I>
where
    S: Clone,
    I: Clone,
    F: Fn(&S, &I) -> S,
    C: Fn(&S) -> u64,
{
    run(start, inputs, step, cell_of, |_: &S| false, config).0
}

/// Explore as [`explore`] does, but stop the moment a state satisfying `goal`
/// is reached, returning the input sequence that reaches it.
///
/// This is the scalable counterpart to
/// [`plan_inputs`](crate::plan::plan_inputs): that answers "what is the
/// *shortest* way to reach this state, or is it provably unreachable within
/// the bound?" and pays exponentially for the guarantee; this answers "here is
/// *a* way to reach it" on state spaces far past where breadth-first search
/// stalls. `None` means only that exploration did not find one within the
/// configured budget — never that the goal is unreachable.
///
/// Returns `Some(vec![])` when `start` already satisfies `goal`. The returned
/// sequence is a plain input list, so it can be minimised with
/// [`shrink_inputs`](crate::shrink::shrink_inputs) and pinned as a regression
/// test.
pub fn explore_until<S, I, F, C, G>(
    start: &S,
    inputs: &[I],
    step: F,
    cell_of: C,
    goal: G,
    config: &ExploreConfig,
) -> Option<Vec<I>>
where
    S: Clone,
    I: Clone,
    F: Fn(&S, &I) -> S,
    C: Fn(&S) -> u64,
    G: Fn(&S) -> bool,
{
    run(start, inputs, step, cell_of, goal, config).1
}

fn run<S, I, F, C, G>(
    start: &S,
    inputs: &[I],
    step: F,
    cell_of: C,
    goal: G,
    config: &ExploreConfig,
) -> (Archive<S, I>, Option<Vec<I>>)
where
    S: Clone,
    I: Clone,
    F: Fn(&S, &I) -> S,
    C: Fn(&S) -> u64,
    G: Fn(&S) -> bool,
{
    let mut archive = Archive::new();
    archive.offer(cell_of(start), Vec::new(), start.clone());
    if goal(start) {
        return (archive, Some(Vec::new()));
    }
    // `below` takes a u32 bound; clamp a pathological input count rather than
    // truncate it into a different (silently wrong) bound.
    let input_bound = inputs.len().min(u32::MAX as usize) as u32;
    if input_bound == 0 {
        return (archive, None);
    }

    let mut rng = SplitMix64::new(config.seed).split(EXPLORE_STREAM);
    let mut weights: Vec<u32> = Vec::new();

    for _ in 0..config.iterations {
        if archive.len() >= config.max_cells {
            break;
        }

        // Return: pick an archived cell, biased toward the least-explored.
        weights.clear();
        weights.extend(
            archive
                .entries
                .iter()
                .map(|e| (SELECT_BASE / (e.selected + 1)).max(1)),
        );
        let idx = match rng.weighted_index(&weights) {
            Some(i) => i,
            None => break,
        };
        archive.entries[idx].selected = archive.entries[idx].selected.saturating_add(1);
        let mut state = archive.entries[idx].state.clone();
        let mut path = archive.entries[idx].path.clone();

        // Explore: random inputs from there, filing everything new.
        for _ in 0..config.steps_per_iteration {
            let input = &inputs[rng.below(input_bound) as usize];
            state = step(&state, input);
            path.push(input.clone());
            archive.offer(cell_of(&state), path.clone(), state.clone());
            if goal(&state) {
                return (archive, Some(path));
            }
            if archive.len() >= config.max_cells {
                break;
            }
        }
    }

    (archive, None)
}

/// [`explore`] for a [`Simulation`], deriving the pure `Fn(&S, &I) -> S`
/// transition from `Clone` + [`Simulation::step`] so the simulation is not
/// written a second way.
pub fn explore_sim<S, C>(
    start: &S,
    inputs: &[S::Input],
    cell_of: C,
    config: &ExploreConfig,
) -> Archive<S, S::Input>
where
    S: Simulation + Clone,
    S::Input: Clone,
    C: Fn(&S) -> u64,
{
    explore(start, inputs, step_of::<S>, cell_of, config)
}

/// [`explore_until`] for a [`Simulation`] — search for an input sequence
/// driving `start` into a state satisfying `goal`.
pub fn explore_sim_until<S, C, G>(
    start: &S,
    inputs: &[S::Input],
    cell_of: C,
    goal: G,
    config: &ExploreConfig,
) -> Option<Vec<S::Input>>
where
    S: Simulation + Clone,
    S::Input: Clone,
    C: Fn(&S) -> u64,
    G: Fn(&S) -> bool,
{
    explore_until(start, inputs, step_of::<S>, cell_of, goal, config)
}

fn step_of<S>(state: &S, input: &S::Input) -> S
where
    S: Simulation + Clone,
{
    let mut next = state.clone();
    next.step(input);
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::plan_inputs;
    use crate::prop::forall_states;
    use crate::world_hash::hash_state;
    use crate::world_hash::{DetHash, Fnv1a};

    /// A corridor of `len` cells. `+1`/`-1` move, clamped at both ends, so a
    /// random walk needs ~len^2 steps to reach the far end while archived
    /// exploration needs ~len.
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct Corridor {
        at: i32,
        len: i32,
    }

    impl DetHash for Corridor {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_i32(self.at);
            h.write_i32(self.len);
        }
    }

    impl Simulation for Corridor {
        type Input = i32;
        fn step(&mut self, input: &i32) {
            self.at = (self.at + input).clamp(0, self.len);
        }
    }

    fn corridor(len: i32) -> Corridor {
        Corridor { at: 0, len }
    }

    fn step(s: &Corridor, i: &i32) -> Corridor {
        let mut next = s.clone();
        next.step(i);
        next
    }

    fn cfg(seed: u64, iterations: usize, steps: usize) -> ExploreConfig {
        ExploreConfig {
            seed,
            iterations,
            steps_per_iteration: steps,
            max_cells: 100_000,
        }
    }

    #[test]
    fn test_every_archived_path_replays_to_its_state() {
        // The archive's core claim: each stored path, replayed from the start
        // state, reproduces the stored state. Checked against the transition
        // function itself rather than trusting the bookkeeping.
        let start = corridor(30);
        let archive = explore(&start, &[1, -1], step, hash_state, &cfg(1, 200, 12));
        assert!(archive.len() > 1, "exploration should discover cells");
        for (cell, path) in archive.iter() {
            let mut replayed = start.clone();
            for input in path {
                replayed = step(&replayed, input);
            }
            assert_eq!(hash_state(&replayed), cell, "path does not reach its cell");
            assert_eq!(
                &replayed,
                archive.state_at(cell).unwrap(),
                "stored state disagrees with replay"
            );
        }
    }

    #[test]
    fn test_no_archived_path_beats_bfs_shortest() {
        // A stored path can be longer than optimal (the search is incomplete),
        // but never shorter than the true shortest — that would mean the
        // archive invented a transition. BFS supplies the oracle.
        let start = corridor(12);
        let archive = explore(&start, &[1, -1], step, hash_state, &cfg(2, 150, 10));
        for (cell, path) in archive.iter() {
            let shortest = plan_inputs(
                start.clone(),
                &[1, -1],
                step,
                |s: &Corridor| hash_state(s) == cell,
                10_000,
            )
            .expect("archived cells are reachable by construction");
            assert!(
                path.len() >= shortest.len(),
                "archived path shorter than BFS optimum: {} < {}",
                path.len(),
                shortest.len()
            );
        }
    }

    #[test]
    fn test_finds_deep_state_that_memoryless_search_misses() {
        // Go-Explore's central claim, machine-checked: with a comparable step
        // budget, archived exploration reaches a state that memoryless random
        // play does not, because it resumes from the frontier instead of
        // restarting at the origin every time.
        const LEN: i32 = 60;
        let start = corridor(LEN);
        let goal = |s: &Corridor| s.at == LEN;

        // Archived: 400 returns x 16 steps.
        let found = explore_until(&start, &[1, -1], step, hash_state, goal, &cfg(3, 400, 16));
        assert!(found.is_some(), "archived exploration should reach the end");

        // Memoryless: 400 fresh sequences of 16 inputs each — same budget, no
        // archive. Reaching the far end needs 60 correct guesses in a row.
        let memoryless = forall_states(
            0..400u64,
            16,
            &start,
            |rng: &mut SplitMix64| {
                if rng.below(2) == 0 {
                    1
                } else {
                    -1
                }
            },
            goal,
        );
        assert!(
            memoryless.is_ok(),
            "memoryless search should not reach depth {LEN} on this budget"
        );
    }

    #[test]
    fn test_deterministic_for_a_fixed_seed() {
        let start = corridor(25);
        let a = explore(&start, &[1, -1], step, hash_state, &cfg(9, 120, 8));
        let b = explore(&start, &[1, -1], step, hash_state, &cfg(9, 120, 8));
        assert_eq!(a.len(), b.len());
        assert_eq!(a.cells().collect::<Vec<_>>(), b.cells().collect::<Vec<_>>());
        for (cell, path) in a.iter() {
            assert_eq!(Some(path), b.path_to(cell));
            assert_eq!(a.selections(cell), b.selections(cell));
        }
    }

    #[test]
    fn test_different_seeds_explore_differently() {
        // Not a correctness requirement, but a seed that changed nothing would
        // mean the seed is not actually driving the search.
        let start = corridor(40);
        let a = explore(&start, &[1, -1], step, hash_state, &cfg(1, 60, 6));
        let b = explore(&start, &[1, -1], step, hash_state, &cfg(2, 60, 6));
        let a_paths: Vec<usize> = a.iter().map(|(_, p)| p.len()).collect();
        let b_paths: Vec<usize> = b.iter().map(|(_, p)| p.len()).collect();
        assert!(a.len() != b.len() || a_paths != b_paths);
    }

    #[test]
    fn test_exploration_does_not_disturb_a_simulation_rng() {
        // Exploration draws only from its own named sub-stream, so a
        // simulation seeded from the same value sees an untouched sequence.
        let mut untouched = SplitMix64::new(77);
        let before: Vec<u64> = (0..8).map(|_| untouched.next_u64()).collect();

        let start = corridor(20);
        let _ = explore(&start, &[1, -1], step, hash_state, &cfg(77, 50, 8));

        let mut after = SplitMix64::new(77);
        let seq: Vec<u64> = (0..8).map(|_| after.next_u64()).collect();
        assert_eq!(before, seq);
    }

    #[test]
    fn test_goal_already_satisfied_returns_empty_path() {
        let start = corridor(10);
        let found = explore_until(
            &start,
            &[1, -1],
            step,
            hash_state,
            |s: &Corridor| s.at == 0,
            &cfg(0, 10, 4),
        );
        assert_eq!(found, Some(Vec::new()));
    }

    #[test]
    fn test_empty_inputs_archives_only_the_start() {
        let start = corridor(10);
        let archive = explore(&start, &[], step, hash_state, &cfg(0, 50, 8));
        assert_eq!(archive.len(), 1);
        assert!(archive.contains(hash_state(&start)));
        assert_eq!(archive.path_to(hash_state(&start)), Some(&[][..]));
    }

    #[test]
    fn test_zero_iterations_archives_only_the_start() {
        let start = corridor(10);
        let archive = explore(&start, &[1, -1], step, hash_state, &cfg(0, 0, 8));
        assert_eq!(archive.len(), 1);
        assert!(!archive.is_empty());
    }

    #[test]
    fn test_max_cells_bounds_the_archive() {
        let start = corridor(200);
        let config = ExploreConfig {
            seed: 5,
            iterations: 500,
            steps_per_iteration: 16,
            max_cells: 12,
        };
        let archive = explore(&start, &[1, -1], step, hash_state, &config);
        // The bound is checked between steps, so the archive can overshoot by
        // at most the one cell discovered on the step that crosses it.
        assert!(
            archive.len() >= 12 && archive.len() <= 13,
            "{}",
            archive.len()
        );
    }

    #[test]
    fn test_unreachable_goal_returns_none_without_hanging() {
        let start = corridor(10);
        let found = explore_until(
            &start,
            &[1, -1],
            step,
            hash_state,
            |s: &Corridor| s.at == 999, // clamped away — unreachable
            &cfg(4, 100, 8),
        );
        assert_eq!(found, None);
    }

    #[test]
    fn test_coarse_cells_collapse_states_but_keep_paths_valid() {
        // Down-sampling: bucket the corridor into groups of 10, with a walk
        // long enough to cross a bucket (see the sizing rule in the module
        // docs). Fewer cells than distinct states, and every stored path still
        // replays into the cell it claims.
        let start = corridor(90);
        let coarse = |s: &Corridor| (s.at / 10) as u64;
        let archive = explore(&start, &[1, -1], step, coarse, &cfg(6, 300, 24));
        assert!(
            archive.len() <= 10,
            "at most 10 buckets, got {}",
            archive.len()
        );
        assert!(archive.len() > 1, "should reach past the first bucket");
        for (cell, path) in archive.iter() {
            let mut replayed = start.clone();
            for input in path {
                replayed = step(&replayed, input);
            }
            assert_eq!(coarse(&replayed), cell);
        }
    }

    #[test]
    fn test_walk_shorter_than_the_cell_stalls_exploration() {
        // The other side of the sizing rule, pinned. Because the archive keeps
        // the *shortest*-path representative, a return lands at the cell's
        // entry edge; a walk shorter than the cell's diameter can never cross
        // it, so exploration spends its whole budget inside one cell. This is
        // a documented consequence of the representative rule, not a bug —
        // if that rule ever changes, this test is where it surfaces.
        let start = corridor(90);
        let too_coarse = |s: &Corridor| (s.at / 10) as u64;
        let archive = explore(&start, &[1, -1], step, too_coarse, &cfg(6, 300, 8));
        assert_eq!(archive.len(), 1, "expected the documented stall");
        assert_eq!(
            archive.selections(too_coarse(&start)),
            Some(300),
            "the whole budget should be recorded on the single stalled cell"
        );

        // Same cells, same seed, a walk longer than the bucket: progress.
        let wider = explore(&start, &[1, -1], step, too_coarse, &cfg(6, 300, 24));
        assert!(wider.len() > 1, "a walk past the cell width should escape");
    }

    #[test]
    fn test_coarse_cells_keep_the_shortest_representative() {
        // Two states in one cell: the archive must keep the one reached by the
        // shorter path, not whichever arrived last.
        let start = corridor(30);
        let coarse = |s: &Corridor| (s.at / 5) as u64;
        let archive = explore(&start, &[1, -1], step, coarse, &cfg(8, 300, 10));
        for (cell, path) in archive.iter() {
            let mut replayed = start.clone();
            for input in path {
                replayed = step(&replayed, input);
            }
            // Reaching bucket k requires at least 5k moves; the stored
            // representative must respect that floor.
            assert!(path.len() >= (cell as usize) * 5);
            assert_eq!(coarse(&replayed), cell);
        }
    }

    #[test]
    fn test_deepest_reports_the_longest_path() {
        let start = corridor(50);
        let archive = explore(&start, &[1, -1], step, hash_state, &cfg(7, 250, 12));
        let (_, deepest) = archive.deepest().expect("non-empty");
        for (_, path) in archive.iter() {
            assert!(path.len() <= deepest.len());
        }
    }

    #[test]
    fn test_selections_accumulate_and_spread() {
        let start = corridor(15);
        let archive = explore(&start, &[1, -1], step, hash_state, &cfg(11, 200, 6));
        let total: u32 = archive
            .cells()
            .map(|c| archive.selections(c).unwrap())
            .sum();
        assert_eq!(total, 200, "every iteration selects exactly one cell");
        // The weight floor keeps every cell selectable, so effort should not
        // pool entirely in one place.
        let explored_from = archive
            .cells()
            .filter(|&c| archive.selections(c).unwrap() > 0)
            .count();
        assert!(explored_from > 1, "selection collapsed onto a single cell");
    }

    #[test]
    fn test_simulation_adapters_match_the_closure_form() {
        let start = corridor(35);
        let closure = explore(&start, &[1, -1], step, hash_state, &cfg(13, 150, 10));
        let adapted = explore_sim(&start, &[1, -1], hash_state, &cfg(13, 150, 10));
        assert_eq!(
            closure.cells().collect::<Vec<_>>(),
            adapted.cells().collect::<Vec<_>>()
        );

        let goal = |s: &Corridor| s.at == 35;
        assert_eq!(
            explore_until(&start, &[1, -1], step, hash_state, goal, &cfg(13, 300, 10)),
            explore_sim_until(&start, &[1, -1], hash_state, goal, &cfg(13, 300, 10))
        );
    }

    #[test]
    fn test_found_path_is_shrinkable_into_a_minimal_regression_test() {
        // The output is a plain input list, so it feeds the rest of the
        // pipeline: shrink the found path down to a minimal one that still
        // reaches the goal.
        use crate::shrink::{is_one_minimal, shrink_inputs};

        const LEN: i32 = 40;
        let start = corridor(LEN);
        let reaches_end = |inputs: &[i32]| {
            let mut s = start.clone();
            for i in inputs {
                s = step(&s, i);
            }
            s.at == LEN
        };
        let path = explore_until(
            &start,
            &[1, -1],
            step,
            hash_state,
            |s: &Corridor| s.at == LEN,
            &cfg(21, 400, 16),
        )
        .expect("reachable");
        assert!(reaches_end(&path));

        let minimal = shrink_inputs(&path, reaches_end);
        assert!(reaches_end(&minimal));
        assert!(is_one_minimal(&minimal, reaches_end));
        // The corridor needs exactly LEN forward moves and nothing else.
        assert_eq!(minimal.len(), LEN as usize);
        assert!(minimal.iter().all(|&i| i == 1));
    }
}
