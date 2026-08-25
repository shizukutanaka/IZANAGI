#!/bin/sh
# The verification gate: every check this repository considers "green", as one
# command. This is the same sequence AGENT_INSTRUCTIONS.md §4.2 prescribes and
# that every commit in the branch history reports — encoded once, so local
# runs, git hooks (.githooks/pre-push) and CI (docs/ci/ci.yml) all mean the
# same thing by "passing".
#
# Usage:  tools/gate.sh          (from the workspace root)
# Exits non-zero at the first failing stage. POSIX sh; no dependencies beyond
# cargo and grep.
set -eu

cd "$(dirname "$0")/.."

KIT_BRIDGE_HASH=353498ec4fbcd160
failures=0

stage() {
    printf '\n\033[1m== %s\033[0m\n' "$1"
}

stage "rustfmt (check only — the gate verifies, it does not rewrite)"
cargo fmt --all -- --check

stage "workspace tests"
cargo test --workspace

stage "clippy (zero warnings tolerated)"
# clippy exits 0 on warnings, so count them; -D warnings would also work but
# grep keeps the output visible in the log instead of aborting at the first.
clippy_out=$(cargo clippy --workspace --all-targets 2>&1) || {
    printf '%s\n' "$clippy_out"
    exit 1
}
warnings=$(printf '%s\n' "$clippy_out" | grep -cE '^(warning|error)' || true)
if [ "$warnings" -ne 0 ]; then
    printf '%s\n' "$clippy_out" | grep -E '^(warning|error)' -A 6
    echo "gate: clippy reported $warnings warning(s)/error(s)"
    exit 1
fi
echo "clippy: clean"

stage "rustdoc (both crates, zero warnings)"
touch izanagi_kit/src/lib.rs izanagi/src/lib.rs
doc_out=$(cargo doc -p izanagi_kit -p izanagi --no-deps 2>&1) || {
    printf '%s\n' "$doc_out"
    exit 1
}
doc_warnings=$(printf '%s\n' "$doc_out" | grep -cE '^(warning|error)' || true)
if [ "$doc_warnings" -ne 0 ]; then
    printf '%s\n' "$doc_out" | grep -E '^(warning|error)' -A 6
    echo "gate: rustdoc reported $doc_warnings warning(s)/error(s)"
    exit 1
fi
echo "rustdoc: clean"

stage "pinned determinism hashes"
(cd izanagi_kit && cargo test --test determinism --test roguelike_sim)

stage "kit_bridge integration hash"
bridge_out=$(cargo run -p izanagi --example kit_bridge 2>&1)
printf '%s\n' "$bridge_out" | tail -1
if ! printf '%s' "$bridge_out" | grep -q "$KIT_BRIDGE_HASH"; then
    echo "gate: kit_bridge output does not contain the pinned hash $KIT_BRIDGE_HASH"
    exit 1
fi

stage "verification pipeline demo (asserts its own claims)"
cargo run -p izanagi_kit --example verify_pipeline_demo >/dev/null
echo "verify_pipeline_demo: ok"

stage "packageability (both crates, verified)"
# No `--no-verify`: the verify step unpacks the tarball and *builds it*, which
# is the only way a "forgot to include that file" bug shows up. `cargo package`
# does this without contacting the registry, so it costs a compile and buys the
# same assurance `cargo publish --dry-run` gives.
cargo package -p izanagi_kit --allow-dirty
cargo package -p izanagi --allow-dirty
rm -rf target/package

printf '\n\033[1;32mgate: all stages green\033[0m\n'
