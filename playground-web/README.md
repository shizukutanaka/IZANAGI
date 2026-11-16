# IZANAGI Playground (web)

Try IZANAGI in a browser — no install, no server. The real `izanagi_kit`,
compiled to WebAssembly, running three of the kit's own samples entirely
on the page:

| Sample | What it shows | Source it mirrors |
| --- | --- | --- |
| Noise terrain | `fbm_2d` value noise → seven biomes, hash-scattered landmarks | `izanagi_kit/examples/noise_terrain_demo.rs` |
| Wave-function collapse | a 5-tile terrain gradient solved on an 80×44 grid, with autotile statistics | `izanagi_kit/examples/wfc_demo.rs` |
| Dungeon + A* + FOV | a generated dungeon walked by A*, FOV per turn, one world hash per turn | the sim half of `izanagi/examples/kit_bridge.rs` |

Pick a sample, change the seed, run it again: same seed → bit-identical
frame, on any machine. That is the kit's whole design claim, so the
dungeon sample prints its final world hash in the status line — for the
default seed it is `353498ec4fbcd160`, the very value `tools/gate.sh`
pins for the native `kit_bridge` example. You can check the claim
instead of taking it on faith.

## Run it

```sh
rustup target add wasm32-unknown-unknown   # once
./build.sh                                  # builds the .wasm into web/
python3 -m http.server -d web 8000          # any static server works
# open http://localhost:8000/
```

The page must be served over HTTP: browsers will not `fetch` a wasm
module from `file://`.

## Verify it

```sh
cargo test                    # native: rendering, determinism, pinned hash
node tests/wasm_smoke.mjs     # the built module, driven exactly as the page drives it
```

The Node test instantiates `web/izanagi_playground_web.wasm` with no
imports, runs every sample twice, and asserts the frame format, the
dimensions, byte-for-byte determinism, and the pinned dungeon hash. A
missing `.wasm` fails loudly with build instructions — it never skips.

## Why it is shaped this way

- **A separate crate, outside the root workspace** — the same pattern as
  `../playground` (the terminal launcher). The workspace's zero-dependency
  rule is verified against the root `Cargo.lock` resolving exactly the
  two library crates; a third member would break that invariant. This
  crate has its own workspace and lockfile, and the root manifests are
  untouched. Unlike `../playground`, which drives example binaries
  through `cargo` and needs no dependencies at all, a wasm module has to
  *link* the kit — so this crate depends on `izanagi_kit` by path, and
  that dependency never leaves this directory.
- **No wasm-bindgen, no JS port.** The kit is the product; a JavaScript
  reimplementation would drift from it on day one. The ABI is three
  exported functions (`sample_count`, `run_sample`, `frame_ptr` /
  `frame_len`) and one documented byte format, so the page stays a thin
  painter and every interesting behaviour stays in tested Rust.
- **The kit, not the engine.** `izanagi_kit` is the integer-only,
  platform-independent half — it already builds for `wasm32` in the
  kit's own width-independence checks. The `izanagi` engine (f32,
  backends, frame loop) is a larger port and deliberately out of scope
  for this trial.

## Frame format

`run_sample(id, seed_lo, seed_hi)` renders into a buffer the page reads
via `frame_ptr()` / `frame_len()` (valid until the next `run_sample`):

```text
u32le width, u32le height
u32le status byte length, status bytes (UTF-8)
per row:  u32le glyph byte length, glyph bytes (UTF-8),
          width×3 foreground RGB bytes, width×3 background RGB bytes
```

A width/height of 0 is a status-only frame, used for errors (e.g. an
unknown sample id) — failures are reported, never silent.

## Scope of the trial

Three representative samples, static frames, seed as the only input.
Not in scope: full example coverage, animation/interaction inside the
wasm, the `izanagi` engine, and hosting (any static host can serve
`web/`; wiring it to GitHub Pages or the docs site is a separate,
owner-side decision — this repository commits no CI workflows by design).
