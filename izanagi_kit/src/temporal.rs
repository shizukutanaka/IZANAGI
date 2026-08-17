//! Temporal property monitors: check properties that span *time*, not just a
//! single state.
//!
//! ## The gap this fills
//!
//! Every assertion the kit could previously make about a running simulation is
//! a predicate on **one state**:
//! [`dst_sweep`](crate::dst::dst_sweep)'s `invariant(&state, tick)`,
//! [`forall_states`](crate::prop::forall_states)' `is_bad(&state)`,
//! [`audit`](crate::sim::audit)'s hash comparison. That covers "HP is never
//! negative" and stops there. It cannot express the properties that actually
//! break games, all of which relate *different ticks to each other*:
//!
//! - "killing the boss is always followed, within 60 ticks, by the loot drop"
//! - "a purchase never happens before the funds exist"
//! - "the door stays locked until the key is picked up"
//! - "once poisoned, health never goes up again until cured"
//!
//! A hand-written check for any of these needs its own scratch state threaded
//! through the tick loop, which is exactly the sort of bookkeeping that gets
//! written once, subtly wrongly, and never audited. This module supplies the
//! bookkeeping, pre-verified.
//!
//! ## Two semantics, and why both are needed
//!
//! Runtime verification distinguishes a verdict on a run that is **still
//! going** from a verdict on a run that is **over**, and conflating them is the
//! classic mistake.
//!
//! - **LTL₃** (Bauer, Leucker & Schallhart, *Runtime Verification for LTL and
//!   TLTL*, ACM TOSEM 20(4):14, 2011) gives a three-valued verdict on a finite
//!   *prefix* of a continuing run: [`Verdict::True`] if every possible
//!   continuation satisfies the property, [`Verdict::False`] if none does, and
//!   [`Verdict::Inconclusive`] otherwise. This is what [`Monitor::verdict`]
//!   reports, and it is what you want *during* a run — it fires the instant a
//!   violation becomes unavoidable, without waiting for the run to end.
//! - **Finite-trace semantics** treats the trace as complete, so every
//!   property gets a definite answer. This is [`Monitor::finish`], for when the
//!   simulation has stopped and the trace is all the evidence there will be.
//!
//! The distinction has teeth. `always(p)` can never return
//! [`Verdict::True`] from [`verdict`](Monitor::verdict) — however long `p` has
//! held, the next tick could break it — but [`finish`](Monitor::finish) says
//! `true` if it never broke. Symmetrically `eventually(p)` can never return
//! [`Verdict::False`] mid-run, but finishes `false` if it never happened. That
//! asymmetry *is* the safety/co-safety distinction, and both tests below pin it.
//!
//! ## Patterns, not a formula language
//!
//! This module offers a fixed set of combinators rather than an LTL parser,
//! because Dwyer, Avrunin & Corbett (*Patterns in Property Specifications for
//! Finite-State Verification*, ICSE 1999) found that a small catalogue of
//! patterns — absence, universality, existence, precedence, response —
//! accounted for the overwhelming majority of specifications collected from
//! real verification efforts. The constructors here are those patterns:
//! [`always`](Monitor::always) (universality), [`never`](Monitor::never)
//! (absence), [`eventually`](Monitor::eventually) (existence),
//! [`precedes`](Monitor::precedes) (precedence),
//! [`responds_within`](Monitor::responds_within) (bounded response), plus
//! [`until`](Monitor::until) and [`weak_until`](Monitor::weak_until).
//!
//! Response is deliberately **bounded**. Unbounded `G(trigger → F response)`
//! is a pure liveness property: on a finite trace it is never decidable, so a
//! monitor for it could only ever answer "inconclusive" and would be useless.
//! `responds_within(trigger, response, k)` is decidable, and "within k ticks"
//! is what a game actually wants to assert anyway.
//!
//! ```
//! use izanagi_kit::temporal::{Monitor, Verdict};
//!
//! #[derive(Clone)]
//! struct Game {
//!     boss_dead: bool,
//!     loot_dropped: bool,
//! }
//!
//! // "Once the boss dies, loot drops within 2 ticks."
//! let mut m = Monitor::responds_within(
//!     |g: &Game| g.boss_dead,
//!     |g: &Game| g.loot_dropped,
//!     2,
//! );
//!
//! m.update(&Game { boss_dead: false, loot_dropped: false });
//! m.update(&Game { boss_dead: true, loot_dropped: false }); // deadline armed
//! assert_eq!(m.verdict(), Verdict::Inconclusive);
//! m.update(&Game { boss_dead: true, loot_dropped: false });
//! m.update(&Game { boss_dead: true, loot_dropped: false }); // 2 ticks late
//! assert_eq!(m.verdict(), Verdict::False);
//! assert_eq!(m.violation(), Some(3));
//! ```

use crate::sim::Simulation;

/// A three-valued runtime-verification verdict (LTL₃).
///
/// Verdicts are **impartial**: once a monitor reports [`True`](Verdict::True)
/// or [`False`](Verdict::False) it never changes its mind, however the run
/// continues. Only [`Inconclusive`](Verdict::Inconclusive) can still move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// Every possible continuation of this run satisfies the property — it is
    /// settled, regardless of what happens next.
    True,
    /// No continuation of this run can satisfy the property — it is settled
    /// the other way. [`Monitor::violation`] reports where.
    False,
    /// The prefix seen so far settles nothing; the property could still go
    /// either way. Use [`Monitor::finish`] to force an answer once the run has
    /// ended.
    Inconclusive,
}

type Pred<S> = Box<dyn Fn(&S) -> bool>;

enum Kind<S> {
    Always(Pred<S>),
    Eventually(Pred<S>),
    Until {
        hold: Pred<S>,
        release: Pred<S>,
    },
    WeakUntil {
        hold: Pred<S>,
        release: Pred<S>,
    },
    Precedes {
        first: Pred<S>,
        second: Pred<S>,
    },
    RespondsWithin {
        trigger: Pred<S>,
        response: Pred<S>,
        within: usize,
    },
}

/// A monitor for one temporal property over a stream of simulation states.
///
/// Feed it every state with [`update`](Monitor::update) — position 0 is the
/// state *before* any input, so a run of `n` inputs produces `n + 1` updates.
/// Read [`verdict`](Monitor::verdict) at any time for the LTL₃ verdict, or
/// [`finish`](Monitor::finish) once the run is over for a definite one.
///
/// Each update is O(1) and allocates nothing, so monitors are cheap enough to
/// leave running through a whole [`dst`](crate::dst) sweep.
pub struct Monitor<S> {
    kind: Kind<S>,
    verdict: Verdict,
    ticks: usize,
    violation: Option<usize>,
    /// `Precedes`: has the `first` predicate held yet?
    seen_first: bool,
    /// `RespondsWithin`: tick of the oldest trigger still awaiting a response.
    /// Tracking only the oldest is sound and complete — a response clears every
    /// outstanding trigger at once, and the oldest has the earliest deadline.
    pending: Option<usize>,
}

impl<S> Monitor<S> {
    fn new(kind: Kind<S>) -> Self {
        Monitor {
            kind,
            verdict: Verdict::Inconclusive,
            ticks: 0,
            violation: None,
            seen_first: false,
            pending: None,
        }
    }

    /// **Universality**: `pred` holds at every tick. Fails at the first tick it
    /// does not.
    ///
    /// A safety property: it can be refuted mid-run but never confirmed, so
    /// [`verdict`](Monitor::verdict) returns [`Verdict::False`] or
    /// [`Verdict::Inconclusive`] and never [`Verdict::True`].
    pub fn always(pred: impl Fn(&S) -> bool + 'static) -> Self {
        Monitor::new(Kind::Always(Box::new(pred)))
    }

    /// **Absence**: `pred` holds at no tick. Exactly `always(!pred)`.
    pub fn never(pred: impl Fn(&S) -> bool + 'static) -> Self {
        Monitor::new(Kind::Always(Box::new(move |s| !pred(s))))
    }

    /// **Existence**: `pred` holds at some tick.
    ///
    /// A co-safety property, the mirror of [`always`](Monitor::always): it can
    /// be confirmed mid-run but never refuted, so [`verdict`](Monitor::verdict)
    /// never returns [`Verdict::False`].
    pub fn eventually(pred: impl Fn(&S) -> bool + 'static) -> Self {
        Monitor::new(Kind::Eventually(Box::new(pred)))
    }

    /// **Strong until** (`hold U release`): `hold` holds at every tick strictly
    /// before the first tick where `release` holds, and `release` must
    /// eventually hold.
    ///
    /// Settles [`True`](Verdict::True) when `release` first holds, and
    /// [`False`](Verdict::False) if `hold` breaks before that. A run that ends
    /// with `hold` still holding and `release` never seen
    /// [`finish`](Monitor::finish)es `false` — the release never came.
    pub fn until(
        hold: impl Fn(&S) -> bool + 'static,
        release: impl Fn(&S) -> bool + 'static,
    ) -> Self {
        Monitor::new(Kind::Until {
            hold: Box::new(hold),
            release: Box::new(release),
        })
    }

    /// **Weak until** (`hold W release`): as [`until`](Monitor::until), except
    /// `release` need never arrive so long as `hold` never breaks.
    ///
    /// This is the honest form of "the door stays locked until the key is
    /// picked up" — never picking the key up is not a bug.
    pub fn weak_until(
        hold: impl Fn(&S) -> bool + 'static,
        release: impl Fn(&S) -> bool + 'static,
    ) -> Self {
        Monitor::new(Kind::WeakUntil {
            hold: Box::new(hold),
            release: Box::new(release),
        })
    }

    /// **Precedence**: `second` never holds unless `first` has held at or
    /// before that tick — the `¬second W first` reading of Dwyer's precedence
    /// pattern.
    ///
    /// Note the "at or before": `first` and `second` holding on the *same* tick
    /// is allowed, which is what you want for "a purchase must be preceded by
    /// sufficient funds" (the funds are still there on the tick you spend
    /// them).
    pub fn precedes(
        first: impl Fn(&S) -> bool + 'static,
        second: impl Fn(&S) -> bool + 'static,
    ) -> Self {
        Monitor::new(Kind::Precedes {
            first: Box::new(first),
            second: Box::new(second),
        })
    }

    /// **Bounded response**: every tick where `trigger` holds is followed,
    /// within `within` further ticks, by a tick where `response` holds.
    ///
    /// The window is inclusive of the trigger's own tick, so `within = 0` means
    /// "must respond on the same tick". A run that ends with a trigger still
    /// unanswered [`finish`](Monitor::finish)es `false`, even if its deadline
    /// had not yet expired — the finite trace is all the evidence there is, and
    /// the response is not in it.
    ///
    /// See the module docs for why the unbounded form is deliberately absent.
    pub fn responds_within(
        trigger: impl Fn(&S) -> bool + 'static,
        response: impl Fn(&S) -> bool + 'static,
        within: usize,
    ) -> Self {
        Monitor::new(Kind::RespondsWithin {
            trigger: Box::new(trigger),
            response: Box::new(response),
            within,
        })
    }

    /// Feed the next state. Position 0 is the state before any input.
    ///
    /// Once the verdict has settled this only advances the tick counter, so
    /// leaving a decided monitor in a loop costs nothing.
    pub fn update(&mut self, state: &S) {
        let i = self.ticks;
        self.ticks += 1;
        if self.verdict != Verdict::Inconclusive {
            return;
        }
        // Field-level borrows: `self.kind` is read while other fields are
        // written, which is why the mutations are inline rather than a method.
        match &self.kind {
            Kind::Always(pred) => {
                if !pred(state) {
                    self.verdict = Verdict::False;
                    self.violation = Some(i);
                }
            }
            Kind::Eventually(pred) => {
                if pred(state) {
                    self.verdict = Verdict::True;
                }
            }
            Kind::Until { hold, release } | Kind::WeakUntil { hold, release } => {
                if release(state) {
                    self.verdict = Verdict::True;
                } else if !hold(state) {
                    self.verdict = Verdict::False;
                    self.violation = Some(i);
                }
            }
            Kind::Precedes { first, second } => {
                if first(state) {
                    self.seen_first = true;
                }
                if second(state) && !self.seen_first {
                    self.verdict = Verdict::False;
                    self.violation = Some(i);
                }
            }
            Kind::RespondsWithin {
                trigger,
                response,
                within,
            } => {
                if response(state) {
                    self.pending = None;
                } else {
                    if trigger(state) && self.pending.is_none() {
                        self.pending = Some(i);
                    }
                    if let Some(armed) = self.pending {
                        if i - armed >= *within {
                            self.verdict = Verdict::False;
                            self.violation = Some(i);
                        }
                    }
                }
            }
        }
    }

    /// The LTL₃ verdict on the run *so far*, treating it as a prefix that may
    /// still continue. See the module docs for how this differs from
    /// [`finish`](Monitor::finish).
    pub fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The definite verdict for a run that is **over**: `true` if the completed
    /// trace satisfies the property.
    ///
    /// This is where an inconclusive property is forced to commit — `always`
    /// that never broke is `true`, `eventually` that never happened is `false`.
    pub fn finish(&self) -> bool {
        match self.verdict {
            Verdict::True => true,
            Verdict::False => false,
            Verdict::Inconclusive => match &self.kind {
                // Never violated, and a run that ends cannot violate them later.
                Kind::Always(_) | Kind::Precedes { .. } | Kind::WeakUntil { .. } => true,
                // The awaited event never arrived.
                Kind::Eventually(_) | Kind::Until { .. } => false,
                // Satisfied unless a trigger is still outstanding.
                Kind::RespondsWithin { .. } => self.pending.is_none(),
            },
        }
    }

    /// The tick at which the property was refuted, if it was.
    ///
    /// Monitors are *anticipatory*: this is the earliest tick at which the
    /// violation became unavoidable, not the tick at which some later check
    /// noticed.
    pub fn violation(&self) -> Option<usize> {
        self.violation
    }

    /// How many states have been fed in.
    pub fn ticks(&self) -> usize {
        self.ticks
    }
}

/// A named collection of monitors advanced together — the practical way to
/// check a batch of properties over one run.
///
/// [`update`](MonitorSet::update) returns `Result<(), String>`, which is
/// exactly the shape [`dst_sweep`](crate::dst::dst_sweep) wants for its
/// `invariant` argument, so a whole property suite drops into a seed sweep:
///
/// ```
/// use izanagi_kit::dst::dst_sweep;
/// use izanagi_kit::temporal::{Monitor, MonitorSet};
///
/// #[derive(Clone)]
/// struct S { hp: i32, healing: bool }
///
/// let mut props = MonitorSet::new()
///     .with("hp-never-negative", Monitor::always(|s: &S| s.hp >= 0))
///     .with("healing-is-answered", Monitor::responds_within(
///         |s: &S| s.healing, |s: &S| s.hp > 0, 4));
///
/// let result = dst_sweep(
///     0..8u64,
///     20,
///     |_seed| S { hp: 10, healing: false },
///     |s: &mut S, _tick| { s.hp -= 1; if s.hp < 1 { s.hp = 10; } },
///     |s: &S, _tick| props.update(s),
/// );
/// assert!(result.is_ok());
/// ```
#[derive(Default)]
pub struct MonitorSet<S> {
    entries: Vec<(String, Monitor<S>)>,
}

impl<S> MonitorSet<S> {
    /// An empty set.
    pub fn new() -> Self {
        MonitorSet {
            entries: Vec::new(),
        }
    }

    /// Add a named monitor, consuming and returning `self` for chaining.
    pub fn with(mut self, name: impl Into<String>, monitor: Monitor<S>) -> Self {
        self.push(name, monitor);
        self
    }

    /// Add a named monitor in place.
    pub fn push(&mut self, name: impl Into<String>, monitor: Monitor<S>) {
        self.entries.push((name.into(), monitor));
    }

    /// Advance every monitor by one state, returning `Err` naming the first
    /// property (in insertion order) whose verdict has settled on
    /// [`Verdict::False`].
    ///
    /// Every monitor is updated before the error is produced, so no monitor
    /// falls behind when one of them fails. Once a property has failed this
    /// keeps reporting it — sweeps stop at the first `Err` anyway.
    pub fn update(&mut self, state: &S) -> Result<(), String> {
        for (_, monitor) in self.entries.iter_mut() {
            monitor.update(state);
        }
        for (name, monitor) in self.entries.iter() {
            if monitor.verdict() == Verdict::False {
                let at = monitor.violation().unwrap_or_else(|| monitor.ticks());
                return Err(format!("temporal property `{name}` violated at tick {at}"));
            }
        }
        Ok(())
    }

    /// The LTL₃ verdict of a named property, or `None` if there is no such
    /// property.
    pub fn verdict(&self, name: &str) -> Option<Verdict> {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, m)| m.verdict())
    }

    /// The names of the properties the completed run does **not** satisfy, in
    /// insertion order — the end-of-run report, using
    /// [`Monitor::finish`] semantics.
    pub fn failed(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|(_, m)| !m.finish())
            .map(|(n, _)| n.as_str())
            .collect()
    }

    /// Whether the completed run satisfies every property.
    pub fn all_hold(&self) -> bool {
        self.entries.iter().all(|(_, m)| m.finish())
    }

    /// Number of monitors in the set.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the set holds no monitors.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Run a [`Simulation`] over `inputs` and check `monitor` against the whole
/// trace, returning the definite finite-trace verdict.
///
/// The initial state is position 0, so `inputs.len() + 1` states are checked.
/// `initial` is left untouched; the run happens on a clone.
pub fn check_run<S>(initial: &S, inputs: &[S::Input], monitor: &mut Monitor<S>) -> bool
where
    S: Simulation + Clone,
{
    let mut state = initial.clone();
    monitor.update(&state);
    for input in inputs {
        state.step(input);
        monitor.update(&state);
    }
    monitor.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prop::forall_inputs;

    /// A two-bit state, so a trace is just a pair of boolean sequences and the
    /// reference semantics below can be written directly over them.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Bits {
        a: bool,
        b: bool,
    }

    fn a_of(t: &[Bits]) -> Vec<bool> {
        t.iter().map(|s| s.a).collect()
    }
    fn b_of(t: &[Bits]) -> Vec<bool> {
        t.iter().map(|s| s.b).collect()
    }

    // --- Reference semantics: the obviously-correct definitions, written
    // --- directly over the whole trace. These are the oracle; the incremental
    // --- monitors must agree with them on every trace.

    fn ref_always(a: &[bool]) -> bool {
        a.iter().all(|&x| x)
    }

    fn ref_eventually(a: &[bool]) -> bool {
        a.iter().any(|&x| x)
    }

    /// `a U b`: some position satisfies `b`, and every strictly earlier
    /// position satisfies `a`.
    fn ref_until(a: &[bool], b: &[bool]) -> bool {
        for i in 0..b.len() {
            if b[i] {
                return a[..i].iter().all(|&x| x);
            }
        }
        false
    }

    /// `a W b`: `a U b`, or `a` holds everywhere.
    fn ref_weak_until(a: &[bool], b: &[bool]) -> bool {
        for i in 0..b.len() {
            if b[i] {
                return a[..i].iter().all(|&x| x);
            }
        }
        a.iter().all(|&x| x)
    }

    /// `¬q W p`: no position satisfies `q` before some position has satisfied
    /// `p`, counting the same position as "not before".
    fn ref_precedes(p: &[bool], q: &[bool]) -> bool {
        let mut seen = false;
        for i in 0..q.len() {
            if p[i] {
                seen = true;
            }
            if q[i] && !seen {
                return false;
            }
        }
        true
    }

    /// `G(t → F≤k r)` over a finite trace: every trigger has a response in its
    /// inclusive window, clipped to the end of the trace.
    fn ref_responds_within(t: &[bool], r: &[bool], k: usize) -> bool {
        let n = t.len();
        for (i, &triggered) in t.iter().enumerate() {
            if triggered {
                let end = (i + k).min(n - 1);
                if !(i..=end).any(|j| r[j]) {
                    return false;
                }
            }
        }
        true
    }

    fn run(monitor: &mut Monitor<Bits>, trace: &[Bits]) {
        for s in trace {
            monitor.update(s);
        }
    }

    /// Random traces, biased so that each predicate is neither always nor never
    /// true — a trace of all-true bits would pass every property vacuously.
    fn gen_bits(rng: &mut crate::rng::SplitMix64) -> Bits {
        Bits {
            a: rng.below(4) != 0, // true ~75% of the time
            b: rng.below(5) == 0, // true ~20% of the time
        }
    }

    // --- Differential tests: incremental monitor vs. denotational reference.

    #[test]
    fn test_always_matches_reference_on_random_traces() {
        let r = forall_inputs(0..400u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::always(|s: &Bits| s.a);
            run(&mut m, trace);
            m.finish() == ref_always(&a_of(trace))
        });
        assert_eq!(r, Ok(()), "always disagreed with its definition");
    }

    #[test]
    fn test_never_matches_reference_on_random_traces() {
        let r = forall_inputs(0..400u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::never(|s: &Bits| s.b);
            run(&mut m, trace);
            // `never(p)` is exactly `!eventually(p)` — stated that way round
            // rather than folded into a `!=`, because that is the claim.
            let expected = !ref_eventually(&b_of(trace));
            m.finish() == expected
        });
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn test_eventually_matches_reference_on_random_traces() {
        let r = forall_inputs(0..400u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::eventually(|s: &Bits| s.b);
            run(&mut m, trace);
            m.finish() == ref_eventually(&b_of(trace))
        });
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn test_until_matches_reference_on_random_traces() {
        let r = forall_inputs(0..400u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::until(|s: &Bits| s.a, |s: &Bits| s.b);
            run(&mut m, trace);
            m.finish() == ref_until(&a_of(trace), &b_of(trace))
        });
        assert_eq!(r, Ok(()), "until disagreed with its definition");
    }

    #[test]
    fn test_weak_until_matches_reference_on_random_traces() {
        let r = forall_inputs(0..400u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::weak_until(|s: &Bits| s.a, |s: &Bits| s.b);
            run(&mut m, trace);
            m.finish() == ref_weak_until(&a_of(trace), &b_of(trace))
        });
        assert_eq!(r, Ok(()), "weak_until disagreed with its definition");
    }

    #[test]
    fn test_precedes_matches_reference_on_random_traces() {
        let r = forall_inputs(0..400u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::precedes(|s: &Bits| s.b, |s: &Bits| s.a);
            run(&mut m, trace);
            m.finish() == ref_precedes(&b_of(trace), &a_of(trace))
        });
        assert_eq!(r, Ok(()), "precedes disagreed with its definition");
    }

    #[test]
    fn test_responds_within_matches_reference_for_every_bound() {
        // Sweep the bound too: the deadline arithmetic is the part most likely
        // to be off by one, and k = 0 and k >= trace length are the edges.
        for k in 0..6usize {
            let r = forall_inputs(0..200u64, 20, gen_bits, |trace: &[Bits]| {
                if trace.is_empty() {
                    return true; // covered by its own test below
                }
                let mut m = Monitor::responds_within(|s: &Bits| s.b, |s: &Bits| s.a, k);
                run(&mut m, trace);
                m.finish() == ref_responds_within(&b_of(trace), &a_of(trace), k)
            });
            assert_eq!(r, Ok(()), "responds_within(k={k}) disagreed");
        }
    }

    // --- Structural properties of the monitors themselves.

    #[test]
    fn test_verdicts_are_impartial() {
        // The defining property of an LTL3 monitor: once it settles on True or
        // False it never revises. Checked over random traces for every pattern.
        let r = forall_inputs(0..300u64, 24, gen_bits, |trace: &[Bits]| {
            let mut monitors: Vec<Monitor<Bits>> = vec![
                Monitor::always(|s: &Bits| s.a),
                Monitor::eventually(|s: &Bits| s.b),
                Monitor::until(|s: &Bits| s.a, |s: &Bits| s.b),
                Monitor::weak_until(|s: &Bits| s.a, |s: &Bits| s.b),
                Monitor::precedes(|s: &Bits| s.b, |s: &Bits| s.a),
                Monitor::responds_within(|s: &Bits| s.b, |s: &Bits| s.a, 3),
            ];
            for m in monitors.iter_mut() {
                let mut settled: Option<Verdict> = None;
                for s in trace {
                    m.update(s);
                    match settled {
                        Some(v) => {
                            if m.verdict() != v {
                                return false; // revised a settled verdict
                            }
                        }
                        None => {
                            if m.verdict() != Verdict::Inconclusive {
                                settled = Some(m.verdict());
                            }
                        }
                    }
                }
                // A settled verdict must agree with the finished one.
                if let Some(v) = settled {
                    if m.finish() != (v == Verdict::True) {
                        return false;
                    }
                }
            }
            true
        });
        assert_eq!(r, Ok(()), "a monitor revised a settled verdict");
    }

    #[test]
    fn test_safety_and_cosafety_verdict_asymmetry() {
        // `always` is a safety property: refutable, never confirmable.
        // `eventually` is co-safety: confirmable, never refutable. This is the
        // LTL3 characterisation, and it must hold on every trace.
        let r = forall_inputs(0..300u64, 24, gen_bits, |trace: &[Bits]| {
            let mut safety = Monitor::always(|s: &Bits| s.a);
            let mut cosafety = Monitor::eventually(|s: &Bits| s.b);
            for s in trace {
                safety.update(s);
                cosafety.update(s);
                if safety.verdict() == Verdict::True {
                    return false;
                }
                if cosafety.verdict() == Verdict::False {
                    return false;
                }
            }
            true
        });
        assert_eq!(r, Ok(()), "safety/co-safety asymmetry broken");
    }

    #[test]
    fn test_always_reports_the_earliest_violating_tick() {
        // Anticipation: the reported tick is the first offending one, not
        // wherever the check happened to notice.
        let r = forall_inputs(0..300u64, 24, gen_bits, |trace: &[Bits]| {
            let mut m = Monitor::always(|s: &Bits| s.a);
            run(&mut m, trace);
            let expected = trace.iter().position(|s| !s.a);
            m.violation() == expected
        });
        assert_eq!(r, Ok(()), "violation tick was not the earliest");
    }

    #[test]
    fn test_responds_within_fires_exactly_at_the_deadline() {
        // A trigger at tick 1 with within=2 must be reported at tick 3 — not
        // 2 (too eager, the window is inclusive) and not 4 (too late).
        let t = |b: bool, a: bool| Bits { a, b };
        for within in 0..4usize {
            let mut m = Monitor::responds_within(|s: &Bits| s.b, |s: &Bits| s.a, within);
            m.update(&t(false, false)); // tick 0: quiet
            m.update(&t(true, false)); // tick 1: trigger, no response
            for _ in 0..6 {
                m.update(&t(false, false));
            }
            assert_eq!(
                m.violation(),
                Some(1 + within),
                "within={within} should fire at tick {}",
                1 + within
            );
        }
    }

    #[test]
    fn test_responds_within_accepts_a_same_tick_response() {
        // The window is inclusive of the trigger's own tick, so within=0 is
        // satisfiable rather than vacuously false.
        let mut m = Monitor::responds_within(|s: &Bits| s.b, |s: &Bits| s.a, 0);
        m.update(&Bits { a: true, b: true });
        m.update(&Bits { a: false, b: false });
        assert_eq!(m.verdict(), Verdict::Inconclusive);
        assert!(m.finish());
    }

    #[test]
    fn test_responds_within_unanswered_at_end_of_trace_fails() {
        // The deadline has not expired, so the run verdict is inconclusive —
        // but the trace is over and the response is not in it.
        let mut m = Monitor::responds_within(|s: &Bits| s.b, |s: &Bits| s.a, 10);
        m.update(&Bits { a: false, b: true });
        assert_eq!(m.verdict(), Verdict::Inconclusive);
        assert!(!m.finish(), "an outstanding trigger must fail at finish");
    }

    #[test]
    fn test_precedes_allows_a_simultaneous_first() {
        // `¬q W p` permits p and q on the same tick; only q strictly earlier is
        // a violation.
        let mut ok = Monitor::precedes(|s: &Bits| s.a, |s: &Bits| s.b);
        ok.update(&Bits { a: true, b: true });
        assert_eq!(ok.verdict(), Verdict::Inconclusive);
        assert!(ok.finish());

        let mut bad = Monitor::precedes(|s: &Bits| s.a, |s: &Bits| s.b);
        bad.update(&Bits { a: false, b: true });
        assert_eq!(bad.verdict(), Verdict::False);
        assert_eq!(bad.violation(), Some(0));
    }

    #[test]
    fn test_empty_trace_matches_reference_for_every_pattern() {
        let empty: [Bits; 0] = [];
        let mut always = Monitor::always(|s: &Bits| s.a);
        let mut eventually = Monitor::eventually(|s: &Bits| s.a);
        let mut until = Monitor::until(|s: &Bits| s.a, |s: &Bits| s.b);
        let mut weak = Monitor::weak_until(|s: &Bits| s.a, |s: &Bits| s.b);
        let mut prec = Monitor::precedes(|s: &Bits| s.a, |s: &Bits| s.b);
        let mut resp = Monitor::responds_within(|s: &Bits| s.a, |s: &Bits| s.b, 2);
        for m in [&mut always, &mut eventually] {
            run(m, &empty);
        }
        run(&mut until, &empty);
        run(&mut weak, &empty);
        run(&mut prec, &empty);
        run(&mut resp, &empty);

        assert_eq!(always.finish(), ref_always(&[]));
        assert_eq!(eventually.finish(), ref_eventually(&[]));
        assert_eq!(until.finish(), ref_until(&[], &[]));
        assert_eq!(weak.finish(), ref_weak_until(&[], &[]));
        assert_eq!(prec.finish(), ref_precedes(&[], &[]));
        assert_eq!(resp.finish(), ref_responds_within(&[], &[], 2));
        assert_eq!(always.ticks(), 0);
    }

    #[test]
    fn test_updates_after_a_settled_verdict_are_inert() {
        let mut m = Monitor::always(|s: &Bits| s.a);
        m.update(&Bits { a: false, b: false });
        assert_eq!(m.violation(), Some(0));
        for _ in 0..5 {
            m.update(&Bits { a: true, b: true });
        }
        assert_eq!(m.verdict(), Verdict::False);
        assert_eq!(m.violation(), Some(0), "violation tick must not move");
        assert_eq!(m.ticks(), 6, "ticks still advance");
    }

    #[test]
    fn test_monitor_is_deterministic_over_the_same_trace() {
        let trace: Vec<Bits> = (0..40)
            .map(|i| Bits {
                a: i % 3 != 0,
                b: i % 7 == 0,
            })
            .collect();
        let build = || Monitor::responds_within(|s: &Bits| s.b, |s: &Bits| s.a, 2);
        let (mut m1, mut m2) = (build(), build());
        run(&mut m1, &trace);
        run(&mut m2, &trace);
        assert_eq!(m1.verdict(), m2.verdict());
        assert_eq!(m1.violation(), m2.violation());
        assert_eq!(m1.finish(), m2.finish());
    }

    // --- MonitorSet and integration with the rest of the kit.

    #[test]
    fn test_monitor_set_names_the_first_failing_property() {
        let mut set = MonitorSet::new()
            .with("a-always", Monitor::always(|s: &Bits| s.a))
            .with("b-never", Monitor::never(|s: &Bits| s.b));
        assert_eq!(set.len(), 2);
        assert!(set.update(&Bits { a: true, b: false }).is_ok());

        let err = set
            .update(&Bits { a: true, b: true })
            .expect_err("b-never is violated");
        assert!(err.contains("b-never"), "{err}");
        assert!(err.contains("tick 1"), "{err}");
        assert_eq!(set.verdict("b-never"), Some(Verdict::False));
        assert_eq!(set.verdict("a-always"), Some(Verdict::Inconclusive));
        assert_eq!(set.verdict("no-such"), None);
        assert_eq!(set.failed(), vec!["b-never"]);
        assert!(!set.all_hold());
    }

    #[test]
    fn test_monitor_set_advances_every_monitor_even_when_one_fails() {
        // A failing monitor must not stop the others from seeing the state, or
        // their tick numbering would drift.
        let mut set = MonitorSet::new()
            .with("fails-now", Monitor::always(|s: &Bits| s.a))
            .with("fails-later", Monitor::never(|s: &Bits| s.b));
        let _ = set.update(&Bits { a: false, b: false });
        let _ = set.update(&Bits { a: false, b: true });
        // The second monitor still observed tick 1 and recorded it.
        assert_eq!(set.verdict("fails-later"), Some(Verdict::False));
        assert_eq!(set.failed(), vec!["fails-now", "fails-later"]);
    }

    #[test]
    fn test_empty_monitor_set_holds_vacuously() {
        let mut set: MonitorSet<Bits> = MonitorSet::new();
        assert!(set.is_empty());
        assert!(set.update(&Bits { a: false, b: true }).is_ok());
        assert!(set.all_hold());
        assert!(set.failed().is_empty());
    }

    #[test]
    fn test_plugs_into_dst_sweep_and_reports_seed_and_tick() {
        use crate::dst::dst_sweep;

        #[derive(Clone)]
        struct Counter {
            n: i32,
        }
        // The sim is fine for 5 ticks and then breaks the property, so the
        // sweep must report the first seed and the exact tick.
        let mut props = MonitorSet::new().with("under-six", Monitor::always(|c: &Counter| c.n < 6));
        let failure = dst_sweep(
            0..4u64,
            10,
            |_seed| Counter { n: 0 },
            |c: &mut Counter, _tick| c.n += 1,
            |c: &Counter, _tick| props.update(c),
        )
        .expect_err("the counter passes 6");
        assert_eq!(failure.seed, 0);
        assert!(failure.message.contains("under-six"), "{failure}");
    }

    #[test]
    fn test_check_run_drives_a_simulation_and_includes_the_initial_state() {
        #[derive(Clone)]
        struct Acc {
            total: i32,
        }
        impl Simulation for Acc {
            type Input = i32;
            fn step(&mut self, input: &i32) {
                self.total += *input;
            }
        }

        // Position 0 is the initial state, so a property already false there is
        // caught before any input is applied.
        let mut m = Monitor::always(|a: &Acc| a.total > 0);
        assert!(!check_run(&Acc { total: 0 }, &[5, 5], &mut m));
        assert_eq!(m.violation(), Some(0));
        assert_eq!(m.ticks(), 3, "initial state plus one per input");

        // And a property that holds across the whole run passes.
        let mut ok = Monitor::eventually(|a: &Acc| a.total >= 10);
        assert!(check_run(&Acc { total: 0 }, &[5, 5], &mut ok));

        let initial = Acc { total: 1 };
        let mut m2 = Monitor::always(|a: &Acc| a.total > 0);
        let _ = check_run(&initial, &[1, 1], &mut m2);
        assert_eq!(initial.total, 1, "check_run must run on a clone");
    }
}
