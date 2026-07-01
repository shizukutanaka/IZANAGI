//! Deterministic multi-player input prediction and misprediction detection.
//!
//! [`replay`](crate::replay) proved that rollback ("snapshot a known-good
//! tick, re-run newer inputs") works, and [`cmdqueue`](crate::cmdqueue) gives
//! a single local player a deterministic input feed — but nothing addressed
//! what a real network session needs before either is useful: **multiple
//! players' inputs for the same tick arrive at different times.** A lockstep
//! or rollback sim cannot simply wait for every peer's input before every
//! tick (that stalls on the slowest connection), so it must **predict** a
//! not-yet-arrived input (the standard technique: repeat that player's last
//! known input) and keep simulating, then detect when the real input finally
//! arrives and **disagrees** with the guess — the signal to roll back and
//! resimulate via [`replay::resimulate`](crate::replay::resimulate).
//!
//! [`NetInputBuffer`] is that missing piece: transport-agnostic (it never
//! touches a socket — feeding it received bytes is the caller's job, matching
//! this kit's headless/zero-dependency scope) tracking of which `(tick,
//! player)` inputs are confirmed ground truth, which are still predictions,
//! and whether a given confirmation contradicts what was already predicted.
//!
//! ```
//! use izanagi_kit::netinput::NetInputBuffer;
//!
//! let mut buf: NetInputBuffer<u8, i32> = NetInputBuffer::new();
//! buf.seed(1, 0);   // player 1's baseline input before anything is confirmed
//!
//! // Tick 5 arrives locally before player 1's real input for tick 5 does —
//! // predict it (repeats the seeded/last-known value).
//! assert_eq!(buf.input_for(5, 1), Some(0));
//! assert!(buf.is_predicted(5, 1));
//!
//! // The real input finally arrives and turns out to differ from the guess.
//! let mispredicted = buf.confirm(5, 1, 7);
//! assert!(mispredicted, "7 != the predicted 0 — the caller must resimulate from tick 5");
//! assert_eq!(buf.confirmed_input(5, 1), Some(&7));
//! assert!(!buf.is_predicted(5, 1), "no longer a guess once confirmed");
//! ```
//!
//! ## Design
//!
//! - `confirmed: BTreeMap<(tick, player), input>` is the ground truth once
//!   known; `last_known: BTreeMap<player, input>` is each player's most recent
//!   confirmed input, the source for future predictions; `predicted:
//!   BTreeMap<(tick, player), input>` memoizes what [`input_for`] guessed, so
//!   [`confirm`](NetInputBuffer::confirm) can compare against it later.
//! - **`predicted` is deliberately excluded from [`DetHash`](crate::world_hash::DetHash).**
//!   Two peers with the same confirmed history but different network timing
//!   will have predicted different (still-unconfirmed) values at the moment
//!   you happen to hash — including that in the checksum would report a
//!   "divergence" that is really just normal prediction skew, not an actual
//!   simulation bug. Only `confirmed` and `last_known` — the parts guaranteed
//!   to converge once the same confirmations are known — are canonical.
//! - This module makes no networking decisions: it does not send, receive, or
//!   order packets. The caller decides transport and delivery; this only
//!   tracks the predict/confirm bookkeeping and flags mispredictions.

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::BTreeMap;

/// Tracks per-tick, per-player inputs for a deterministic multiplayer
/// simulation: confirmed ground truth, in-flight predictions, and each
/// player's most recently confirmed input (the prediction source).
///
/// `last_known` always resolves to the input at the **highest tick confirmed
/// so far** for that player, tracked via `last_known_tick` — never simply
/// "whichever `confirm` call happened most recently in real time." Real
/// delivery (even within a single process feeding this from a queue) is not
/// guaranteed to hand ticks to `confirm` in increasing order, and if a
/// late-arriving *older* tick were allowed to overwrite a newer one already
/// recorded, predictions would regress to a stale value depending on
/// arrival order alone — the max-tick rule makes the result of any sequence
/// of `confirm` calls order-independent, an associative-fold guarantee in
/// the same spirit as `threat`'s decay or `pool`'s clamped regen.
#[derive(Clone, Debug, Default)]
pub struct NetInputBuffer<P: Ord + Clone, I: Clone + PartialEq> {
    confirmed: BTreeMap<(u32, P), I>,
    predicted: BTreeMap<(u32, P), I>,
    last_known: BTreeMap<P, I>,
    last_known_tick: BTreeMap<P, u32>,
}

impl<P: Ord + Clone, I: Clone + PartialEq> NetInputBuffer<P, I> {
    /// An empty buffer: no confirmed inputs, no predictions, no known players.
    pub fn new() -> Self {
        NetInputBuffer {
            confirmed: BTreeMap::new(),
            predicted: BTreeMap::new(),
            last_known: BTreeMap::new(),
            last_known_tick: BTreeMap::new(),
        }
    }

    /// Seed `player`'s baseline input, used for prediction before their first
    /// real input is confirmed. A no-op if `player` already has a known input
    /// (from an earlier `seed` or `confirm`) — seeding never overwrites
    /// history that has already been established, and any real `confirm`
    /// (at any tick) always supersedes a seed.
    pub fn seed(&mut self, player: P, input: I) {
        self.last_known.entry(player).or_insert(input);
    }

    /// Record `player`'s real input for `tick`. Returns `true` if this
    /// contradicts an input already predicted for that `(tick, player)` — the
    /// signal that the caller must roll back to `tick` and resimulate with
    /// the corrected input (e.g. via [`replay::resimulate`](crate::replay::resimulate)).
    /// Returns `false` if nothing was predicted for that slot (the input
    /// arrived before it was ever needed) or if the prediction happened to be
    /// correct. Overwrites any earlier confirmation for the same
    /// `(tick, player)` (idempotent re-confirmation still compares against
    /// the original prediction, not the value being written). Updates
    /// `player`'s `last_known` input only if `tick` is at or after the
    /// highest tick already confirmed for that player, so out-of-order
    /// arrival can never regress a prediction to a stale value.
    pub fn confirm(&mut self, tick: u32, player: P, input: I) -> bool {
        let mispredicted = self
            .predicted
            .get(&(tick, player.clone()))
            .map(|pred| pred != &input)
            .unwrap_or(false);
        self.confirmed.insert((tick, player.clone()), input.clone());
        let is_newest = match self.last_known_tick.get(&player) {
            Some(&best) => tick >= best,
            None => true,
        };
        if is_newest {
            self.last_known_tick.insert(player.clone(), tick);
            self.last_known.insert(player, input);
        }
        mispredicted
    }

    /// The input to actually simulate `player` with at `tick`: the confirmed
    /// value if known, otherwise a prediction (that player's `last_known`
    /// input), memoized so a later [`confirm`](Self::confirm) can detect a
    /// misprediction. Returns `None` if `player` has neither a confirmed
    /// input at `tick` nor any prior seed/confirmation to predict from.
    /// Repeated calls for the same `(tick, player)` before it is confirmed
    /// return the same predicted value (idempotent).
    pub fn input_for(&mut self, tick: u32, player: P) -> Option<I> {
        if let Some(v) = self.confirmed.get(&(tick, player.clone())) {
            return Some(v.clone());
        }
        if let Some(p) = self.predicted.get(&(tick, player.clone())) {
            return Some(p.clone());
        }
        let guess = self.last_known.get(&player)?.clone();
        self.predicted.insert((tick, player), guess.clone());
        Some(guess)
    }

    /// The confirmed (ground-truth) input at `(tick, player)`, or `None` if
    /// not yet confirmed — never predicts, unlike [`input_for`](Self::input_for).
    pub fn confirmed_input(&self, tick: u32, player: P) -> Option<&I> {
        self.confirmed.get(&(tick, player))
    }

    /// `true` if `(tick, player)` currently holds a prediction that has not
    /// yet been confirmed (i.e. the value [`input_for`] would return there is
    /// still a guess, not ground truth).
    pub fn is_predicted(&self, tick: u32, player: P) -> bool {
        self.predicted.contains_key(&(tick, player.clone())) && !self.confirmed.contains_key(&(tick, player))
    }

    /// `true` if every player in `players` has a confirmed input at `tick`
    /// (vacuously `true` for an empty slice).
    pub fn is_confirmed_for(&self, tick: u32, players: &[P]) -> bool {
        players
            .iter()
            .all(|p| self.confirmed.contains_key(&(tick, p.clone())))
    }

    /// The number of confirmed `(tick, player)` entries currently held.
    pub fn len_confirmed(&self) -> usize {
        self.confirmed.len()
    }

    /// Discard confirmed and predicted entries at ticks strictly before
    /// `tick` — a "we will never roll back past this point" horizon (e.g.
    /// once every peer has acknowledged it). `last_known` is untouched: it is
    /// per-player *current* state, not tick-indexed history.
    pub fn prune_before(&mut self, tick: u32) {
        self.confirmed.retain(|&(t, _), _| t >= tick);
        self.predicted.retain(|&(t, _), _| t >= tick);
    }
}

impl<P: Ord + Clone + DetHash, I: Clone + PartialEq + DetHash> DetHash for NetInputBuffer<P, I> {
    /// Folds `confirmed` and `last_known` only — see the module docs for why
    /// `predicted` (provisional, expected to differ across peers before
    /// confirmation) is deliberately excluded from the canonical checksum.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.confirmed.len() as u32);
        for (&(tick, ref player), input) in &self.confirmed {
            hasher.write_u32(tick);
            player.det_hash(hasher);
            input.det_hash(hasher);
        }
        hasher.write_u32(self.last_known.len() as u32);
        for (player, input) in &self.last_known {
            player.det_hash(hasher);
            input.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        assert_eq!(buf.len_confirmed(), 0);
        assert_eq!(buf.confirmed_input(0, 1), None);
        assert_eq!(buf.input_for(0, 1), None, "no seed, no confirm → nothing to predict");
    }

    #[test]
    fn test_seed_enables_prediction_before_any_confirm() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 42);
        assert_eq!(buf.input_for(5, 1), Some(42));
        assert!(buf.is_predicted(5, 1));
    }

    #[test]
    fn test_seed_does_not_overwrite_existing_last_known() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(0, 1, 10);
        buf.seed(1, 999); // must not clobber the real, already-known input
        assert_eq!(buf.input_for(1, 1), Some(10));
    }

    #[test]
    fn test_confirm_before_prediction_is_never_a_misprediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        let mispredicted = buf.confirm(3, 1, 7);
        assert!(!mispredicted, "arriving on time, nothing was guessed");
        assert_eq!(buf.confirmed_input(3, 1), Some(&7));
    }

    /// A late, out-of-order confirmation for an *older* tick must not clobber
    /// `last_known` with a stale value — `last_known` always tracks the
    /// highest tick confirmed so far, regardless of the order `confirm` was
    /// called in. This is the property that makes `NetInputBuffer` safe to
    /// feed from a real (possibly reordering) network transport.
    #[test]
    fn test_out_of_order_confirm_does_not_regress_last_known() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(10, 1, 100); // the newer tick arrives first
        buf.confirm(3, 1, 999); // an older tick arrives late
        assert_eq!(
            buf.input_for(20, 1),
            Some(100),
            "last_known must still reflect tick 10's value, not the late tick 3"
        );
    }

    #[test]
    fn test_input_for_prefers_confirmed_over_predicted() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 0);
        buf.confirm(5, 1, 99);
        assert_eq!(buf.input_for(5, 1), Some(99), "confirmed value wins even though a prediction exists");
    }

    #[test]
    fn test_input_for_memoizes_prediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 5);
        assert_eq!(buf.input_for(10, 1), Some(5));
        // Even if last_known later changes (a different tick's confirm), the
        // already-made prediction for tick 10 must not silently change.
        buf.confirm(11, 1, 999);
        assert_eq!(buf.input_for(10, 1), Some(5), "memoized prediction is stable");
    }

    #[test]
    fn test_confirm_matching_prediction_is_not_a_misprediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 3);
        assert_eq!(buf.input_for(2, 1), Some(3));
        let mispredicted = buf.confirm(2, 1, 3); // matches the guess exactly
        assert!(!mispredicted);
    }

    #[test]
    fn test_confirm_differing_from_prediction_is_a_misprediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 3);
        assert_eq!(buf.input_for(2, 1), Some(3));
        let mispredicted = buf.confirm(2, 1, 8);
        assert!(mispredicted);
    }

    #[test]
    fn test_is_predicted_false_before_any_input_for_call() {
        let buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        assert!(!buf.is_predicted(0, 1));
    }

    #[test]
    fn test_is_predicted_becomes_false_after_confirm() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 0);
        buf.input_for(5, 1);
        assert!(buf.is_predicted(5, 1));
        buf.confirm(5, 1, 0);
        assert!(!buf.is_predicted(5, 1), "confirmed slot is ground truth, not a prediction");
    }

    #[test]
    fn test_is_confirmed_for_all_players() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(1, 10, 0);
        buf.confirm(1, 20, 0);
        assert!(buf.is_confirmed_for(1, &[10, 20]));
        assert!(!buf.is_confirmed_for(1, &[10, 20, 30]), "player 30 unconfirmed");
    }

    #[test]
    fn test_is_confirmed_for_empty_players_is_vacuously_true() {
        let buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        assert!(buf.is_confirmed_for(0, &[]));
    }

    #[test]
    fn test_len_confirmed_counts_distinct_tick_player_pairs() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(0, 1, 0);
        buf.confirm(0, 2, 0);
        buf.confirm(1, 1, 0);
        assert_eq!(buf.len_confirmed(), 3);
        buf.confirm(0, 1, 99); // overwrite, not a new pair
        assert_eq!(buf.len_confirmed(), 3);
    }

    #[test]
    fn test_prune_before_removes_old_ticks_only() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(0, 1, 10);
        buf.confirm(5, 1, 20);
        buf.confirm(10, 1, 30);
        buf.prune_before(5);
        assert_eq!(buf.confirmed_input(0, 1), None, "pruned");
        assert_eq!(buf.confirmed_input(5, 1), Some(&20), "boundary tick kept");
        assert_eq!(buf.confirmed_input(10, 1), Some(&30), "newer tick kept");
    }

    #[test]
    fn test_prune_before_does_not_touch_last_known() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(0, 1, 10);
        buf.prune_before(100);
        assert_eq!(buf.input_for(200, 1), Some(10), "last_known survives pruning of old ticks");
    }

    #[test]
    fn test_multiple_players_independent() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 100);
        buf.seed(2, 200);
        assert_eq!(buf.input_for(0, 1), Some(100));
        assert_eq!(buf.input_for(0, 2), Some(200));
        buf.confirm(0, 1, 111);
        assert_eq!(buf.confirmed_input(0, 1), Some(&111));
        assert_eq!(buf.confirmed_input(0, 2), None, "player 2 unaffected by player 1's confirm");
    }

    #[test]
    fn test_reconfirm_same_slot_is_idempotent_against_original_prediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 0);
        buf.input_for(3, 1); // predicts 0
        let first = buf.confirm(3, 1, 5); // mispredicted (0 != 5)
        let second = buf.confirm(3, 1, 5); // re-confirming the same value
        assert!(first);
        assert!(second, "still compared against the original prediction of 0, not the new value");
    }

    #[test]
    fn test_det_hash_excludes_predicted_state() {
        let mut a: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        a.confirm(0, 1, 10);
        a.seed(2, 999);
        // Clone after seeding, so both share identical confirmed + last_known;
        // only `a` will go on to materialize a predicted-map entry.
        let b = a.clone();

        a.input_for(5, 2); // records a prediction in `a` only

        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "a and b differ only in predicted state, which must not affect the hash"
        );
    }

    #[test]
    fn test_det_hash_sensitive_to_confirmed_value() {
        let mut a: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        a.confirm(0, 1, 10);
        let mut b: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        b.confirm(0, 1, 11);
        assert_ne!(hash_state(&a), hash_state(&b), "different confirmed input → different hash");
    }

    #[test]
    fn test_det_hash_order_independent() {
        let mut a: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        a.confirm(0, 1, 10);
        a.confirm(1, 2, 20);
        let mut b: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        b.confirm(1, 2, 20);
        b.confirm(0, 1, 10);
        assert_eq!(hash_state(&a), hash_state(&b), "confirmation order does not affect the hash");
    }
}
