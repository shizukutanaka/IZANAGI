# The determinism guarantee

This document states exactly what IZANAGI guarantees about replay, what it
does not, and how the guarantee is enforced. The normative form of the rules
is [`izanagi_kit/SPEC.md`](./izanagi_kit/SPEC.md) §2 (invariants G1–G11);
this page is the reader-facing statement of the same facts.

## What is guaranteed

For a simulation built on `izanagi_kit`: **given the same initial state and
the same input sequence, the simulation produces a bit-identical sequence of
world hashes on every OS, CPU and compiler target the crate builds on** —
32-bit and 64-bit, little-endian and big-endian, debug profile and release
profile alike.

The world hash is FNV-1a/64 folded over the state's canonical bytes
(`world_hash::DetHash`). Bit-identical hashes mean bit-identical simulation
states, which is what replay, lockstep networking and snapshot-as-checksum
testing all reduce to.

This is a property of the default configuration, not an opt-in mode. There
is no feature flag that turns determinism on, and none that turns it off.

## Why it holds

The guarantee rests on the simulation layer having no source of variation
left to disagree about:

- **Fixed-point only.** Simulation math is `fixed::Fixed` — Q16.16 with
  saturating arithmetic over `i64` intermediates — and integer math. There
  is no `f32`/`f64` anywhere in the kit's production code for two machines
  to round differently.
- **Canonical order.** State is folded into the hash in a canonical
  (sorted) order. Collections whose iteration order is process-dependent
  are not used in the simulation layer except under a named, reasoned
  allowlist (see below).
- **Seeded randomness only.** Randomness flows through seeded generators
  (`SplitMix64`, xoshiro) with named independent sub-streams, consumed in a
  fixed order. Nothing in the kit seeds from the wall clock.
- **Integer time.** `timestep` advances in integer nanoseconds.
- **Width and endianness cannot leak in.** No `usize`/`isize` value has a
  `DetHash` implementation, and every hash write is explicitly
  little-endian (SPEC G9/G10).
- **The build cannot fork the code.** Library code does not branch on
  target, profile or feature via `cfg`, and reads no build-time environment
  (SPEC G11), so "the same code" is a fact about the source, not a hope
  about how it was compiled.
- **No ambient inputs.** The kit library cannot read the filesystem,
  processes, std I/O, environment variables, the wall clock, or thread
  state. Those are inputs a replay log cannot reproduce, so they are
  mechanically banned from the library (the one legitimate consumer, the
  `gamec` binary, is a separate crate root and is checked separately).

### Content definitions cannot express nondeterminism

The `.game` content format (grammar: SPEC.md §9.1, kept in 1:1 agreement
with `src/parser.rs` by a dedicated test) closes the authoring side of the
same hole — not by checking definitions and rejecting bad ones, but by
having no way to write them:

- Every numeric token in the grammar is an integer: `int` (i32) or `uint`
  (u32). `1.5`, `1e3`, `NaN` and `inf` are not float literals the format
  warns about; they are not tokens the grammar can produce, and a line
  containing one fails to parse.
- The grammar has nine keywords (`prefab`, `tile`, `level`, `glyph`,
  `color`, `stat`, `flag`, `row`, `spawn`). There is no seed or random
  directive: a simulation's seed is a host-side API parameter, never bundle
  data. There are no tokens that refer to the time, the environment or the
  locale, and the parser is a pure function of the source text.
- `Content`'s collections are declaration-ordered `Vec`s and sorted
  `BTreeMap`s, so a definition cannot create a structure whose iteration
  order is undefined.

## How it is enforced, and how to re-verify it

The guarantee is not a code-review convention. It is re-verified
mechanically on every run of the gate:

- `izanagi_kit/tests/no_float_in_sim.rs` scans every production line of
  the kit and rejects `f32`/`f64` tokens and decimal float literals.
- `izanagi_kit/tests/no_nondeterminism_in_sim.rs` bans wall-clock reads,
  threading, environment reads, pointer identity and `DefaultHasher`
  outright, and permits `HashMap`/`HashSet` only at named sites with a
  written reason and an exact expected count.
- `izanagi_kit/tests/hashes_are_width_independent.rs` and
  `hashes_are_endian_independent.rs` pin SPEC G9/G10 statically — the
  current CI targets are all little-endian, so these are precisely the
  divergences a test *run* could never catch.
- Two end-to-end simulations pin their final world hashes:
  `PINNED_FINAL_HASH = 0xd1a9236e96a2c802`
  (`izanagi_kit/tests/determinism.rs`) and
  `PINNED_ROGUELIKE_HASH = 0x5286d1420200fe66`
  (`izanagi_kit/tests/roguelike_sim.rs`). `tools/gate.sh` runs both in the
  debug and the release profile, because arithmetic that wraps silently
  would otherwise only diverge in the profile that ships.
- The gate is one script, `tools/gate.sh`, and it is the only definition of
  "green": the pre-push hook and the proposed CI run exactly it. Among its
  stages, every shipped example is run twice and required to produce
  byte-identical output, and the example that hosts a kit simulation
  inside the engine must reproduce its pinned integration hash.

Run it yourself, from the repository root:

```text
tools/gate.sh
```

Or just the pinned hashes, in both profiles:

```text
cargo test -p izanagi_kit --test determinism --test roguelike_sim
cargo test --release -p izanagi_kit --test determinism --test roguelike_sim
```

For your own simulation, the same machinery is shipped as a product, not
kept as an internal test: implement `sim::Simulation` and the crate's
interrogation modules — audit, bounded model checking, deterministic
simulation testing, rollback sync tests — apply to it.
`cargo run -p izanagi_kit --example verify_pipeline_demo` walks one small
simulation through the whole pipeline.

One honest limit of the enforcement: the float scanner cannot see a fully
type-inferred float binding that carries no `f32`/`f64` token and no float
literal. In practice a float can only reach hashed state through a typed
field or a cast, both of which the scan does see — but the blind spot is
documented here rather than left for a reader to discover.

## What is not guaranteed

- **The engine crate (`izanagi`).** The engine renders and animates in
  `f32`, including its `physics` module. Seeding its `Rng` gives you a
  reproducible run on one machine; it is explicitly *not* bit-exact across
  different CPUs or compilers. Eight engine modules are float-free
  (`assets`, `ecs`, `error`, `event`, `log`, `save`, `scene`, `state`),
  which is necessary for their state to take part in a replay but not
  sufficient — the stepping and hashing live in the kit. The composition
  is the intended shape: simulate in the kit, render in the engine, and
  never let a float flow back.
- **Presentation.** Rendered output, frame pacing and wall-clock timing
  are not part of the guarantee; the state is.
- **Transport.** The kit ships no networking. Whatever carries inputs
  between machines is outside the guarantee — what is guaranteed is that
  identical delivered input sequences produce identical simulations at
  every end.
- **Across crate versions.** Hashes, traces and saves are **not** promised
  stable across `izanagi_kit` versions (see the hash stability policy in
  `src/world_hash.rs`). Changing the shape of a `DetHash` implementation
  is breaking for persisted artifacts: the `savefile` header version is
  bumped in the same commit, and callers who persist hashes should record
  the crate version alongside them.
- **Your code, outside the kit's surface.** If your own step function
  reads a wall clock or folds a `HashMap`'s iteration order into state,
  the kit cannot stop you. It can catch you: that is what `sim::audit` and
  the rest of the interrogation family are for.

## An honest note on floats

Determinism on floats is achievable — with strict discipline: identical
toolchains and settings on every client, hand-written replacements for
transcendental functions, and constant vigilance. Production precedent
exists; the developers of Mushroom Wars 2 have documented a decade of
cross-platform lockstep and rollback netcode run that way
([write-up](https://dev.to/chromealex/ten-years-of-rollback-netcode-in-a-cross-platform-rts-without-fixed-point-2c71)).

IZANAGI's claim is therefore not "floats make determinism impossible". It
is narrower and stronger: here, determinism does not depend on the user's
discipline. It is the default, it follows from the structure of the code,
and it is re-verified mechanically every time the gate runs.

## Supply chain

- **Zero runtime dependencies.** Both crates' `[dependencies]` sections
  are empty, and the workspace lockfile resolves to the two workspace
  crates and nothing else. The code a user has to audit is the crates' own
  code plus the standard library.
- **`#![forbid(unsafe_code)]`** in both crates — not a lint level a leaf
  module can relax, a hard compiler error.
- **Declared, checked MSRVs.** `izanagi` declares `rust-version = "1.65"`,
  `izanagi_kit` declares `rust-version = "1.75"`, and a static scanner
  (`izanagi_kit/tests/msrv_is_respected.rs`) keeps the sources inside
  them.

## Related documents

- [`izanagi_kit/SPEC.md`](./izanagi_kit/SPEC.md) — the normative
  invariants (G1–G11) and per-module contracts.
- [`izanagi_kit/README.md`](./izanagi_kit/README.md) — the kit's module
  table and quickstart.
- [`izanagi/README.md`](./izanagi/README.md) — the engine, including its
  own statement of the determinism boundary.
- [`SECURITY.md`](./SECURITY.md) — how to report a vulnerability.
