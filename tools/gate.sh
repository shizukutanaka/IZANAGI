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

stage "every example runs, prints, and reproduces"
# The gate used to run two of the workspace's 29 examples. The other 27 were
# compiled and never executed, so an example that panicked, hung or printed
# nothing would ship — and CLAUDE.md's rule that an example "must run headless"
# and "must print something useful" was a rule nothing enforced.
#
# The list is read from the filesystem rather than written here, so a new
# example is covered the day it is added. Each is run twice and the two outputs
# must match byte for byte: this crate's entire promise is reproducibility, and
# its own shop window is the last place that should go unchecked. All 29 pass
# today, which makes this a guard rather than a discovery.
#
# What this does *not* check is whether the numbers an example prints are
# right. Only `verify_pipeline_demo` and `kit_bridge` assert their own output;
# see AGENT_INSTRUCTIONS.md §2.
cargo build --workspace --examples --quiet
# `timeout` catches a hang instead of stalling CI for its whole job limit. It
# is coreutils, not POSIX, so a machine without it still runs the checks — it
# just waits instead of failing fast.
if command -v timeout >/dev/null 2>&1; then
    cap="timeout 120"
else
    cap=""
fi
example_count=0
for crate_dir in izanagi izanagi_kit; do
    for path in "$crate_dir"/examples/*.rs; do
        name=$(basename "$path" .rs)
        label="$crate_dir::$name"
        if ! first=$($cap cargo run -q -p "$crate_dir" --example "$name" 2>&1); then
            printf '%s\n' "$first" | tail -20
            echo "gate: example $label did not complete successfully"
            exit 1
        fi
        if [ -z "$first" ]; then
            echo "gate: example $label printed nothing — an example must show a result"
            exit 1
        fi
        if ! second=$($cap cargo run -q -p "$crate_dir" --example "$name" 2>&1); then
            echo "gate: example $label failed on its second run"
            exit 1
        fi
        if [ "$first" != "$second" ]; then
            echo "gate: example $label is not reproducible across runs:"
            printf '%s\n' "$first" > /tmp/gate_ex_a.$$
            printf '%s\n' "$second" > /tmp/gate_ex_b.$$
            diff /tmp/gate_ex_a.$$ /tmp/gate_ex_b.$$ | head -20
            rm -f /tmp/gate_ex_a.$$ /tmp/gate_ex_b.$$
            exit 1
        fi
        example_count=$((example_count + 1))
    done
done
echo "examples: $example_count ran headless, printed a result, and reproduced it"

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

stage "packageability (both crates, verified, doctests included)"
# No `--no-verify`: the verify step unpacks the tarball and *builds it*, which
# is the only way a "forgot to include that file" bug shows up. `cargo package`
# does this without contacting the registry, so it costs a compile and buys the
# same assurance `cargo publish --dry-run` gives.
cargo package -p izanagi_kit --allow-dirty
cargo package -p izanagi --allow-dirty

# ...but a build is not enough. `#[cfg(doctest)]` items do not exist during a
# build, so a `#[doc = include_str!(...)]` pointing outside the package builds
# and verifies happily, then fails for the first consumer who runs the
# doctests. That shipped once: izanagi included the workspace README with
# `../../README.md`, the gate was green, and the unpacked crate could not test
# itself. Running the doctests inside the unpacked tarball is what actually
# proves the published crate is self-contained.
for pkg_dir in target/package/izanagi_kit-*/ target/package/izanagi-*/; do
    [ -d "$pkg_dir" ] || continue
    case "$pkg_dir" in
        *.crate) continue ;;
    esac
    name=$(basename "$pkg_dir")
    if ! (cd "$pkg_dir" && cargo test --doc --quiet >/dev/null 2>&1); then
        echo "gate: the packaged crate $name cannot run its own doctests"
        (cd "$pkg_dir" && cargo test --doc 2>&1 | tail -30)
        exit 1
    fi
    # Both crates ship their tests on purpose — "the evidence is part of the
    # product". Evidence that fails on arrival is worse than none, and it did:
    # six repository-scoped test files read documents and manifests living
    # above the package, so a consumer running `cargo test` on the published
    # crate met a wall of failures about files they never received. Those files
    # are excluded now, and this is what keeps the rest honest.
    if ! (cd "$pkg_dir" && cargo test --tests --quiet >/dev/null 2>&1); then
        echo "gate: the packaged crate $name cannot run its own test suite"
        (cd "$pkg_dir" && cargo test --tests 2>&1 | tail -30)
        exit 1
    fi
    echo "packaged $name: doctests and tests both pass inside the tarball"
done
rm -rf target/package

printf '\n\033[1;32mgate: all stages green\033[0m\n'
