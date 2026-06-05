# izanagi_kit — 改善点の洗い出し (Improvement Backlog)

Findings from a full-source review of `izanagi_kit`, cross-referenced against
comparable software (Bevy / EnTT / Flecs ECS, bracket-lib / doryen-rs terminal
roguelike toolkits, the `fixed` / `cordic` crates) and the literature
(arXiv + practitioner sources). Phase 1 is **implemented in this branch**;
Phases 2–4 are proposed follow-ups. Every item keeps the kit's rules:
zero runtime dependencies, `#![forbid(unsafe_code)]`, tests with new code.

---

## Phase 1 — Confirmed bugs (FIXED in this branch)

The kit's two headline guarantees — *panic-free / bounded parser* and
*saturating, never-sign-flipping fixed-point* — were violated on reachable
inputs. The existing tests and the structure-aware fuzzer only exercise the
safe region (ASCII glyphs, `from_int` values in `[-3, 3]`, 7-char ASCII
colors), so none could reach these. Each fix ships with a regression test that
exercises exactly the input the old suite avoided.

| # | Location | Bug | Fix |
|---|----------|-----|-----|
| 1 | `content.rs::parse_color` | A 7-**byte** color containing a multi-byte UTF-8 char (`#aéABC`) passed the `len()==7` check, then fixed-offset `&s[i..i+2]` slicing cut through a char boundary → **panic**. Defeated the panic-free parser. | Parse hex nibbles straight from bytes; non-ASCII/continuation bytes return an error. |
| 2 | `fixed.rs::from_ratio` | Zero denominator → **divide-by-zero panic** (while `div()` was already hardened). | `den == 0` saturates toward the numerator's sign; out-of-range quotient clamps via `from_wide`. |
| 3 | `fixed.rs::from_int` | `value << 16` silently **flips sign** for `|value| ≥ 32768` (`from_int(32768)` → `i32::MIN`), breaking the saturation invariant at construction. | `value.saturating_mul(ONE)`. |
| 4 | `.github/workflows/ci.yml` (audit job) | `Cargo.lock` is `.gitignored` but `cargo audit` ran with no lockfile-generation step → job **fails every run**. | Add `cargo generate-lockfile` before `cargo audit`. |
| 5 | `rng.rs::below` | `bound == 0` guarded only by `debug_assert!`; release silently consumed a draw and returned 0, **desyncing replays** between profiles. | Return `0` without drawing, identically in debug/release; documented. |

(Also fixed in passing: `from_ratio` no longer wraps via a truncating `as i32`
on an out-of-range quotient; clippy `should_implement_trait` on the deliberate
named `mul`/`div` methods is now an explicit, documented `allow`.)

---

## Phase 2 — Determinism & replay hardening (proposed)

Grounds: deterministic-lockstep practice (Gaffer, *Deterministic Lockstep*) and
"Lock-step simulation is child's play" (arXiv:1705.09704) — a per-frame state
checksum is the canonical desync detector, and **iteration order / RNG / float**
are the classic non-determinism sources.

- Implement `DetHash` for the core types (`Entity`, `Fixed`, `Color`,
  `Position`, `Render`) instead of forcing every caller to hand-roll it; add
  `SparseSet::det_hash_canonical` that folds in ascending entity-index order so
  "fold in canonical order" is enforced by the API, not a comment.
- A minimal **replay harness**: record `(seed, per-frame inputs)`, replay, and
  assert the per-frame hash sequence — locating any divergence by step index.
- Pin `PINNED_FINAL_HASH` across a CI matrix (Linux/macOS/Windows) to prove
  cross-target bit-exactness, not just same-machine reproducibility.
- RNG (PCG author's *Bugs in SplitMix*): SplitMix64 is a *seeding* generator and
  near-by seeds correlate. Mix the seed in `SplitMix64::new`, de-bias `below`
  with Lemire rejection for an exactly-uniform range, and consider a
  xoshiro256\*\* / PCG main stream (SplitMix64 demoted to seeding) — matching
  Bevy's convention.

## Phase 3 — Fixed-point & ECS ergonomics (proposed)

Grounds: the `fixed` / `cordic` crates provide rounding modes, `sqrt`, and
CORDIC trig as table stakes; the EG study *Run-time Performance Comparison of
Sparse-set and Archetype ECS* and Flecs 4.1 (dual storage) frame the ECS
trade-off.

- `Fixed`: integer `sqrt` (Newton), CORDIC `sin`/`cos`/`atan2`, round-to-nearest
  `mul`/`div` variants, and `Mul`/`Div`/`Neg` operator impls to match `Add`/`Sub`.
- `SparseSet`: a `query2`/`zip` helper that drives multi-component iteration from
  the smaller set (today callers hand-write `iter_sorted` + `get`, a classic
  correctness footgun); avoid the per-call `Vec`+sort in `iter_sorted` on hot
  paths by keeping `dense` index-ordered or offering a sort-free variant.

## Phase 4 — Feature parity with terminal roguelike toolkits (proposed, larger)

Grounds: bracket-lib (RLTK) and doryen-rs ship these; all are implementable in
pure integer math and slot naturally onto the `.game` level/tile/spawn model.

- FOV (symmetric shadowcasting), pathfinding (A\* / Dijkstra maps), procedural
  map generation (BSP / cellular automata) — each deterministic and
  replay-safe, raising the demo value of the content pipeline.
- Diagnostics: display-width-aware caret alignment (tabs / full-width chars),
  `--max-errors`, and optional miette-style "defined here" related notes.
- Testing: extend the structure-aware generator to emit boundary/adversarial
  values (multi-byte colors, out-of-range `from_int`, `den = 0`, huge rows) and
  add an optional `cargo +nightly fuzz` target.

---

## References

- ECS storage trade-offs — EG, *Run-time Performance Comparison of Sparse-set
  and Archetype Component Storage*:
  <https://diglib.eg.org/bitstreams/766b72a4-70ae-4e8e-935b-949d589ed962/download>
- Flecs 4.1 (dual archetype + sparse-set):
  <https://ajmmertens.medium.com/flecs-4-1-is-out-fab4f32e36f6>
- Deterministic lockstep — Gaffer On Games:
  <https://gafferongames.com/post/deterministic_lockstep/>
- *Lock-step simulation is child's play* (arXiv:1705.09704):
  <https://arxiv.org/abs/1705.09704>
- SplitMix64 weaknesses — PCG, *Bugs in SplitMix*:
  <https://www.pcg-random.org/posts/bugs-in-splitmix.html>
- Fixed-point reference — the `fixed` crate: <https://docs.rs/fixed>
- Structure-aware / grammar fuzzing — *Growing a Test Corpus with Bonsai
  Fuzzing* (arXiv:2103.04388) <https://arxiv.org/pdf/2103.04388>, *Parser Knows
  Best / ParserFuzz* (arXiv:2503.03893) <https://arxiv.org/pdf/2503.03893>
- Terminal roguelike toolkits — bracket-lib <https://lib.rs/crates/brltk>,
  doryen-rs <https://github.com/jice-nospam/doryen-rs>
