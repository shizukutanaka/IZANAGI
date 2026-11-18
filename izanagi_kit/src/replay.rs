//! Deterministic replay & desync detection harness.
//!
//! The kit's reason for existing is bit-exact replay: the same initial state
//! driven by the same inputs must produce the same state, tick for tick. This
//! module turns that promise into reusable tooling, generalising what
//! `tests/determinism.rs` does by hand.
//!
//! - [`record_trace`] runs a simulation and returns the per-tick state-hash
//!   sequence (the "replay trace").
//! - [`check_trace`] re-runs against a recorded trace and reports the **first**
//!   diverging tick — the starting point for any desync hunt.
//! - [`encode_trace`] / [`decode_trace`] persist a recorded trace — seed,
//!   per-tick input frames, and expected hashes — as canonical bytes, so a
//!   run can be stored, diffed, and re-verified by [`check_trace`] in a
//!   later process or on another machine.
//! - [`first_divergence`] compares two traces directly (e.g. from two peers).
//! - [`resimulate`] clones a snapshot and replays inputs onto it — the basis of
//!   rollback netcode (snapshot a known-good tick, re-run newer inputs).
//!
//! The simulation is supplied as a `step(&mut state, &input)` closure, so this
//! is engine-agnostic. State is hashed via [`DetHash`], so any state built from
//! the kit's value types is replay-checkable for free.

use crate::world_hash::{hash_state, DetHash, Fnv1a};

/// Where two replay traces first disagree. `tick` is the 0-based step index;
/// `expected`/`actual` are the state hashes there (0 if that trace was shorter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divergence {
    /// 0-based step index where the traces first disagreed.
    pub tick: usize,
    /// The expected (reference) state hash at that tick.
    pub expected: u64,
    /// The actual (observed) state hash at that tick.
    pub actual: u64,
}

impl core::fmt::Display for Divergence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "replay divergence at tick {}: expected {:#018x}, got {:#018x}",
            self.tick, self.expected, self.actual
        )
    }
}

/// Advance `state` through `inputs` with `step`, recording the state hash after
/// each tick. The returned trace has length `inputs.len()`.
pub fn record_trace<S, I, F>(state: &mut S, inputs: &[I], mut step: F) -> Vec<u64>
where
    S: DetHash,
    F: FnMut(&mut S, &I),
{
    let mut trace = Vec::with_capacity(inputs.len());
    for input in inputs {
        step(state, input);
        trace.push(hash_state(state));
    }
    trace
}

/// Re-run `state` through `inputs` and compare against a previously recorded
/// `expected` trace, returning the first diverging tick if any. A length
/// mismatch counts as a divergence at the first missing tick.
pub fn check_trace<S, I, F>(
    state: &mut S,
    inputs: &[I],
    expected: &[u64],
    step: F,
) -> Result<(), Divergence>
where
    S: DetHash,
    F: FnMut(&mut S, &I),
{
    let actual = record_trace(state, inputs, step);
    first_divergence(expected, &actual)
}

/// Compare two state-hash traces tick by tick. `Ok(())` iff identical (same
/// length and values); otherwise the earliest disagreement. Useful for
/// comparing dumps from two peers / two builds to localise a desync.
pub fn first_divergence(expected: &[u64], actual: &[u64]) -> Result<(), Divergence> {
    let ticks = expected.len().max(actual.len());
    for tick in 0..ticks {
        let e = expected.get(tick).copied();
        let a = actual.get(tick).copied();
        if e != a {
            return Err(Divergence {
                tick,
                expected: e.unwrap_or(0),
                actual: a.unwrap_or(0),
            });
        }
    }
    Ok(())
}

/// Count the number of ticks where `expected` and `actual` disagree.
/// Ticks beyond the shorter trace count as divergences (the two runs produced
/// different lengths). Returns 0 for identical traces.
pub fn count_divergences(expected: &[u64], actual: &[u64]) -> usize {
    let ticks = expected.len().max(actual.len());
    (0..ticks)
        .filter(|&i| expected.get(i) != actual.get(i))
        .count()
}

/// Collect **all** ticks where `expected` and `actual` disagree into a
/// `Vec<Divergence>`. Unlike [`first_divergence`] (stops at the first mismatch)
/// or [`count_divergences`] (returns only a count), this returns every
/// divergence so that multiple desyncs can be inspected at once. Ticks beyond
/// the shorter trace are included as divergences (missing hash treated as `0`).
pub fn find_all_divergences(expected: &[u64], actual: &[u64]) -> Vec<Divergence> {
    let ticks = expected.len().max(actual.len());
    (0..ticks)
        .filter_map(|tick| {
            let e = expected.get(tick).copied();
            let a = actual.get(tick).copied();
            if e != a {
                Some(Divergence {
                    tick,
                    expected: e.unwrap_or(0),
                    actual: a.unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Returns `true` when `expected` and `actual` are identical (no divergence).
/// Thin wrapper around [`first_divergence`] for callers that only need a boolean
/// answer — avoids `.is_ok()` boilerplate on the `Result`.
#[inline]
pub fn replay_ok(expected: &[u64], actual: &[u64]) -> bool {
    first_divergence(expected, actual).is_ok()
}

/// Percentage of ticks (0–100) at which the two traces diverge.
///
/// Ticks beyond the shorter trace count as divergences. Returns `0` when both
/// traces are empty. Result is integer-rounded toward zero — use this for
/// "reject if > 5% of ticks diverged" CI gates and replay-quality dashboards
/// without floating-point arithmetic.
pub fn divergence_percent(expected: &[u64], actual: &[u64]) -> u32 {
    let total = expected.len().max(actual.len());
    if total == 0 {
        return 0;
    }
    let diverged = count_divergences(expected, actual);
    (diverged.saturating_mul(100) / total) as u32
}

/// Replay `inputs` onto a **clone** of `snapshot`, returning the resulting
/// state and leaving `snapshot` untouched. This is the core rollback operation:
/// keep a confirmed-good snapshot, then re-simulate the inputs received since.
pub fn resimulate<S, I, F>(snapshot: &S, inputs: &[I], mut step: F) -> S
where
    S: Clone,
    F: FnMut(&mut S, &I),
{
    let mut state = snapshot.clone();
    for input in inputs {
        step(&mut state, input);
    }
    state
}

// ─────────────────────────────────────────────────────────────────────────
// Subsystem-localized divergence
// ─────────────────────────────────────────────────────────────────────────

/// Where two *labeled* replay traces first disagree — the tick **and the
/// subsystem** within it. This is the localization a plain [`Divergence`]
/// cannot give: a per-tick `u64` says "tick 4207 differs", a labeled trace
/// says "tick 4207, subsystem `enemies` differs" — the difference between an
/// afternoon of bisection and a one-line fix.
///
/// A labeled trace is one [`LabeledDigest`](crate::world_hash::LabeledDigest)'s
/// `parts()` per tick: `&[&[(&'static str, u64)]]`. Build each tick's digest by
/// hashing your subsystems under stable labels (see `LabeledDigest`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabeledDivergence {
    /// 0-based tick where the traces first disagreed.
    pub tick: usize,
    /// The first subsystem (by position within the tick) that differed.
    /// `"<missing>"` if one trace had fewer subsystems at this tick;
    /// `"<label-mismatch>"` if the two traces labeled position *i* differently
    /// (a structural mismatch — the traces don't describe the same world).
    pub subsystem: &'static str,
    /// The expected (reference) subsystem hash (`0` if absent).
    pub expected: u64,
    /// The actual (observed) subsystem hash (`0` if absent).
    pub actual: u64,
}

impl core::fmt::Display for LabeledDivergence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "replay divergence at tick {}, subsystem '{}': expected {:#018x}, got {:#018x}",
            self.tick, self.subsystem, self.expected, self.actual
        )
    }
}

/// Compare two labeled traces tick by tick, and within each tick subsystem by
/// subsystem (in order), returning the earliest disagreement.
///
/// Each element of a trace is the `parts()` of that tick's
/// [`LabeledDigest`](crate::world_hash::LabeledDigest). At each tick the two
/// part-lists are walked in parallel:
/// - a differing hash at the same label ⇒ that subsystem diverged;
/// - the same position labeled differently ⇒ `"<label-mismatch>"` (the traces
///   aren't describing the same set of subsystems — itself a bug);
/// - one list shorter than the other ⇒ `"<missing>"` at the extra position.
///
/// A tick present in only one trace (length mismatch between the two traces)
/// reports `"<missing>"` at that tick. `Ok(())` iff every tick's part-list is
/// identical in labels, order, and hashes.
pub fn first_divergence_labeled(
    expected: &[&[(&'static str, u64)]],
    actual: &[&[(&'static str, u64)]],
) -> Result<(), LabeledDivergence> {
    let ticks = expected.len().max(actual.len());
    for tick in 0..ticks {
        match (expected.get(tick), actual.get(tick)) {
            (Some(e), Some(a)) => {
                let n = e.len().max(a.len());
                for i in 0..n {
                    match (e.get(i), a.get(i)) {
                        (Some(&(el, eh)), Some(&(al, ah))) => {
                            if el != al {
                                return Err(LabeledDivergence {
                                    tick,
                                    subsystem: "<label-mismatch>",
                                    expected: eh,
                                    actual: ah,
                                });
                            }
                            if eh != ah {
                                return Err(LabeledDivergence {
                                    tick,
                                    subsystem: el,
                                    expected: eh,
                                    actual: ah,
                                });
                            }
                        }
                        (Some(&(el, eh)), None) => {
                            return Err(LabeledDivergence {
                                tick,
                                subsystem: el,
                                expected: eh,
                                actual: 0,
                            });
                        }
                        (None, Some(&(al, ah))) => {
                            return Err(LabeledDivergence {
                                tick,
                                subsystem: al,
                                expected: 0,
                                actual: ah,
                            });
                        }
                        (None, None) => unreachable!("i < max(len) so at least one side is Some"),
                    }
                }
            }
            // One trace ran longer than the other: the whole tick is missing.
            (Some(_), None) | (None, Some(_)) => {
                return Err(LabeledDivergence {
                    tick,
                    subsystem: "<missing>",
                    expected: 0,
                    actual: 0,
                });
            }
            (None, None) => unreachable!("tick < max(len) so at least one side is Some"),
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Production desync reporting
// ─────────────────────────────────────────────────────────────────────────

/// A self-contained desync **repro bundle** — For Honor's operational lesson
/// (GDC 2019, "Networking For Honor"): a desync you cannot reproduce is a
/// desync you cannot fix, so the moment one is detected, capture everything a
/// developer needs to replay it *in the same artifact that reports it*.
///
/// The bundle carries the divergence itself, the subsystem localization when
/// labeled traces were available, the run's seed, and the window of inputs
/// leading up to (and including) the diverging tick. With the kit's
/// determinism guarantee, `seed + recent_inputs` replayed from the preceding
/// snapshot reproduces the desync exactly.
///
/// Build one with [`desync_report`] (plain traces) or
/// [`desync_report_labeled`] (subsystem-labeled traces).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesyncReport<I> {
    /// Where the traces first disagreed (tick + expected/actual hashes).
    pub divergence: Divergence,
    /// The first diverging subsystem, when the traces were labeled
    /// ([`desync_report_labeled`]); `None` for plain `u64` traces.
    pub subsystem: Option<&'static str>,
    /// The run's identity — typically the world seed. Echoed verbatim from
    /// the caller so the report is replayable standalone.
    pub seed: u64,
    /// The inputs for the ticks leading up to and **including** the diverging
    /// tick (at most the `window` most recent; input *i* produced trace entry
    /// *i*, matching [`record_trace`]).
    pub recent_inputs: Vec<I>,
    /// The tick index of `recent_inputs[0]` — where the captured window
    /// starts. Replaying the window from a snapshot of tick
    /// `first_input_tick - 1` reproduces the divergence.
    pub first_input_tick: usize,
}

impl<I> core::fmt::Display for DesyncReport<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.divergence)?;
        if let Some(subsystem) = self.subsystem {
            write!(f, " (subsystem '{}')", subsystem)?;
        }
        write!(
            f,
            "; seed {:#018x}; {} input(s) captured from tick {}",
            self.seed,
            self.recent_inputs.len(),
            self.first_input_tick
        )
    }
}

/// Clip the input window for a divergence at `tick`: at most `window` inputs
/// ending at (and including) the diverging tick.
fn clip_input_window<I: Clone>(inputs: &[I], tick: usize, window: usize) -> (Vec<I>, usize) {
    // Inclusive end: input `tick` produced the diverging trace entry. The
    // divergence may lie beyond the inputs we hold (length-mismatch case).
    let end = (tick + 1).min(inputs.len());
    let first = end.saturating_sub(window);
    (inputs[first..end].to_vec(), first)
}

/// Compare two plain replay traces and, if they diverge, bundle the evidence
/// into a [`DesyncReport`]: the divergence, the run's `seed`, and the at most
/// `window` most recent inputs up to and including the diverging tick.
/// Returns `None` iff the traces are identical.
///
/// `inputs` is the same slice the traces were recorded from ([`record_trace`]
/// alignment: input *i* → trace entry *i*). A `window` of `0` captures no
/// inputs (the report still localizes the tick).
pub fn desync_report<I: Clone>(
    expected: &[u64],
    actual: &[u64],
    seed: u64,
    inputs: &[I],
    window: usize,
) -> Option<DesyncReport<I>> {
    let divergence = first_divergence(expected, actual).err()?;
    let (recent_inputs, first_input_tick) = clip_input_window(inputs, divergence.tick, window);
    Some(DesyncReport {
        divergence,
        subsystem: None,
        seed,
        recent_inputs,
        first_input_tick,
    })
}

/// [`desync_report`] for **labeled** traces: additionally pins the first
/// diverging subsystem (via [`first_divergence_labeled`]), so the report
/// names both the tick and the subsystem to look at.
pub fn desync_report_labeled<I: Clone>(
    expected: &[&[(&'static str, u64)]],
    actual: &[&[(&'static str, u64)]],
    seed: u64,
    inputs: &[I],
    window: usize,
) -> Option<DesyncReport<I>> {
    let labeled = first_divergence_labeled(expected, actual).err()?;
    let (recent_inputs, first_input_tick) = clip_input_window(inputs, labeled.tick, window);
    Some(DesyncReport {
        divergence: Divergence {
            tick: labeled.tick,
            expected: labeled.expected,
            actual: labeled.actual,
        },
        subsystem: Some(labeled.subsystem),
        seed,
        recent_inputs,
        first_input_tick,
    })
}

/// What a session should do about a confirmed desync — the recovery
/// vocabulary from For Honor's production postmortem (GDC 2019). The kit
/// deliberately ships the *vocabulary*, not the decision: which policy
/// applies is a game/netcode judgement (how authoritative is each peer, can
/// state be re-sent, how disruptive is a kick), so the session layer picks a
/// variant and acts on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DesyncPolicy {
    /// Re-send authoritative state to the diverged peer and resume — least
    /// disruptive; requires a trusted source of truth and transferable state.
    Resync,
    /// Remove the diverged peer and continue the session for everyone else.
    Kick,
    /// End the session for all peers — the honest option when no peer is
    /// authoritative (pure lockstep) and the simulations cannot be reconciled.
    Disband,
}

impl core::fmt::Display for DesyncPolicy {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            DesyncPolicy::Resync => "resync",
            DesyncPolicy::Kick => "kick",
            DesyncPolicy::Disband => "disband",
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Persistent trace format
// ─────────────────────────────────────────────────────────────────────────

/// Magic bytes identifying a persistent replay trace written by
/// [`encode_trace`]: ASCII `IZTR`. Deliberately distinct from the save-file
/// magic (`b"IZNG"`, see [`crate::savefile`]) — the two formats share a
/// header layout, but a trace must never be loadable as a game save or vice
/// versa.
const TRACE_MAGIC: &[u8; 4] = b"IZTR";

/// Header size of the persistent trace format: magic(4) + version(4) +
/// checksum(8) + payload length(4). The same framing layout as
/// [`crate::savefile`], so the two decoders fail in the same ways.
const TRACE_HEADER_LEN: usize = 20;

/// Version of the persistent replay-trace format written by [`encode_trace`]
/// and required by [`decode_trace`]. Bump this — never silently change the
/// layout — when the payload schema changes; [`decode_trace`] reports the
/// stored version via [`TraceError::UnsupportedVersion`], so a caller can
/// distinguish "written by a newer build" from "corrupt".
pub const TRACE_FORMAT_VERSION: u32 = 1;

/// A replay trace in **persistent** form: everything needed to re-verify a
/// recorded run later, in another process, or on another machine.
///
/// - `seed` — the run's identity: typically the world seed the simulation
///   was constructed from (the same value [`DesyncReport`] echoes).
/// - `inputs` — one opaque byte frame per tick. The kit cannot know a
///   game's input type, so each input is stored as the bytes the game
///   encodes it to; turning frames back into inputs is the caller's half of
///   the contract — the same split [`crate::savefile`] uses for save
///   payloads.
/// - `expected` — the per-tick state hashes [`record_trace`] produced for
///   those inputs: input *i* produced hash *i*, in tick order.
///
/// `inputs` and `expected` normally have the same length. The format stores
/// the two counts independently, so a mismatched pair round-trips as-is —
/// [`check_trace`] already reads a length mismatch as a divergence at the
/// first missing tick, which is the honest interpretation of such a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayTrace {
    /// The run's seed, echoed verbatim.
    pub seed: u64,
    /// One opaque input frame per tick, in tick order.
    pub inputs: Vec<Vec<u8>>,
    /// The expected per-tick state hashes, in tick order.
    pub expected: Vec<u64>,
}

/// Encode a [`ReplayTrace`] into its canonical byte form.
///
/// The encoding is a pure function of the trace value: the same trace
/// always produces the same bytes, on every platform. All integers are
/// little-endian (never native-endian — the rule SPEC.md G10 states for
/// world hashes applies to bytes that outlive the process too), and the
/// payload is FNV-1a checksummed so truncation or corruption is caught by
/// [`decode_trace`] before any of it is trusted.
///
/// Layout:
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ magic[4] │ version[4LE] │ checksum[8LE] │ payload_len[4LE] │     │
/// ├──────────────────────────────────────────────────────────────────┤
/// │ seed[8LE] │ input_count[4LE] │ per input: len[4LE] + bytes │     │
/// │ hash_count[4LE] │ per hash: hash[8LE]                            │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
///
/// This function only produces bytes; writing them to a file (or anywhere
/// else) is the caller's job — the kit reads no ambient input itself.
pub fn encode_trace(trace: &ReplayTrace) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&trace.seed.to_le_bytes());
    payload.extend_from_slice(&(trace.inputs.len() as u32).to_le_bytes());
    for frame in &trace.inputs {
        payload.extend_from_slice(&(frame.len() as u32).to_le_bytes());
        payload.extend_from_slice(frame);
    }
    payload.extend_from_slice(&(trace.expected.len() as u32).to_le_bytes());
    for hash in &trace.expected {
        payload.extend_from_slice(&hash.to_le_bytes());
    }
    let mut hasher = Fnv1a::new();
    hasher.write_bytes(&payload);
    let checksum = hasher.finish();
    let mut out = Vec::with_capacity(TRACE_HEADER_LEN + payload.len());
    out.extend_from_slice(TRACE_MAGIC);
    out.extend_from_slice(&TRACE_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// Decode a [`ReplayTrace`] previously written by [`encode_trace`].
///
/// The decoder is total: truncated buffers, hostile declared lengths, and
/// arbitrary garbage all produce a [`TraceError`], never a panic and never
/// an allocation sized by an unvalidated count (each declared count is
/// first bounded by the bytes actually present). Only the payload slice
/// named by the header is parsed; bytes after it in `data` are ignored,
/// matching [`crate::savefile::load_bytes`].
pub fn decode_trace(data: &[u8]) -> Result<ReplayTrace, TraceError> {
    if data.len() < TRACE_HEADER_LEN {
        return Err(TraceError::TooShort);
    }
    if &data[0..4] != TRACE_MAGIC {
        return Err(TraceError::BadMagic);
    }
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version != TRACE_FORMAT_VERSION {
        return Err(TraceError::UnsupportedVersion(version));
    }
    let checksum = u64::from_le_bytes([
        data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
    ]);
    let payload_len = u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;
    // `data.len() >= TRACE_HEADER_LEN` is guaranteed above, so the
    // subtraction cannot underflow. Comparing this way — rather than
    // `data.len() < TRACE_HEADER_LEN + payload_len` — avoids overflowing
    // the addition when a hostile header declares a `payload_len` near
    // `usize::MAX` on 32-bit targets (the same reasoning as `savefile`).
    if payload_len > data.len() - TRACE_HEADER_LEN {
        return Err(TraceError::TooShort);
    }
    let payload = &data[TRACE_HEADER_LEN..TRACE_HEADER_LEN + payload_len];
    let mut hasher = Fnv1a::new();
    hasher.write_bytes(payload);
    if hasher.finish() != checksum {
        return Err(TraceError::ChecksumMismatch);
    }
    parse_trace_payload(payload)
}

/// Parse the (already checksum-validated) payload of a trace file. Every
/// read is preceded by a bounds check against the bytes remaining, so a
/// structurally broken payload yields [`TraceError::TooShort`] rather than
/// a panic.
fn parse_trace_payload(payload: &[u8]) -> Result<ReplayTrace, TraceError> {
    // seed(8) + input count(4) is the minimum payload prefix.
    if payload.len() < 12 {
        return Err(TraceError::TooShort);
    }
    let seed = u64::from_le_bytes([
        payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
        payload[7],
    ]);
    let input_count =
        u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]) as usize;
    let mut pos = 12usize;
    // Every frame costs at least its 4-byte length prefix, so a count larger
    // than `remaining / 4` names frames this buffer cannot contain. Checking
    // before `with_capacity` is what keeps a hostile count from forcing an
    // allocation the input never backed (the count may still be a lie about
    // the frames' *contents* — the per-frame checks below catch that).
    if input_count as u64 * 4 > (payload.len() - pos) as u64 {
        return Err(TraceError::TooShort);
    }
    let mut inputs = Vec::with_capacity(input_count);
    for _ in 0..input_count {
        if payload.len() - pos < 4 {
            return Err(TraceError::TooShort);
        }
        let frame_len = u32::from_le_bytes([
            payload[pos],
            payload[pos + 1],
            payload[pos + 2],
            payload[pos + 3],
        ]) as usize;
        pos += 4;
        if frame_len > payload.len() - pos {
            return Err(TraceError::TooShort);
        }
        inputs.push(payload[pos..pos + frame_len].to_vec());
        pos += frame_len;
    }
    if payload.len() - pos < 4 {
        return Err(TraceError::TooShort);
    }
    let hash_count = u32::from_le_bytes([
        payload[pos],
        payload[pos + 1],
        payload[pos + 2],
        payload[pos + 3],
    ]) as usize;
    pos += 4;
    // Hashes are fixed-size, so this bound is exact: a count that does not
    // fit the remaining bytes is rejected before any of it is read.
    if hash_count as u64 * 8 > (payload.len() - pos) as u64 {
        return Err(TraceError::TooShort);
    }
    let mut expected = Vec::with_capacity(hash_count);
    for _ in 0..hash_count {
        expected.push(u64::from_le_bytes([
            payload[pos],
            payload[pos + 1],
            payload[pos + 2],
            payload[pos + 3],
            payload[pos + 4],
            payload[pos + 5],
            payload[pos + 6],
            payload[pos + 7],
        ]));
        pos += 8;
    }
    if pos != payload.len() {
        return Err(TraceError::TrailingBytes);
    }
    Ok(ReplayTrace {
        seed,
        inputs,
        expected,
    })
}

/// Errors returned by [`decode_trace`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceError {
    /// The buffer is shorter than the header, the declared payload length
    /// exceeds the buffer, or a declared count/length inside the payload
    /// exceeds the bytes remaining.
    TooShort,
    /// The first four bytes do not match the trace magic `b"IZTR"` — the
    /// buffer is not a replay trace at all.
    BadMagic,
    /// The file's format version is not the version this build reads
    /// ([`TRACE_FORMAT_VERSION`]). The stored version is carried so the
    /// caller can report or migrate it.
    UnsupportedVersion(u32),
    /// The payload's FNV-1a hash does not match the stored checksum.
    ChecksumMismatch,
    /// The payload has bytes left over after the last declared hash — it
    /// was not produced by [`encode_trace`].
    TrailingBytes,
}

impl TraceError {
    /// Returns `true` when the error indicates a wrong-file condition
    /// rather than data corruption — i.e. the slot should be treated as
    /// empty rather than damaged. Only [`BadMagic`](Self::BadMagic)
    /// qualifies, mirroring [`crate::savefile::LoadError::is_recoverable`].
    #[inline]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, TraceError::BadMagic)
    }

    /// A short human-readable description of the error, suitable for
    /// logging. Does not allocate; returns a `'static str`.
    #[inline]
    pub fn message(&self) -> &'static str {
        match self {
            TraceError::TooShort => "replay trace too short",
            TraceError::BadMagic => "not a replay trace (bad magic)",
            TraceError::UnsupportedVersion(_) => "unsupported replay trace format version",
            TraceError::ChecksumMismatch => "replay trace corrupted (checksum mismatch)",
            TraceError::TrailingBytes => "replay trace has trailing bytes",
        }
    }
}

impl core::fmt::Display for TraceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TraceError::TooShort => write!(
                f,
                "replay trace too short or a declared length exceeds the buffer"
            ),
            TraceError::BadMagic => {
                write!(f, "replay trace magic mismatch (expected b\"IZTR\")")
            }
            TraceError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "unsupported replay trace format version {v} (this build reads version {})",
                    TRACE_FORMAT_VERSION
                )
            }
            TraceError::ChecksumMismatch => {
                write!(
                    f,
                    "replay trace checksum mismatch: payload may be corrupted"
                )
            }
            TraceError::TrailingBytes => {
                write!(f, "replay trace payload has trailing bytes")
            }
        }
    }
}

impl std::error::Error for TraceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed;
    use crate::rng::SplitMix64;
    use crate::world_hash::Fnv1a;

    /// A tiny but representative simulation state: a fixed-point position driven
    /// by a seeded RNG. Built only from kit value types, so it hashes for free.
    #[derive(Clone)]
    struct Sim {
        pos: Fixed,
        rng: SplitMix64,
    }

    impl DetHash for Sim {
        fn det_hash(&self, hasher: &mut Fnv1a) {
            self.pos.det_hash(hasher);
            self.rng.det_hash(hasher);
        }
    }

    fn new_sim() -> Sim {
        Sim {
            pos: Fixed::ZERO,
            rng: SplitMix64::new(0x1234),
        }
    }

    // Step: move by `input` units plus a small random jitter from the stream.
    fn step(s: &mut Sim, input: &i32) {
        let jitter = s.rng.range(0, 3);
        s.pos = s.pos + Fixed::from_int(*input + jitter);
    }

    #[test]
    fn test_divergence_display_contains_tick_and_hashes() {
        let d = Divergence {
            tick: 7,
            expected: 0xDEAD_BEEF_0000_1234,
            actual: 0xCAFE_BABE_5678_9ABC,
        };
        let s = d.to_string();
        assert!(s.contains("7"), "tick must appear in output");
        assert!(s.contains("0xdeadbeef00001234") || s.contains("0xDEADBEEF00001234"));
    }

    #[test]
    fn test_record_trace_is_reproducible() {
        let inputs = [1, -2, 3, 0, 5];
        let a = record_trace(&mut new_sim(), &inputs, step);
        let b = record_trace(&mut new_sim(), &inputs, step);
        assert_eq!(a, b, "same seed + inputs must reproduce the trace");
        assert_eq!(a.len(), inputs.len());
    }

    #[test]
    fn test_check_trace_accepts_a_faithful_replay() {
        let inputs = [4, 4, 4, 4];
        let expected = record_trace(&mut new_sim(), &inputs, step);
        assert_eq!(
            check_trace(&mut new_sim(), &inputs, &expected, step),
            Ok(())
        );
    }

    #[test]
    fn test_check_trace_localises_a_divergence() {
        let inputs = [1, 2, 3, 4, 5];
        let mut expected = record_trace(&mut new_sim(), &inputs, step);
        // Corrupt the hash at tick 2 → divergence must be reported there.
        expected[2] ^= 0xFFFF;
        match check_trace(&mut new_sim(), &inputs, &expected, step) {
            Err(d) => assert_eq!(d.tick, 2),
            Ok(()) => panic!("expected a divergence at tick 2"),
        }
    }

    #[test]
    fn test_first_divergence_edge_cases() {
        assert_eq!(first_divergence(&[1, 2, 3], &[1, 2, 3]), Ok(()));
        assert_eq!(
            first_divergence(&[1, 2, 3], &[1, 9, 3]),
            Err(Divergence {
                tick: 1,
                expected: 2,
                actual: 9
            })
        );
        // Shorter actual diverges at the first missing tick.
        assert_eq!(
            first_divergence(&[1, 2, 3], &[1, 2]),
            Err(Divergence {
                tick: 2,
                expected: 3,
                actual: 0
            })
        );
    }

    #[test]
    fn test_resimulate_matches_inline_run_and_preserves_snapshot() {
        let inputs = [2, 7, -3];
        // Run inline to tick 1, snapshot, finish inline.
        let mut inline = new_sim();
        step(&mut inline, &inputs[0]);
        let snapshot = inline.clone();
        for input in &inputs[1..] {
            step(&mut inline, input);
        }
        // Rollback path: resimulate the tail from the snapshot.
        let rolled = resimulate(&snapshot, &inputs[1..], step);
        assert_eq!(hash_state(&rolled), hash_state(&inline));
        // Snapshot itself is untouched (still at tick 1).
        assert_ne!(hash_state(&snapshot), hash_state(&inline));
    }

    #[test]
    fn test_count_divergences_identical_traces_is_zero() {
        let trace = vec![1u64, 2, 3, 4];
        assert_eq!(count_divergences(&trace, &trace.clone()), 0);
    }

    #[test]
    fn test_count_divergences_all_differ() {
        let a = vec![1u64, 2, 3];
        let b = vec![4u64, 5, 6];
        assert_eq!(count_divergences(&a, &b), 3);
    }

    #[test]
    fn test_count_divergences_length_mismatch_counts_extra() {
        let longer = vec![1u64, 2, 3, 4];
        let shorter = vec![1u64, 2];
        // ticks 2 and 3 are in longer but not shorter → 2 divergences
        assert_eq!(count_divergences(&longer, &shorter), 2);
    }

    #[test]
    fn test_find_all_divergences_identical_is_empty() {
        let t = vec![1u64, 2, 3];
        assert!(find_all_divergences(&t, &t).is_empty());
    }

    #[test]
    fn test_find_all_divergences_returns_all() {
        let a = vec![1u64, 2, 3];
        let b = vec![1u64, 9, 3]; // tick 1 diverges
        let divs = find_all_divergences(&a, &b);
        assert_eq!(divs.len(), 1);
        assert_eq!(divs[0].tick, 1);
        assert_eq!(divs[0].expected, 2);
        assert_eq!(divs[0].actual, 9);
    }

    #[test]
    fn test_find_all_divergences_length_mismatch_included() {
        let longer = vec![1u64, 2, 3];
        let shorter = vec![1u64];
        let divs = find_all_divergences(&longer, &shorter);
        // ticks 1 and 2 are missing from shorter → 2 divergences
        assert_eq!(divs.len(), 2);
        assert_eq!(divs[0].tick, 1);
        assert_eq!(divs[1].tick, 2);
    }

    #[test]
    fn test_replay_ok_identical_traces() {
        let trace = vec![1u64, 2, 3, 4];
        assert!(replay_ok(&trace, &trace));
    }

    #[test]
    fn test_replay_ok_false_on_divergence() {
        let a = vec![1u64, 2, 3];
        let b = vec![1u64, 9, 3];
        assert!(!replay_ok(&a, &b));
    }

    #[test]
    fn test_replay_ok_empty_traces_are_ok() {
        assert!(replay_ok(&[], &[]));
    }

    #[test]
    fn test_divergence_percent_zero_for_identical_traces() {
        let t = vec![1u64, 2, 3, 4];
        assert_eq!(divergence_percent(&t, &t), 0);
    }

    #[test]
    fn test_divergence_percent_100_for_all_diverged() {
        let a = vec![1u64, 2, 3];
        let b = vec![9u64, 8, 7];
        assert_eq!(divergence_percent(&a, &b), 100);
    }

    #[test]
    fn test_divergence_percent_empty_returns_zero() {
        assert_eq!(divergence_percent(&[], &[]), 0);
    }

    // --- first_divergence_labeled ---

    #[test]
    fn test_labeled_identical_traces_are_ok() {
        let t0: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let t1: &[(&str, u64)] = &[("pos", 3), ("hp", 4)];
        let trace = [t0, t1];
        assert_eq!(first_divergence_labeled(&trace, &trace), Ok(()));
    }

    #[test]
    fn test_labeled_localizes_diverging_subsystem() {
        // Same tick, same labels/order, but the `hp` hash differs at tick 1.
        let e0: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let e1: &[(&str, u64)] = &[("pos", 3), ("hp", 4)];
        let a0: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let a1: &[(&str, u64)] = &[("pos", 3), ("hp", 99)];
        let got = first_divergence_labeled(&[e0, e1], &[a0, a1]);
        assert_eq!(
            got,
            Err(LabeledDivergence {
                tick: 1,
                subsystem: "hp",
                expected: 4,
                actual: 99,
            })
        );
    }

    #[test]
    fn test_labeled_reports_first_subsystem_by_position() {
        // Both `pos` and `hp` differ; the earlier position (`pos`) wins.
        let e: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let a: &[(&str, u64)] = &[("pos", 5), ("hp", 6)];
        let got = first_divergence_labeled(&[e], &[a]);
        assert_eq!(got.unwrap_err().subsystem, "pos");
    }

    #[test]
    fn test_labeled_detects_label_mismatch() {
        // Same position labeled differently: the traces don't describe the
        // same subsystem set — a structural bug, flagged distinctly.
        let e: &[(&str, u64)] = &[("pos", 1)];
        let a: &[(&str, u64)] = &[("velocity", 1)];
        let got = first_divergence_labeled(&[e], &[a]);
        assert_eq!(got.unwrap_err().subsystem, "<label-mismatch>");
    }

    #[test]
    fn test_labeled_detects_missing_subsystem_within_tick() {
        let e: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let a: &[(&str, u64)] = &[("pos", 1)]; // hp absent
        let got = first_divergence_labeled(&[e], &[a]).unwrap_err();
        assert_eq!(got.subsystem, "hp");
        assert_eq!(got.expected, 2);
        assert_eq!(got.actual, 0);
    }

    #[test]
    fn test_labeled_detects_missing_tick() {
        let e0: &[(&str, u64)] = &[("pos", 1)];
        let e1: &[(&str, u64)] = &[("pos", 2)];
        let a0: &[(&str, u64)] = &[("pos", 1)];
        let got = first_divergence_labeled(&[e0, e1], &[a0]).unwrap_err();
        assert_eq!(got.tick, 1);
        assert_eq!(got.subsystem, "<missing>");
    }

    #[test]
    fn test_labeled_integrates_with_labeled_digest() {
        use crate::world_hash::LabeledDigest;
        // Build two ticks from LabeledDigest, mutate one subsystem, confirm
        // the localizer names exactly it. This is the intended end-to-end use.
        let mut good = LabeledDigest::new();
        good.add("terrain", &[1u32, 2, 3][..])
            .add("actors", &[10u32, 20][..]);
        let good_parts = good.parts().to_vec();

        let mut bad = LabeledDigest::new();
        bad.add("terrain", &[1u32, 2, 3][..])
            .add("actors", &[10u32, 21][..]); // actors moved
        let bad_parts = bad.parts().to_vec();

        let expected = [good_parts.as_slice()];
        let actual = [bad_parts.as_slice()];
        let got = first_divergence_labeled(&expected, &actual).unwrap_err();
        assert_eq!(got.subsystem, "actors");
        assert_eq!(got.tick, 0);
    }

    // --- DesyncReport ---

    #[test]
    fn test_desync_report_none_when_traces_agree() {
        let trace = [1u64, 2, 3];
        let inputs = [10u8, 20, 30];
        assert!(desync_report(&trace, &trace, 0xABCD, &inputs, 8).is_none());
    }

    #[test]
    fn test_desync_report_captures_window_ending_at_divergence() {
        let expected = [1u64, 2, 3, 4, 5];
        let actual = [1u64, 2, 3, 99, 5]; // diverges at tick 3
        let inputs = [10u8, 11, 12, 13, 14];
        let report = desync_report(&expected, &actual, 7, &inputs, 3).unwrap();
        assert_eq!(report.divergence.tick, 3);
        assert_eq!(report.divergence.expected, 4);
        assert_eq!(report.divergence.actual, 99);
        assert_eq!(report.subsystem, None);
        assert_eq!(report.seed, 7);
        // Window of 3 ending at (and including) the diverging tick's input.
        assert_eq!(report.recent_inputs, vec![11, 12, 13]);
        assert_eq!(report.first_input_tick, 1);
    }

    #[test]
    fn test_desync_report_window_larger_than_history_starts_at_zero() {
        let expected = [1u64, 2];
        let actual = [1u64, 9];
        let inputs = [10u8, 11];
        let report = desync_report(&expected, &actual, 0, &inputs, 100).unwrap();
        assert_eq!(report.recent_inputs, vec![10, 11]);
        assert_eq!(report.first_input_tick, 0);
    }

    #[test]
    fn test_desync_report_zero_window_captures_no_inputs() {
        let expected = [1u64, 2];
        let actual = [1u64, 9];
        let inputs = [10u8, 11];
        let report = desync_report(&expected, &actual, 0, &inputs, 0).unwrap();
        assert!(report.recent_inputs.is_empty());
        assert_eq!(report.first_input_tick, 2);
    }

    #[test]
    fn test_desync_report_divergence_beyond_inputs_is_clamped() {
        // Length-mismatch divergence at a tick we hold no input for.
        let expected = [1u64, 2, 3];
        let actual = [1u64, 2];
        let inputs = [10u8, 11]; // only 2 inputs held
        let report = desync_report(&expected, &actual, 0, &inputs, 8).unwrap();
        assert_eq!(report.divergence.tick, 2);
        assert_eq!(report.recent_inputs, vec![10, 11]);
        assert_eq!(report.first_input_tick, 0);
    }

    #[test]
    fn test_desync_report_labeled_names_subsystem() {
        let e: &[(&str, u64)] = &[("terrain", 1), ("actors", 2)];
        let a: &[(&str, u64)] = &[("terrain", 1), ("actors", 5)];
        let ok: &[(&str, u64)] = &[("terrain", 1), ("actors", 2)];
        let inputs = [10u8, 11];
        let report = desync_report_labeled(&[ok, e], &[ok, a], 42, &inputs, 8).unwrap();
        assert_eq!(report.divergence.tick, 1);
        assert_eq!(report.subsystem, Some("actors"));
        assert_eq!(report.divergence.expected, 2);
        assert_eq!(report.divergence.actual, 5);
        assert_eq!(report.recent_inputs, vec![10, 11]);
    }

    #[test]
    fn test_desync_report_labeled_none_when_identical() {
        let t: &[(&str, u64)] = &[("terrain", 1)];
        let inputs: [u8; 1] = [10];
        assert!(desync_report_labeled(&[t], &[t], 0, &inputs, 4).is_none());
    }

    #[test]
    fn test_desync_report_display_mentions_tick_seed_and_subsystem() {
        let e: &[(&str, u64)] = &[("actors", 2)];
        let a: &[(&str, u64)] = &[("actors", 5)];
        let inputs = [10u8];
        let report = desync_report_labeled(&[e], &[a], 0xBEEF, &inputs, 4).unwrap();
        let text = report.to_string();
        assert!(text.contains("tick 0"), "{text}");
        assert!(text.contains("actors"), "{text}");
        assert!(text.contains("0x000000000000beef"), "{text}");
        assert!(text.contains("1 input(s) captured from tick 0"), "{text}");
    }

    #[test]
    fn test_desync_policy_display() {
        assert_eq!(DesyncPolicy::Resync.to_string(), "resync");
        assert_eq!(DesyncPolicy::Kick.to_string(), "kick");
        assert_eq!(DesyncPolicy::Disband.to_string(), "disband");
    }

    #[test]
    fn test_desync_report_replays_to_reproduce_the_divergence() {
        // End-to-end: record a good trace, corrupt one input on the "peer",
        // build the report, then use ONLY the report's window + a snapshot of
        // the tick before it to re-derive the diverging hash — proving the
        // bundle is a sufficient repro artifact.
        let inputs: Vec<u8> = (1..=10).collect();
        let mut reference = new_sim();
        let step = |s: &mut Sim, i: &u8| {
            let jitter = s.rng.below(3) as i32;
            s.pos = s.pos + Fixed::from_int(*i as i32 + jitter);
        };
        let expected = record_trace(&mut reference, &inputs, step);

        let mut peer_inputs = inputs.clone();
        peer_inputs[6] = 99; // corruption at tick 6
        let mut peer = new_sim();
        let actual = record_trace(&mut peer, &peer_inputs, step);

        let report = desync_report(&expected, &actual, 0x1234, &peer_inputs, 4).unwrap();
        assert_eq!(report.divergence.tick, 6);

        // Rebuild the pre-window snapshot by replaying the prefix, then replay
        // just the report's window: the final hash must equal the "actual"
        // (diverged) hash the report captured.
        let mut snapshot = new_sim();
        for input in &peer_inputs[..report.first_input_tick] {
            step(&mut snapshot, input);
        }
        let replayed = resimulate(&snapshot, &report.recent_inputs, step);
        assert_eq!(hash_state(&replayed), report.divergence.actual);
    }

    // --- Persistent trace format ---

    fn sample_trace() -> ReplayTrace {
        ReplayTrace {
            seed: 0x0102_0304_0506_0708,
            inputs: vec![vec![0xAA, 0xBB], Vec::new(), vec![1, 2, 3]],
            expected: vec![
                0x1111_2222_3333_4444,
                0x5555_6666_7777_8888,
                0x9999_AAAA_BBBB_CCCC,
            ],
        }
    }

    #[test]
    fn test_encode_trace_is_deterministic() {
        let trace = sample_trace();
        assert_eq!(encode_trace(&trace), encode_trace(&trace));
    }

    #[test]
    fn test_trace_roundtrip_empty() {
        let trace = ReplayTrace {
            seed: 42,
            inputs: Vec::new(),
            expected: Vec::new(),
        };
        let decoded = decode_trace(&encode_trace(&trace)).unwrap();
        assert_eq!(decoded, trace);
    }

    #[test]
    fn test_trace_roundtrip_with_frames() {
        let trace = sample_trace();
        let decoded = decode_trace(&encode_trace(&trace)).unwrap();
        assert_eq!(decoded, trace);
    }

    #[test]
    fn test_trace_roundtrip_mismatched_counts_roundtrip_as_is() {
        // The format stores the input count and the hash count independently:
        // a trace whose two columns disagree (a truncated recording, say)
        // must survive the round-trip unchanged — `check_trace` is the layer
        // that reads the mismatch as a divergence.
        let trace = ReplayTrace {
            seed: 7,
            inputs: vec![vec![1], vec![2]],
            expected: vec![0xFEED],
        };
        let decoded = decode_trace(&encode_trace(&trace)).unwrap();
        assert_eq!(decoded, trace);
    }

    #[test]
    fn test_decode_trace_too_short() {
        assert_eq!(decode_trace(&[]), Err(TraceError::TooShort));
        assert_eq!(decode_trace(&[0u8; 19]), Err(TraceError::TooShort));
    }

    #[test]
    fn test_decode_trace_bad_magic() {
        let mut data = encode_trace(&sample_trace());
        data[0] = 0xFF;
        assert_eq!(decode_trace(&data), Err(TraceError::BadMagic));
    }

    #[test]
    fn test_decode_trace_unsupported_version_reports_stored_version() {
        let mut data = encode_trace(&sample_trace());
        data[4..8].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(decode_trace(&data), Err(TraceError::UnsupportedVersion(99)));
    }

    #[test]
    fn test_decode_trace_checksum_mismatch_on_payload_corruption() {
        let mut data = encode_trace(&sample_trace());
        *data.last_mut().unwrap() ^= 0xFF;
        assert_eq!(decode_trace(&data), Err(TraceError::ChecksumMismatch));
    }

    #[test]
    fn test_decode_trace_declared_payload_len_u32_max_is_too_short() {
        // A hostile header declaring the maximum payload length must fail
        // cleanly, never panic (the subtraction-based check cannot wrap).
        let mut data = encode_trace(&sample_trace());
        data[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode_trace(&data), Err(TraceError::TooShort));
    }

    /// Reframe a hand-built payload with a correct header (magic, current
    /// version, real checksum), so payload-level guards can be tested
    /// without the checksum rejecting the buffer first.
    fn frame_payload(payload: &[u8]) -> Vec<u8> {
        let mut hasher = Fnv1a::new();
        hasher.write_bytes(payload);
        let mut out = Vec::new();
        out.extend_from_slice(TRACE_MAGIC);
        out.extend_from_slice(&TRACE_FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&hasher.finish().to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn test_decode_trace_hostile_input_count_is_too_short_not_huge_allocation() {
        // Declares u32::MAX input frames in a 12-byte payload. The count is
        // bounded by the bytes present before any allocation, so this
        // returns TooShort instead of trying to reserve room for 4 billion
        // frames.
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u64.to_le_bytes());
        payload.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            decode_trace(&frame_payload(&payload)),
            Err(TraceError::TooShort)
        );
    }

    #[test]
    fn test_decode_trace_hostile_hash_count_is_too_short() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u64.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes()); // no inputs
        payload.extend_from_slice(&u32::MAX.to_le_bytes()); // absurd hash count
        assert_eq!(
            decode_trace(&frame_payload(&payload)),
            Err(TraceError::TooShort)
        );
    }

    #[test]
    fn test_decode_trace_frame_len_beyond_payload_is_too_short() {
        // One frame declared, and its length prefix names more bytes than
        // the payload holds after it.
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u64.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes()); // one input
        payload.extend_from_slice(&1000u32.to_le_bytes()); // frame len 1000
        payload.extend_from_slice(&0u32.to_le_bytes()); // hash count 0
        assert_eq!(
            decode_trace(&frame_payload(&payload)),
            Err(TraceError::TooShort)
        );
    }

    #[test]
    fn test_decode_trace_trailing_bytes_rejected() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u64.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.push(0xEE); // not part of any declared section
        assert_eq!(
            decode_trace(&frame_payload(&payload)),
            Err(TraceError::TrailingBytes)
        );
    }

    #[test]
    fn decode_trace_never_panics_on_any_truncation_or_single_byte_corruption() {
        // The same discipline the save-file decoder is held to: every prefix
        // and every single-byte corruption of a valid file must yield a
        // value or an Err — never a panic.
        let full = encode_trace(&sample_trace());
        for n in 0..=full.len() {
            let _ = decode_trace(&full[..n]);
        }
        for i in 0..full.len() {
            let mut d = full.clone();
            d[i] ^= 0xFF;
            let _ = decode_trace(&d);
        }
        // Arbitrary garbage, deterministic and dependency-free.
        let mut g = vec![0xAAu8; 96];
        for seed in 0..512u32 {
            for (i, b) in g.iter_mut().enumerate() {
                *b = seed.wrapping_mul(31).wrapping_add(i as u32) as u8;
            }
            let _ = decode_trace(&g);
        }
    }

    #[test]
    fn test_trace_error_display_and_message_are_non_empty() {
        for e in [
            TraceError::TooShort,
            TraceError::BadMagic,
            TraceError::UnsupportedVersion(9),
            TraceError::ChecksumMismatch,
            TraceError::TrailingBytes,
        ] {
            assert!(!e.to_string().is_empty());
            assert!(!e.message().is_empty());
        }
        assert!(TraceError::UnsupportedVersion(9).to_string().contains('9'));
    }

    #[test]
    fn test_trace_error_is_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&TraceError::TooShort);
    }

    #[test]
    fn test_trace_error_only_bad_magic_is_recoverable() {
        assert!(TraceError::BadMagic.is_recoverable());
        assert!(!TraceError::TooShort.is_recoverable());
        assert!(!TraceError::ChecksumMismatch.is_recoverable());
        assert!(!TraceError::UnsupportedVersion(2).is_recoverable());
        assert!(!TraceError::TrailingBytes.is_recoverable());
    }

    #[test]
    fn test_decoded_trace_verifies_with_check_trace() {
        // The point of the format: record once, persist, decode later, and
        // the decoded columns drive `check_trace` exactly as the in-memory
        // ones did.
        let inputs: Vec<i32> = vec![3, -1, 4, 1, 5, -9, 2, 6];
        let expected = record_trace(&mut new_sim(), &inputs, step);
        let stored = ReplayTrace {
            seed: 0x1234,
            inputs: inputs.iter().map(|i| i.to_le_bytes().to_vec()).collect(),
            expected,
        };
        let decoded = decode_trace(&encode_trace(&stored)).unwrap();
        assert_eq!(decoded.seed, 0x1234);
        let decoded_inputs: Vec<i32> = decoded
            .inputs
            .iter()
            .map(|f| i32::from_le_bytes([f[0], f[1], f[2], f[3]]))
            .collect();
        assert_eq!(decoded_inputs, inputs);
        assert_eq!(
            check_trace(&mut new_sim(), &decoded_inputs, &decoded.expected, step),
            Ok(())
        );
    }

    #[test]
    fn test_decoded_trace_with_tampered_hash_diverges_at_that_tick() {
        let inputs: Vec<i32> = vec![1, 2, 3, 4];
        let mut stored = ReplayTrace {
            seed: 0x1234,
            inputs: inputs.iter().map(|i| i.to_le_bytes().to_vec()).collect(),
            expected: record_trace(&mut new_sim(), &inputs, step),
        };
        stored.expected[2] ^= 0xFFFF;
        let decoded = decode_trace(&encode_trace(&stored)).unwrap();
        let decoded_inputs: Vec<i32> = decoded
            .inputs
            .iter()
            .map(|f| i32::from_le_bytes([f[0], f[1], f[2], f[3]]))
            .collect();
        match check_trace(&mut new_sim(), &decoded_inputs, &decoded.expected, step) {
            Err(d) => assert_eq!(d.tick, 2),
            Ok(()) => panic!("expected a divergence at tick 2"),
        }
    }

    // --- Golden encoding (the on-disk bytes are the format) ---

    /// The trace whose exact encoding is pinned in `GOLDEN_TRACE`.
    fn golden_trace() -> ReplayTrace {
        ReplayTrace {
            seed: 0x0102_0304_0506_0708,
            inputs: vec![vec![0xAA, 0xBB], Vec::new()],
            expected: vec![0x1111_2222_3333_4444, 0x5555_6666_7777_8888],
        }
    }

    /// The exact bytes [`encode_trace`] writes for `golden_trace()`:
    /// header (magic `IZTR` | version 1 LE | FNV-1a checksum LE | payload
    /// len LE) then payload (seed LE | 2 inputs | 2 hashes). A diff here
    /// means the on-disk trace format changed — regenerate **only**
    /// alongside a deliberate `TRACE_FORMAT_VERSION` bump, never to "make
    /// the test pass". Regenerate via `print_golden_trace` (ignored).
    const GOLDEN_TRACE: &[u8] = &[
        0x49, 0x5a, 0x54, 0x52, 0x01, 0x00, 0x00, 0x00, 0x70, 0xac, 0xe4, 0x48, //
        0x8f, 0x03, 0x88, 0x2f, 0x2a, 0x00, 0x00, 0x00, 0x08, 0x07, 0x06, 0x05, //
        0x04, 0x03, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, //
        0xaa, 0xbb, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x44, 0x44, //
        0x33, 0x33, 0x22, 0x22, 0x11, 0x11, 0x88, 0x88, 0x77, 0x77, 0x66, 0x66, //
        0x55, 0x55,
    ];

    #[test]
    fn test_encode_trace_matches_golden_encoding() {
        let data = encode_trace(&golden_trace());
        assert_eq!(
            data.as_slice(),
            GOLDEN_TRACE,
            "trace framing changed — stored traces would break. Regenerate \
             GOLDEN_TRACE only with a deliberate TRACE_FORMAT_VERSION bump."
        );
    }

    #[test]
    fn test_decode_trace_reads_golden_bytes() {
        // A build must read the exact bytes a prior build wrote, not merely
        // bytes it just produced itself.
        let decoded = decode_trace(GOLDEN_TRACE).unwrap();
        assert_eq!(decoded, golden_trace());
    }

    #[test]
    fn test_golden_trace_layout_offsets() {
        assert_eq!(&GOLDEN_TRACE[0..4], b"IZTR", "magic offset/value");
        assert_eq!(
            u32::from_le_bytes(GOLDEN_TRACE[4..8].try_into().unwrap()),
            TRACE_FORMAT_VERSION,
            "version offset/endianness"
        );
        assert_eq!(
            u64::from_le_bytes(GOLDEN_TRACE[20..28].try_into().unwrap()),
            0x0102_0304_0506_0708,
            "seed offset/endianness"
        );
    }

    /// Prints the current encoding of the golden fixture for pasting into
    /// `GOLDEN_TRACE`. Ignored by default; run only when intentionally
    /// bumping the trace format version.
    #[test]
    #[ignore]
    fn print_golden_trace() {
        let data = encode_trace(&golden_trace());
        print!("const GOLDEN_TRACE: &[u8] = &[");
        for (i, b) in data.iter().enumerate() {
            if i % 12 == 0 {
                print!("\n    ");
            }
            print!("0x{b:02x}, ");
        }
        println!("\n];");
    }
}
