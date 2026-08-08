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
//!   BTreeMap<(tick, player), input>` memoizes what [`input_for`](NetInputBuffer::input_for) guessed, so
//!   [`confirm`](NetInputBuffer::confirm) can compare against it later.
//! - **`predicted` is deliberately excluded from [`DetHash`].**
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
use std::collections::{BTreeMap, VecDeque};

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
    /// yet been confirmed (i.e. the value [`Self::input_for`] would return there is
    /// still a guess, not ground truth).
    pub fn is_predicted(&self, tick: u32, player: P) -> bool {
        self.predicted.contains_key(&(tick, player.clone()))
            && !self.confirmed.contains_key(&(tick, player))
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

/// Adaptive input-delay controller: turns [`NetInputBuffer`]'s misprediction
/// signal into a recommended **input delay** (how many frames to buffer local
/// input before simulating it).
///
/// Overwatch's netcode (Tim Ford & Philip Orwig, GDC 2017) tunes this delay to
/// the connection instead of fixing it: a higher delay buffers more input so
/// fewer predictions are ever needed (fewer rollbacks, smoother remote motion)
/// at the cost of local input latency; a lower delay is snappier but rolls
/// back more when the network is jittery. The right value tracks how often
/// predictions are actually wrong.
///
/// Feed each [`confirm`](NetInputBuffer::confirm) result in:
///
/// ```
/// use izanagi_kit::netinput::{NetInputBuffer, AdaptiveDelay};
///
/// let mut buf: NetInputBuffer<u8, i32> = NetInputBuffer::new();
/// let mut delay = AdaptiveDelay::new(2, 8, 4); // delay in [2, 8] frames, 4-sample window
/// buf.seed(1, 0);
/// # let mut tick = 0;
/// # for _ in 0..4 {
/// let guessed = buf.input_for(tick, 1);
/// // ... later, the real input arrives ...
/// let mispredicted = buf.confirm(tick, 1, 0);
/// delay.record(mispredicted);
/// # tick += 1;
/// # }
/// let frames_to_buffer = delay.recommended_delay();
/// ```
///
/// **Determinism & hysteresis.** The controller is integer-only (misprediction
/// rate in parts-per-thousand, no float), so its output is reproducible across
/// targets. It re-evaluates only once its sample window is full, and uses a
/// hysteresis band — the delay rises by one frame when the rate is at/above
/// `raise_permille`, falls by one when at/below `lower_permille`, and holds
/// steady in between — so a rate hovering near one threshold cannot make the
/// delay oscillate every frame. The recommendation is advisory metadata, not
/// simulation state: like [`NetInputBuffer`]'s `predicted` map it should not
/// feed the world hash (two peers with different connections will legitimately
/// choose different delays).
#[derive(Clone, Debug)]
pub struct AdaptiveDelay {
    window: VecDeque<bool>,
    window_cap: usize,
    misses: usize,
    delay: u32,
    min_delay: u32,
    max_delay: u32,
    lower_permille: u32,
    raise_permille: u32,
}

impl AdaptiveDelay {
    /// Default rate (‰) at/below which the delay is lowered by one frame.
    pub const DEFAULT_LOWER_PERMILLE: u32 = 50;
    /// Default rate (‰) at/above which the delay is raised by one frame.
    pub const DEFAULT_RAISE_PERMILLE: u32 = 250;

    /// Controller recommending a delay in `[min_delay, max_delay]`, measuring
    /// the misprediction rate over the most recent `window_cap` samples, with
    /// the default hysteresis thresholds ([`Self::DEFAULT_LOWER_PERMILLE`] /
    /// [`Self::DEFAULT_RAISE_PERMILLE`]). Starts at `min_delay`. Panics if
    /// `min_delay > max_delay` or `window_cap == 0`.
    pub fn new(min_delay: u32, max_delay: u32, window_cap: usize) -> Self {
        Self::with_thresholds(
            min_delay,
            max_delay,
            window_cap,
            Self::DEFAULT_LOWER_PERMILLE,
            Self::DEFAULT_RAISE_PERMILLE,
        )
    }

    /// [`Self::new`] with explicit hysteresis thresholds (parts-per-thousand).
    /// Panics if `min_delay > max_delay`, `window_cap == 0`, or
    /// `lower_permille > raise_permille` (an inverted band).
    pub fn with_thresholds(
        min_delay: u32,
        max_delay: u32,
        window_cap: usize,
        lower_permille: u32,
        raise_permille: u32,
    ) -> Self {
        assert!(min_delay <= max_delay, "min_delay must be <= max_delay");
        assert!(window_cap > 0, "window_cap must be > 0");
        assert!(
            lower_permille <= raise_permille,
            "lower_permille must be <= raise_permille"
        );
        AdaptiveDelay {
            window: VecDeque::with_capacity(window_cap),
            window_cap,
            misses: 0,
            delay: min_delay,
            min_delay,
            max_delay,
            lower_permille,
            raise_permille,
        }
    }

    /// Record one prediction outcome (`true` = the confirmed input contradicted
    /// the prediction — pass the `bool` returned by
    /// [`NetInputBuffer::confirm`]). Once the sample window is full this
    /// re-evaluates the recommended delay: +1 frame if the windowed
    /// misprediction rate is at/above `raise_permille` (and below `max_delay`),
    /// −1 if at/below `lower_permille` (and above `min_delay`), unchanged
    /// otherwise. At most one frame of change per call, so the recommendation
    /// moves smoothly.
    pub fn record(&mut self, mispredicted: bool) {
        // `pop_front` runs only when the window is full (short-circuit), so
        // this eviction is equivalent to the nested form.
        if self.window.len() == self.window_cap && self.window.pop_front() == Some(true) {
            self.misses -= 1;
        }
        self.window.push_back(mispredicted);
        if mispredicted {
            self.misses += 1;
        }

        if self.window.len() == self.window_cap {
            let rate = self.misprediction_permille();
            if rate >= self.raise_permille && self.delay < self.max_delay {
                self.delay += 1;
            } else if rate <= self.lower_permille && self.delay > self.min_delay {
                self.delay -= 1;
            }
        }
    }

    /// The current recommended input delay in frames, always within
    /// `[min_delay, max_delay]`.
    pub fn recommended_delay(&self) -> u32 {
        self.delay
    }

    /// The misprediction rate over the current window, in parts-per-thousand
    /// (`0..=1000`). `0` when no samples have been recorded yet.
    pub fn misprediction_permille(&self) -> u32 {
        if self.window.is_empty() {
            0
        } else {
            (self.misses * 1000 / self.window.len()) as u32
        }
    }

    /// Number of samples currently in the window (`<= window_cap`).
    pub fn sample_count(&self) -> usize {
        self.window.len()
    }

    /// The configured delay bounds, `(min, max)`.
    pub fn bounds(&self) -> (u32, u32) {
        (self.min_delay, self.max_delay)
    }
}

/// Schedules locally-captured input to **execute `delay` ticks later** — the
/// turn-pipelining half of deterministic lockstep.
///
/// Terrano & Bettner's *1500 Archers on a 28.8: Network Programming in Age of
/// Empires and Beyond* (GDC 2001) is the canonical statement of the technique:
/// commands issued during turn *N* are not executed until turn *N+2*, which
/// buys the network two full turns to deliver every peer's commands before the
/// turn that depends on them. The simulation therefore never waits on a packet
/// mid-turn — it simply runs a fixed distance behind the player's hands, and
/// the cost is input latency rather than a stall.
///
/// Pair this with the rest of the module: [`AdaptiveDelay`] decides *how far*
/// behind to run from the measured misprediction rate, this schedules against
/// that decision, and [`NetInputBuffer`] covers whatever still fails to arrive.
///
/// ```
/// use izanagi_kit::netinput::DelayScheduler;
///
/// let mut sched: DelayScheduler<char> = DelayScheduler::new(2);
/// assert_eq!(sched.capture(10, 'a'), 12); // pressed at tick 10, acts at 12
/// assert_eq!(sched.take(11), None);       // nothing scheduled for 11
/// assert_eq!(sched.take(12), Some('a'));
/// assert_eq!(sched.take(12), None, "an input executes exactly once");
/// ```
///
/// # Changing the delay mid-session
///
/// Re-tuning the delay is where a naive scheduler corrupts the input stream,
/// so the rules here are explicit:
///
/// - **Lowering** the delay would let a newly captured input land on a tick
///   that is already spoken for (or already past). [`capture`](Self::capture)
///   therefore assigns `max(tick + delay, last_scheduled + 1)`: execute ticks
///   are **strictly increasing**, so an input can never collide with, or jump
///   ahead of, one already scheduled. The new shorter delay takes effect once
///   the schedule catches up, rather than by double-booking a tick.
/// - **Raising** the delay leaves a genuine **gap**: no input was ever captured
///   for the ticks now skipped over. That is not corruption — it is the
///   simulation legitimately having nothing new from this player — and
///   [`take`](Self::take) reports it honestly as `None`, which is exactly the
///   case [`NetInputBuffer::input_for`] predicts through by repeating the last
///   known input.
///
/// Already-scheduled inputs are never rescheduled by a delay change; moving
/// them is what would produce duplicates or holes.
///
/// # Hashing
///
/// Like [`NetInputBuffer`]'s prediction map, a `DelayScheduler` is **local
/// scheduling metadata, not simulation state**: each peer buffers only its own
/// input, and two peers on different connections legitimately hold different
/// pending queues. Fold the inputs that actually *executed* into the world
/// hash — never the queue itself.
#[derive(Clone, Debug)]
pub struct DelayScheduler<I> {
    delay: u32,
    pending: BTreeMap<u64, I>,
    last_scheduled: Option<u64>,
}

impl<I> DelayScheduler<I> {
    /// A scheduler that runs `delay` ticks behind captured input. `0` executes
    /// input on the tick it was captured (single-player / no pipelining).
    pub fn new(delay: u32) -> Self {
        DelayScheduler {
            delay,
            pending: BTreeMap::new(),
            last_scheduled: None,
        }
    }

    /// Schedule `input` captured at `tick`, returning the tick it will execute
    /// on — normally `tick + delay`, or one past the last scheduled tick when
    /// that would be earlier (see the type docs on lowering the delay).
    pub fn capture(&mut self, tick: u64, input: I) -> u64 {
        let target = tick.saturating_add(self.delay as u64);
        let exec = match self.last_scheduled {
            Some(last) if target <= last => last.saturating_add(1),
            _ => target,
        };
        self.pending.insert(exec, input);
        self.last_scheduled = Some(exec);
        exec
    }

    /// Remove and return the input scheduled to execute on `tick`, or `None`
    /// when nothing was scheduled for it (a gap left by a delay increase, or a
    /// tick before any capture). Each scheduled input is returned exactly once.
    pub fn take(&mut self, tick: u64) -> Option<I> {
        self.pending.remove(&tick)
    }

    /// The input scheduled for `tick` without consuming it.
    pub fn peek(&self, tick: u64) -> Option<&I> {
        self.pending.get(&tick)
    }

    /// Whether an input is scheduled to execute on `tick`.
    pub fn is_scheduled(&self, tick: u64) -> bool {
        self.pending.contains_key(&tick)
    }

    /// Re-tune the delay. Inputs already scheduled keep their execute tick;
    /// only later captures use the new value. See the type docs for what
    /// raising and lowering each imply.
    pub fn set_delay(&mut self, delay: u32) {
        self.delay = delay;
    }

    /// The current delay in ticks.
    pub fn delay(&self) -> u32 {
        self.delay
    }

    /// How many captured inputs are still waiting to execute.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// The furthest tick any pending input is scheduled for, or `None` when
    /// the queue is empty. Note this reflects *pending* work: unlike the
    /// internal high-water mark used to keep execute ticks increasing, it drops
    /// back as inputs are taken.
    pub fn horizon(&self) -> Option<u64> {
        self.pending.keys().next_back().copied()
    }

    /// Drop every pending input scheduled strictly before `tick` — for
    /// recovering after a stall, where inputs whose moment has passed should be
    /// discarded rather than executed late. Returns how many were dropped.
    pub fn discard_before(&mut self, tick: u64) -> usize {
        let before = self.pending.len();
        self.pending.retain(|&t, _| t >= tick);
        before - self.pending.len()
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
        assert_eq!(
            buf.input_for(0, 1),
            None,
            "no seed, no confirm → nothing to predict"
        );
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
        assert_eq!(
            buf.input_for(5, 1),
            Some(99),
            "confirmed value wins even though a prediction exists"
        );
    }

    #[test]
    fn test_input_for_memoizes_prediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 5);
        assert_eq!(buf.input_for(10, 1), Some(5));
        // Even if last_known later changes (a different tick's confirm), the
        // already-made prediction for tick 10 must not silently change.
        buf.confirm(11, 1, 999);
        assert_eq!(
            buf.input_for(10, 1),
            Some(5),
            "memoized prediction is stable"
        );
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
        assert!(
            !buf.is_predicted(5, 1),
            "confirmed slot is ground truth, not a prediction"
        );
    }

    #[test]
    fn test_is_confirmed_for_all_players() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.confirm(1, 10, 0);
        buf.confirm(1, 20, 0);
        assert!(buf.is_confirmed_for(1, &[10, 20]));
        assert!(
            !buf.is_confirmed_for(1, &[10, 20, 30]),
            "player 30 unconfirmed"
        );
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
        assert_eq!(
            buf.input_for(200, 1),
            Some(10),
            "last_known survives pruning of old ticks"
        );
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
        assert_eq!(
            buf.confirmed_input(0, 2),
            None,
            "player 2 unaffected by player 1's confirm"
        );
    }

    #[test]
    fn test_reconfirm_same_slot_is_idempotent_against_original_prediction() {
        let mut buf: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        buf.seed(1, 0);
        buf.input_for(3, 1); // predicts 0
        let first = buf.confirm(3, 1, 5); // mispredicted (0 != 5)
        let second = buf.confirm(3, 1, 5); // re-confirming the same value
        assert!(first);
        assert!(
            second,
            "still compared against the original prediction of 0, not the new value"
        );
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
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "different confirmed input → different hash"
        );
    }

    #[test]
    fn test_det_hash_order_independent() {
        let mut a: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        a.confirm(0, 1, 10);
        a.confirm(1, 2, 20);
        let mut b: NetInputBuffer<u32, i32> = NetInputBuffer::new();
        b.confirm(1, 2, 20);
        b.confirm(0, 1, 10);
        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "confirmation order does not affect the hash"
        );
    }

    // --- AdaptiveDelay ---

    #[test]
    fn test_adaptive_delay_starts_at_min() {
        let d = AdaptiveDelay::new(2, 8, 4);
        assert_eq!(d.recommended_delay(), 2);
        assert_eq!(d.sample_count(), 0);
        assert_eq!(d.misprediction_permille(), 0);
        assert_eq!(d.bounds(), (2, 8));
    }

    #[test]
    fn test_adaptive_delay_no_change_until_window_full() {
        let mut d = AdaptiveDelay::new(2, 8, 4);
        // Three all-miss samples, but the 4-slot window isn't full yet.
        for _ in 0..3 {
            d.record(true);
        }
        assert_eq!(
            d.recommended_delay(),
            2,
            "no adjustment before window fills"
        );
        // Fourth sample fills the window → 100% miss rate → raise by one.
        d.record(true);
        assert_eq!(d.recommended_delay(), 3);
    }

    #[test]
    fn test_adaptive_delay_raises_under_high_misprediction() {
        let mut d = AdaptiveDelay::new(0, 5, 4);
        // Sustained 100% misprediction climbs to max, one frame per full window.
        for _ in 0..40 {
            d.record(true);
        }
        assert_eq!(d.recommended_delay(), 5, "clamps at max_delay");
        assert_eq!(d.misprediction_permille(), 1000);
    }

    #[test]
    fn test_adaptive_delay_lowers_under_clean_prediction() {
        let mut d = AdaptiveDelay::new(1, 6, 4);
        // Drive it up first.
        for _ in 0..40 {
            d.record(true);
        }
        assert_eq!(d.recommended_delay(), 6);
        // Then a clean streak walks it back down to the floor.
        for _ in 0..40 {
            d.record(false);
        }
        assert_eq!(d.recommended_delay(), 1, "clamps at min_delay");
        assert_eq!(d.misprediction_permille(), 0);
    }

    #[test]
    fn test_adaptive_delay_holds_in_hysteresis_band() {
        // Band is (lower 50‰, raise 250‰). A steady 25% (250‰) rate sits
        // exactly on the raise threshold, so keep the rate strictly between:
        // 1 miss in 10 = 100‰, inside the band → delay must hold.
        let mut d = AdaptiveDelay::with_thresholds(3, 7, 10, 50, 250);
        for i in 0..100 {
            d.record(i % 10 == 0); // exactly 1 miss per 10 samples → 100‰
        }
        assert_eq!(d.misprediction_permille(), 100);
        assert_eq!(
            d.recommended_delay(),
            3,
            "100‰ is inside (50, 250) → no change"
        );
    }

    #[test]
    fn test_adaptive_delay_permille_tracks_window_contents() {
        let mut d = AdaptiveDelay::new(0, 10, 4);
        d.record(true);
        d.record(false);
        d.record(true);
        d.record(false); // window: T,F,T,F → 2/4 = 500‰
        assert_eq!(d.misprediction_permille(), 500);
        // Slide in four clean samples, fully evicting both misses.
        for _ in 0..4 {
            d.record(false); // window eventually F,F,F,F → 0‰
        }
        assert_eq!(d.misprediction_permille(), 0);
    }

    #[test]
    fn test_adaptive_delay_is_deterministic() {
        let run = || {
            let mut d = AdaptiveDelay::new(1, 9, 6);
            for i in 0..200u32 {
                d.record(i % 3 == 0);
            }
            (d.recommended_delay(), d.misprediction_permille())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_adaptive_delay_integrates_with_netinput_confirm() {
        // The intended wiring: feed confirm()'s misprediction bool into the
        // controller. A jittery player whose inputs keep contradicting the
        // prediction should push the recommended delay up.
        let mut buf: NetInputBuffer<u8, i32> = NetInputBuffer::new();
        let mut d = AdaptiveDelay::new(1, 6, 4);
        buf.seed(1, 0);
        for tick in 0..40 {
            buf.input_for(tick, 1); // predicts last_known
            let value = if tick % 2 == 0 { 1 } else { 0 }; // always flips → always mispredicts
            let missed = buf.confirm(tick, 1, value);
            d.record(missed);
        }
        assert!(
            d.recommended_delay() > 1,
            "sustained misprediction must raise the delay above the floor"
        );
    }

    #[test]
    #[should_panic(expected = "min_delay must be <= max_delay")]
    fn test_adaptive_delay_rejects_inverted_bounds() {
        let _ = AdaptiveDelay::new(5, 2, 4);
    }

    #[test]
    #[should_panic(expected = "window_cap must be > 0")]
    fn test_adaptive_delay_rejects_zero_window() {
        let _ = AdaptiveDelay::new(0, 4, 0);
    }

    #[test]
    #[should_panic(expected = "lower_permille must be <= raise_permille")]
    fn test_adaptive_delay_rejects_inverted_band() {
        let _ = AdaptiveDelay::with_thresholds(0, 4, 4, 300, 100);
    }

    // --- DelayScheduler ---

    #[test]
    fn test_delay_scheduler_executes_delay_ticks_later() {
        let mut s: DelayScheduler<char> = DelayScheduler::new(2);
        assert_eq!(s.capture(10, 'a'), 12);
        assert_eq!(s.capture(11, 'b'), 13);
        assert_eq!(s.take(10), None, "not yet — it acts at 12");
        assert_eq!(s.take(11), None);
        assert_eq!(s.take(12), Some('a'));
        assert_eq!(s.take(13), Some('b'));
    }

    #[test]
    fn test_delay_scheduler_zero_delay_executes_immediately() {
        let mut s: DelayScheduler<u8> = DelayScheduler::new(0);
        assert_eq!(s.capture(7, 42), 7);
        assert_eq!(s.take(7), Some(42));
    }

    #[test]
    fn test_delay_scheduler_input_executes_exactly_once() {
        let mut s: DelayScheduler<u8> = DelayScheduler::new(1);
        s.capture(0, 9);
        assert_eq!(s.take(1), Some(9));
        assert_eq!(s.take(1), None, "consumed");
        assert_eq!(s.pending_count(), 0);
    }

    #[test]
    fn test_delay_scheduler_peek_does_not_consume() {
        let mut s: DelayScheduler<u8> = DelayScheduler::new(1);
        s.capture(0, 5);
        assert_eq!(s.peek(1), Some(&5));
        assert!(s.is_scheduled(1));
        assert_eq!(s.peek(1), Some(&5), "peek is non-destructive");
        assert_eq!(s.take(1), Some(5));
        assert!(!s.is_scheduled(1));
    }

    #[test]
    fn test_delay_scheduler_raising_delay_leaves_an_honest_gap() {
        // delay 1: tick 10 -> 11. Raise to 3: tick 11 -> 14. Ticks 12 and 13
        // were never captured for, so they are genuine gaps, not corruption.
        let mut s: DelayScheduler<char> = DelayScheduler::new(1);
        assert_eq!(s.capture(10, 'a'), 11);
        s.set_delay(3);
        assert_eq!(s.capture(11, 'b'), 14);
        assert_eq!(s.take(11), Some('a'));
        assert_eq!(s.take(12), None, "gap the predictor must cover");
        assert_eq!(s.take(13), None, "gap the predictor must cover");
        assert_eq!(s.take(14), Some('b'));
    }

    #[test]
    fn test_delay_scheduler_lowering_delay_never_collides() {
        // delay 5: tick 10 -> 15. Drop to 1: tick 11 would want 12, which is
        // *earlier* than the already-scheduled 15. The monotonic rule pushes it
        // to 16 instead of double-booking or reordering.
        let mut s: DelayScheduler<char> = DelayScheduler::new(5);
        assert_eq!(s.capture(10, 'a'), 15);
        s.set_delay(1);
        assert_eq!(s.capture(11, 'b'), 16, "clamped to last_scheduled + 1");
        assert_eq!(s.capture(12, 'c'), 17);
        // Both inputs survive, in capture order, one per tick.
        assert_eq!(s.take(15), Some('a'));
        assert_eq!(s.take(16), Some('b'));
        assert_eq!(s.take(17), Some('c'));
    }

    #[test]
    fn test_delay_scheduler_no_input_is_ever_lost_or_duplicated() {
        // Property: across an arbitrary delay schedule, every captured input is
        // returned exactly once, in capture order.
        let mut s: DelayScheduler<u32> = DelayScheduler::new(2);
        let mut exec_ticks = Vec::new();
        for tick in 0..60u64 {
            // Wobble the delay across the whole legal range as we go.
            s.set_delay((tick % 7) as u32);
            exec_ticks.push(s.capture(tick, tick as u32));
        }
        // Execute ticks are strictly increasing → one input per tick, no
        // collisions, no reordering.
        assert!(
            exec_ticks.windows(2).all(|w| w[0] < w[1]),
            "execute ticks must be strictly increasing: {exec_ticks:?}"
        );
        let mut got = Vec::new();
        for tick in 0..=*exec_ticks.last().unwrap() {
            if let Some(v) = s.take(tick) {
                got.push(v);
            }
        }
        let expect: Vec<u32> = (0..60).collect();
        assert_eq!(got, expect, "every input exactly once, in order");
        assert_eq!(s.pending_count(), 0);
    }

    #[test]
    fn test_delay_scheduler_pending_count_and_horizon() {
        let mut s: DelayScheduler<u8> = DelayScheduler::new(3);
        assert_eq!(s.pending_count(), 0);
        assert_eq!(s.horizon(), None);
        s.capture(0, 1);
        s.capture(1, 2);
        assert_eq!(s.pending_count(), 2);
        assert_eq!(s.horizon(), Some(4));
        s.take(3);
        assert_eq!(s.pending_count(), 1);
        assert_eq!(s.horizon(), Some(4));
    }

    #[test]
    fn test_delay_scheduler_discard_before() {
        let mut s: DelayScheduler<u8> = DelayScheduler::new(0);
        for t in 0..5u64 {
            s.capture(t, t as u8);
        }
        assert_eq!(s.discard_before(3), 3, "ticks 0,1,2 dropped");
        assert_eq!(s.take(2), None);
        assert_eq!(s.take(3), Some(3));
        assert_eq!(s.take(4), Some(4));
    }

    #[test]
    fn test_delay_scheduler_is_deterministic() {
        let run = || {
            let mut s: DelayScheduler<u32> = DelayScheduler::new(2);
            let mut out = Vec::new();
            for t in 0..30u64 {
                s.set_delay((t % 5) as u32);
                out.push(s.capture(t, t as u32));
            }
            out
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_delay_scheduler_drives_netinput_buffer() {
        // The intended wiring: scheduled input is the ground truth confirmed
        // for its execute tick; a gap is left to the buffer's prediction.
        let mut sched: DelayScheduler<i32> = DelayScheduler::new(2);
        let mut buf: NetInputBuffer<u8, i32> = NetInputBuffer::new();
        buf.seed(1, 0);

        sched.capture(0, 10);
        sched.capture(1, 20);
        sched.set_delay(4); // opens a gap at execute-tick 4
        sched.capture(2, 30);

        let mut executed = Vec::new();
        for tick in 0..7u32 {
            match sched.take(tick as u64) {
                Some(input) => {
                    buf.confirm(tick, 1, input);
                }
                None => {
                    // Nothing scheduled — predict, exactly as a real session
                    // does for an input that has not arrived.
                }
            }
            executed.push(buf.input_for(tick, 1).unwrap());
        }
        // Ticks 2 and 3 carry the captured inputs; tick 4 is the gap and
        // repeats the last known value (20) rather than stalling.
        assert_eq!(executed[2], 10);
        assert_eq!(executed[3], 20);
        assert_eq!(executed[4], 20, "gap covered by prediction");
        assert_eq!(executed[6], 30);
    }

    #[test]
    fn test_delay_scheduler_consumes_adaptive_delay_recommendation() {
        // AdaptiveDelay decides how far behind to run; DelayScheduler schedules
        // against that decision.
        let mut ad = AdaptiveDelay::new(1, 6, 4);
        let mut sched: DelayScheduler<u32> = DelayScheduler::new(ad.recommended_delay());
        let mut ticks = Vec::new();
        for t in 0..40u64 {
            ad.record(t % 2 == 0); // jittery link → delay should climb
            sched.set_delay(ad.recommended_delay());
            ticks.push(sched.capture(t, t as u32));
        }
        assert!(
            ad.recommended_delay() > 1,
            "sustained misprediction must raise the delay"
        );
        assert!(
            ticks.windows(2).all(|w| w[0] < w[1]),
            "schedule stays collision-free while the delay moves"
        );
    }
}
