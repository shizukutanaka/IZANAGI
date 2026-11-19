# Desync FAQ

Practical answers for when two runs of an `izanagi_kit` simulation disagree.
The guarantee itself — what is promised, why it holds, how it is enforced —
is stated in [`DETERMINISM.md`](../DETERMINISM.md). This page is about what to
do when your own simulation does not live up to it.

## What counts as a desync?

Two runs that were given the same initial state and the same input sequence
produce different per-tick state hashes. "Same" means bit-identical: the
kit compares `u64` world hashes (`world_hash::hash_state`), never rendered
output or timing.

## How do I find the first diverging tick?

Record a trace on the reference run and check the other run against it:

- `replay::record_trace` returns one state hash per tick.
- `replay::check_trace` re-runs the inputs and returns the first
  `Divergence` (tick, expected hash, actual hash).
- `replay::first_divergence` compares two already-recorded traces, e.g. one
  from each peer.

The first diverging tick is where to start looking. Every later tick is
downstream of it and usually tells you nothing new.

## The tick is known. Which part of the state is wrong?

Hash each subsystem under a stable label with `world_hash::LabeledDigest`
and compare the labeled traces with `replay::first_divergence_labeled`. The
result names the tick **and** the first subsystem that differs (for example
`enemies`), instead of one opaque `u64`.

## How do I make a desync reproducible for someone else?

Capture it at the moment it is detected:

- `replay::desync_report` (or `desync_report_labeled`) bundles the
  divergence, the run's seed, the subsystem when known, and a window of
  the inputs up to and including the diverging tick.
- `replay::encode_trace` / `decode_trace` persist a whole run — seed,
  per-tick input frames and expected hashes — as canonical bytes that
  another process or machine can re-verify with `check_trace`. The byte
  layout and its compatibility rules are in
  [`FORMAT_COMPATIBILITY.md`](./FORMAT_COMPATIBILITY.md).

## My simulation desyncs, but the kit's pinned hashes pass. Why?

The pinned hashes (`PINNED_FINAL_HASH`, `PINNED_ROGUELIKE_HASH`) prove the
kit's own simulations are deterministic. They say nothing about code you
write around the kit. The usual causes, all outside the kit's surface:

- Reading the wall clock, the environment, or thread state inside `step`.
- Folding the iteration order of a `HashMap`/`HashSet` into state.
- Using `f32`/`f64` in state that is hashed or that feeds hashed state.
- Consuming randomness in an order that depends on any of the above.
- Inputs that are not actually identical on both ends (a transport issue —
  the kit ships no networking).

## How do I catch these before they reach players?

Implement `sim::Simulation` for your state and run `sim::audit`. It runs the
inputs twice and compares, and with a non-zero `check_distance` also runs
a rollback sync test (`sim::sync_test`), which catches state that does not
survive snapshot-and-resimulate. `world_hash::hash_covers` catches the
opposite failure: a field that is never folded into the hash, so a real
difference stays invisible.

## Can a `.game` content file introduce nondeterminism?

No. The format has no way to express it: numeric tokens are `int`/`uint`
only, and there is no seed or random directive. See the "Content definitions
cannot express nondeterminism" section of
[`DETERMINISM.md`](../DETERMINISM.md).

## A desync is confirmed in a live session. What now?

`replay::DesyncPolicy` names the three recovery options — `Resync`, `Kick`
and `Disband`. The kit ships the vocabulary, not the decision: which one
applies depends on whether any peer is authoritative and whether state can
be re-sent, and that is a session-layer choice.

## Will a trace recorded today replay on a newer kit version?

Same code, any machine: yes. Across `izanagi_kit` versions: not promised.
See [`FORMAT_COMPATIBILITY.md`](./FORMAT_COMPATIBILITY.md).
