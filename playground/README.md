# IZANAGI Playground

A terminal launcher for the workspace's examples. The examples in
[`izanagi/examples`](../izanagi/examples/pong.rs) and
[`izanagi_kit/examples`](../izanagi_kit/examples/wfc_demo.rs) are the
project's shop window — each one runs headless, prints a result, and
reproduces it byte for byte — but they are normally reached one at a time
through cargo, or all at once by the verification gate in
[`tools/gate.sh`](../tools/gate.sh). The playground puts them on a single
menu so they can be browsed and played directly.

The playground contains no demo logic of its own. It reads the demo list
from the filesystem at run time, builds the examples with cargo, and runs
the freshly built binaries, so a new example appears on the menu as soon
as it is added. Demos run attached to the terminal, which is what makes
the interactive ones playable: pong, for example, accepts a `--terminal`
argument and can be played from the menu.

## Running it

From this directory:

```sh
cargo run
```

That opens the menu. Type a demo number to run it, a number followed by
arguments to pass them through (for example `5 --terminal`), `a` to run
every demo, `l` to list again, or `q` to quit.

Non-interactive forms, for scripts and for checking the playground
itself:

```sh
cargo run -- --list
cargo run -- --run pong
cargo run -- --run pong -- --terminal
cargo run -- --all
```

`--all` runs every demo with its output captured and reports which ones
ran clean — the same judgement the gate's example stage makes (success,
and something printed), from the user's seat instead of the verifier's.

For comparison, a single demo can always be run through the workspace
directly, for example:

```sh
cargo run -p izanagi --example pong
```

## Why it lives outside the workspace

This crate is a separate workspace on purpose. The root workspace's
zero-dependency rule is verified against the root lockfile resolving
exactly the two library crates, and the gate's stages are defined over
those two crates. A third member would break both. The playground
therefore depends on nothing but the standard library and drives the
examples through the `cargo` command and the built binaries, leaving the
library crates — and every invariant stated in
[`AGENT_INSTRUCTIONS.md`](../AGENT_INSTRUCTIONS.md) — untouched. Its own
build artifacts land in `playground/target/`, covered by
`playground/.gitignore` (the root ignore rules anchor `/target` to the
workspace root).

## Layout

```text
playground/
  Cargo.toml    the separate-workspace manifest (no dependencies)
  README.md     this file
  src/main.rs   discovery, menu, and the three non-interactive modes
```
