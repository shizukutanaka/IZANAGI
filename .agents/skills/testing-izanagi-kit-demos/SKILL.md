---
name: testing-izanagi-kit-demos
description: How to run, visually verify, and determinism-check the izanagi_kit terminal-rendered examples (ANSI Screen demos) on macOS.
---

# Testing izanagi_kit ANSI demos

All ~30 examples in `izanagi_kit/examples/` render an 80-col (sometimes taller)
truecolor ANSI `Screen` frame to **stdout** and print the human-readable summary
to **stderr** via `eprintln!`. They are headless — no input, no TTY required —
and are designed to be deterministic under their pinned seeds.

## Run one

```bash
cargo run --example <name> -p izanagi_kit        # from repo root or izanagi_kit/
```

`cargo run` from the repo root needs `-p izanagi_kit` because the workspace has
two crates with examples.

## Capture evidence

- **Text evidence (exec tool):** run with `2>/tmp/err.txt >/tmp/out.bin`; the
  summary line is in err.txt. Strip ANSI with
  `sed -e 's/\x1b\[[0-9;]*[A-Za-z]//g' /tmp/out.bin` to grep panel values.
- **Determinism:** `cargo run --example <name> -p izanagi_kit 2>/dev/null | shasum -a 256`
  twice — identical hashes prove byte-identical stdout (cargo's own
  "Finished/Running" lines go to stderr too, so they never pollute stdout).
  For stricter proof, `cmp` two full stdout captures.
- **Visual evidence (recording):** `open -a Terminal` launches macOS
  Terminal.app, which renders the truecolor SGR output faithfully. Enlarge the
  window first (`osascript -e 'tell application "Terminal" to set bounds of
  front window to {30,30,980,790}'`) so the full frame plus the stderr summary
  stay visible — a default 80x24 window scrolls the summary off-screen for
  taller demos (e.g. wfc_demo is 80x44).

## Gotchas

- Demos clip gracefully: `Screen::draw_str`/`set` silently drop cells outside
  the screen rect, so side panels may visibly truncate (e.g. scatter_pipeline_demo
  shows 7 of 9 MST edge labels and 20 of 40 packet hex bytes on its 80x24
  screen). Truncation consistent with the screen dimensions is expected, not a
  bug — verify counts from the stderr summary instead.
- Glyph/color legibility: region glyphs may be re-used modulo the demo's
  palette size when there are more logical regions than glyphs (e.g. 10 seeds /
  8 letters); stderr carries the true counts.
- Scoped unit suites: `cargo test -p izanagi_kit <filter>` matches module paths
  too (e.g. `voronoi` runs `voronoi::tests::*`, `worley` runs
  `noise::tests::test_worley_*`). Filters that match nothing still exit 0 with
  "0 passed" — check that a non-zero count ran.

## Secrets needed

None — no network, login, or env vars required.
