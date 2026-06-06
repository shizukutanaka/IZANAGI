# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `content::parse_color`: no longer panics on a 7-byte input containing a
  multi-byte UTF-8 char (e.g. `#aéABC`). It now parses hex straight from bytes
  instead of slicing the `&str` at fixed offsets, restoring the parser's
  panic-free guarantee. Regression covered in unit tests and the garbage-input
  fuzz.
- `fixed::from_int`: saturates instead of silently flipping sign for
  out-of-range inputs (`from_int(32768)` previously wrapped to `i32::MIN` via an
  unchecked `<< 16`), closing the last hole in the "never wrap to the opposite
  extreme" invariant.
- `fixed::from_ratio`: a zero denominator now saturates toward the numerator's
  sign (consistent with `div`) rather than panicking with a divide-by-zero, and
  an out-of-range quotient clamps via `from_wide` instead of a wrapping cast.
- `rng::below(0)`: now returns `0` without consuming a draw, identically in
  debug and release. Previously the bound check was `debug_assert!`-only, so
  release builds could silently advance the stream and desync a replay.
- `ci`: the security-audit job now runs `cargo generate-lockfile` before
  `cargo audit`; `Cargo.lock` is `.gitignored`, so the job previously failed
  with "Couldn't find Cargo.lock" on every run.

### Added
- `pathfinding`: deterministic 8-directional A* grid pathfinding. Integer octile
  costs (10 orthogonal / 14 diagonal) and heuristic (no float), open set ordered
  by a total `(f, h, x, y)` key so the result is reproducible, fixed-order
  neighbour expansion, and no diagonal corner-cutting through walls. Generic over
  an `is_blocked` closure. Covered by straight-line, diagonal, detour,
  corner-cut, unreachable, and determinism tests.
- `fov`: symmetric recursive shadowcasting field-of-view (Albert Ford's
  algorithm). Integer-only rational slopes (no float), fixed quadrant/column
  iteration order, zero allocation — bit-identical across targets. Generic over
  `is_opaque`/`mark_visible` closures so it works with any grid. Guarantees
  symmetric visibility (A sees B iff B sees A); covered by open-field,
  shadow-casting, symmetry-property, and determinism tests.
- `fixed`: transcendental math without floats — `sqrt` (integer bit-by-bit
  square root) and `sin`/`cos`/`sin_cos`/`atan2` (rotation- and vectoring-mode
  CORDIC, 16 iterations, constants as integer literals). All results are
  bit-identical across targets and safe to feed the world hash. Negative `sqrt`
  saturates to zero; `atan2` is four-quadrant and degenerate-input safe. Covered
  by cardinal-angle, Pythagorean-identity, inverse, and determinism tests.
- `parser`: column-aware diagnostics. Every parse error now carries a 1-based
  column, and `Diagnostic::render` prints the source line with a caret under the
  offending token (rustc/clang style).
- `serializer`: `Content` -> `.game` text, the inverse of the parser. Output is
  canonical and idempotent (`serialize(parse(serialize(c))) == serialize(c)`).
- `content_eq`: semantic equality for round-trip checking.
- `timestep`: fixed-timestep accumulator with a death-spiral guard
  (`max_steps`), using integer nanoseconds.
- `gamec --fmt`: emit canonical `.game` text to stdout.
- Structure-aware round-trip fuzz test (3000+ generated bundles) and a
  canonical-form fuzz test (2000+ bundles).

### Changed
- `fixed`: `Add`/`Sub` now **saturate** instead of wrapping, and `mul`/`div`
  clamp the wide intermediate into range. Division by zero saturates by sign
  instead of panicking. This fixes a latent bug where an overflowing coordinate
  could silently flip to the opposite extreme.

## [0.1.0]

### Added
- `entity` / `sparse_set`: sparse-set ECS storage with generational handles.
- `fixed`: Q16.16 fixed-point scalar.
- `rng`: SplitMix64 seeded PRNG.
- `world_hash`: FNV-1a deterministic state checksum, with an end-to-end
  bit-exact replay test.
- `content` / `parser` / `validator` / `loader`: the text-to-ECS content
  pipeline.
- `gamec`: content checker CLI usable as a CI gate.
