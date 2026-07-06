//! Quest and objective tracking for RPG-flavoured roguelikes.
//!
//! [`progression`](crate::progression) captures *global* character growth
//! (experience → level); [`eventqueue`](crate::eventqueue) carries game events
//! between systems; but nothing tracked *named tasks with completion
//! conditions* — the quest axis. A [`Quest`] bundles one or more
//! [`Objective`]s; each objective has a count target and a current progress
//! counter; the quest as a whole is complete when every objective is.
//!
//! ```
//! use izanagi_kit::quest::{Quest, Objective, QuestState};
//!
//! let mut q = Quest::new("Goblin Hunt")
//!     .with_objective(Objective::new("Kill goblins", 5))
//!     .with_objective(Objective::new("Open the chest", 1));
//!
//! assert_eq!(q.state(), QuestState::Active);
//!
//! q.progress(0, 3);   // kill 3 goblins
//! q.progress(0, 2);   // kill 2 more → objective 0 done
//! q.progress(1, 1);   // open the chest → objective 1 done
//!
//! assert_eq!(q.state(), QuestState::Complete);
//! assert_eq!(q.completed_count(), 2);
//! ```
//!
//! ## Design
//!
//! Progress is **monotone**: `progress(i, n)` adds `n` to objective `i`'s
//! counter (saturating) and never decreases it. Objectives can also be
//! individually **failed** (e.g. NPC escort died); a quest transitions to
//! `QuestState::Failed` once any objective is failed. Quests can be
//! **abandoned** at the quest level to reset everything.
//!
//! [`Quest`] and [`Objective`] implement
//! [`DetHash`](crate::world_hash::DetHash), folding the current progress state
//! into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// The lifecycle state of a quest or objective.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QuestState {
    /// Active and not yet completed or failed.
    Active,
    /// All objectives satisfied.
    Complete,
    /// One or more objectives failed; the quest cannot be completed.
    Failed,
    /// Abandoned by the player before completion.
    Abandoned,
}

impl DetHash for QuestState {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(*self as u8);
    }
}

/// A single trackable objective: a named counter with a target.
#[derive(Clone, Debug)]
pub struct Objective {
    name: String,
    target: u32,
    current: u32,
    state: QuestState,
}

impl Objective {
    /// Create a new active objective with `target` as the completion threshold.
    /// A target of `0` is treated as already complete.
    pub fn new(name: impl Into<String>, target: u32) -> Self {
        let mut obj = Objective {
            name: name.into(),
            target,
            current: 0,
            state: QuestState::Active,
        };
        if target == 0 {
            obj.state = QuestState::Complete;
        }
        obj
    }

    /// The objective's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The count needed to complete this objective.
    pub fn target(&self) -> u32 {
        self.target
    }

    /// Current progress toward the target.
    pub fn current(&self) -> u32 {
        self.current
    }

    /// Current lifecycle state.
    pub fn state(&self) -> QuestState {
        self.state
    }

    /// `true` if the objective is complete (`current >= target`).
    pub fn is_complete(&self) -> bool {
        self.state == QuestState::Complete
    }

    /// `true` if the objective was failed.
    pub fn is_failed(&self) -> bool {
        self.state == QuestState::Failed
    }

    /// Add `amount` to the progress counter (saturating). If `current` reaches
    /// or exceeds `target`, the state transitions to `Complete`. No-op if
    /// already `Complete` or `Failed`.
    pub fn progress(&mut self, amount: u32) {
        if self.state != QuestState::Active {
            return;
        }
        self.current = self.current.saturating_add(amount);
        if self.current >= self.target {
            self.current = self.target; // clamp to target
            self.state = QuestState::Complete;
        }
    }

    /// Mark this objective as failed. No-op if already `Complete` or `Failed`.
    pub fn fail(&mut self) {
        if self.state == QuestState::Active {
            self.state = QuestState::Failed;
        }
    }

    /// Remaining progress needed: `target.saturating_sub(current)`.
    pub fn remaining(&self) -> u32 {
        self.target.saturating_sub(self.current)
    }

    /// Progress as an integer percentage (0–100): `current * 100 / target`.
    /// Returns `100` for a zero-target objective, `0` if target is `u32::MAX`
    /// and current is `0`.
    pub fn percent(&self) -> u32 {
        if self.target == 0 {
            return 100;
        }
        (self.current as u64 * 100 / self.target as u64) as u32
    }
}

impl DetHash for Objective {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(&self.name);
        hasher.write_u32(self.target);
        hasher.write_u32(self.current);
        self.state.det_hash(hasher);
    }
}

/// A quest — a named collection of [`Objective`]s.
///
/// The quest is [`QuestState::Active`] until every objective is
/// [`QuestState::Complete`] (→ `QuestState::Complete`) or any objective is
/// [`QuestState::Failed`] (→ `QuestState::Failed`). Abandoning the quest sets
/// every objective to `QuestState::Abandoned` and the quest to
/// `QuestState::Abandoned`.
#[derive(Clone, Debug)]
pub struct Quest {
    name: String,
    objectives: Vec<Objective>,
}

impl Quest {
    /// Create a quest with no objectives yet.
    pub fn new(name: impl Into<String>) -> Self {
        Quest {
            name: name.into(),
            objectives: Vec::new(),
        }
    }

    /// Add an objective and return `self` for builder chaining.
    pub fn with_objective(mut self, obj: Objective) -> Self {
        self.objectives.push(obj);
        self
    }

    /// The quest's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The number of objectives.
    pub fn objective_count(&self) -> usize {
        self.objectives.len()
    }

    /// Borrow objective `i`, or `None` if out of range.
    pub fn get(&self, i: usize) -> Option<&Objective> {
        self.objectives.get(i)
    }

    /// The overall quest state:
    /// - `Failed` if any objective is failed.
    /// - `Complete` if every objective is complete (and none are failed).
    /// - `Abandoned` if `abandon()` was called.
    /// - `Active` otherwise.
    pub fn state(&self) -> QuestState {
        if self
            .objectives
            .iter()
            .any(|o| o.state == QuestState::Abandoned)
        {
            return QuestState::Abandoned;
        }
        if self.objectives.iter().any(|o| o.is_failed()) {
            return QuestState::Failed;
        }
        if self.objectives.is_empty() || self.objectives.iter().all(|o| o.is_complete()) {
            return QuestState::Complete;
        }
        QuestState::Active
    }

    /// The number of completed objectives.
    pub fn completed_count(&self) -> usize {
        self.objectives.iter().filter(|o| o.is_complete()).count()
    }

    /// The number of failed objectives.
    pub fn failed_count(&self) -> usize {
        self.objectives.iter().filter(|o| o.is_failed()).count()
    }

    /// The number of still-active objectives.
    pub fn active_count(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| o.state() == QuestState::Active)
            .count()
    }

    /// Add `amount` to objective `i`. No-op if `i` is out of range.
    pub fn progress(&mut self, i: usize, amount: u32) {
        if let Some(obj) = self.objectives.get_mut(i) {
            obj.progress(amount);
        }
    }

    /// Fail objective `i`. No-op if out of range.
    pub fn fail_objective(&mut self, i: usize) {
        if let Some(obj) = self.objectives.get_mut(i) {
            obj.fail();
        }
    }

    /// Abandon the quest: mark every objective as `Abandoned`.
    pub fn abandon(&mut self) {
        for obj in &mut self.objectives {
            obj.state = QuestState::Abandoned;
        }
    }

    /// Iterate over all objectives.
    pub fn iter(&self) -> impl Iterator<Item = &Objective> {
        self.objectives.iter()
    }
}

impl DetHash for Quest {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(&self.name);
        hasher.write_u32(self.objectives.len() as u32);
        for obj in &self.objectives {
            obj.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn goblin_quest() -> Quest {
        Quest::new("Goblin Hunt")
            .with_objective(Objective::new("Kill goblins", 5))
            .with_objective(Objective::new("Open the chest", 1))
    }

    #[test]
    fn test_new_quest_is_active() {
        let q = goblin_quest();
        assert_eq!(q.state(), QuestState::Active);
        assert_eq!(q.objective_count(), 2);
        assert_eq!(q.completed_count(), 0);
        assert_eq!(q.active_count(), 2);
    }

    #[test]
    fn test_progress_monotone() {
        let mut q = goblin_quest();
        q.progress(0, 3);
        assert_eq!(q.get(0).unwrap().current(), 3);
        assert_eq!(q.state(), QuestState::Active);
        q.progress(0, 2);
        assert_eq!(q.get(0).unwrap().current(), 5);
        assert!(q.get(0).unwrap().is_complete());
    }

    #[test]
    fn test_complete_when_all_objectives_done() {
        let mut q = goblin_quest();
        q.progress(0, 10); // clamped to target=5
        assert_eq!(q.get(0).unwrap().current(), 5, "current clamped at target");
        q.progress(1, 1);
        assert_eq!(q.state(), QuestState::Complete);
        assert_eq!(q.completed_count(), 2);
    }

    #[test]
    fn test_fail_propagates_to_quest() {
        let mut q = goblin_quest();
        q.progress(0, 3);
        q.fail_objective(1);
        assert_eq!(q.state(), QuestState::Failed);
        assert_eq!(q.failed_count(), 1);
    }

    #[test]
    fn test_complete_objective_cannot_fail() {
        let mut obj = Objective::new("task", 1);
        obj.progress(1);
        assert!(obj.is_complete());
        obj.fail();
        assert_eq!(
            obj.state(),
            QuestState::Complete,
            "complete can't be failed"
        );
    }

    #[test]
    fn test_progress_noop_when_complete_or_failed() {
        let mut obj = Objective::new("task", 3);
        obj.progress(3);
        assert!(obj.is_complete());
        obj.progress(99);
        assert_eq!(obj.current(), 3, "progress after complete is no-op");

        let mut obj2 = Objective::new("task", 5);
        obj2.fail();
        obj2.progress(5);
        assert_eq!(obj2.current(), 0, "progress after fail is no-op");
    }

    #[test]
    fn test_zero_target_starts_complete() {
        let obj = Objective::new("trivial", 0);
        assert!(obj.is_complete());
        assert_eq!(obj.percent(), 100);
    }

    #[test]
    fn test_abandon() {
        let mut q = goblin_quest();
        q.progress(0, 3);
        q.abandon();
        assert_eq!(q.state(), QuestState::Abandoned);
        // Progress after abandon is a no-op.
        q.progress(0, 5);
        assert_eq!(q.get(0).unwrap().current(), 3, "no progress after abandon");
    }

    #[test]
    fn test_objective_partition_invariant() {
        let mut q = goblin_quest();
        q.progress(0, 5); // complete
        let active = q.active_count();
        let completed = q.completed_count();
        let failed = q.failed_count();
        assert_eq!(active + completed + failed, q.objective_count());
    }

    #[test]
    fn test_remaining_and_percent() {
        let mut obj = Objective::new("task", 10);
        obj.progress(3);
        assert_eq!(obj.remaining(), 7);
        assert_eq!(obj.percent(), 30);
        obj.progress(7);
        assert_eq!(obj.remaining(), 0);
        assert_eq!(obj.percent(), 100);
    }

    #[test]
    fn test_out_of_range_progress_noop() {
        let mut q = goblin_quest();
        q.progress(99, 100); // no-op
        assert_eq!(q.state(), QuestState::Active);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = goblin_quest();
        let b = goblin_quest();
        assert_eq!(hash_state(&a), hash_state(&b), "same quest, same hash");
        let mut c = goblin_quest();
        c.progress(0, 1);
        assert_ne!(hash_state(&a), hash_state(&c), "progress must change hash");
    }
}
