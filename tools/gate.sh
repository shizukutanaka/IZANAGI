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

# The gate defines "green" — so it must not inherit build inputs from the
# ambient shell. RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS can pass `--cfg` or
# `--cap-lints allow` to rustc and RUSTDOCFLAGS the same to rustdoc; either
# would let a developer's environment weaken or fork checks this repository
# believes it is running. They are scrubbed unconditionally: a build that
# needs them is exactly the build the gate does not measure.
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTDOCFLAGS

# Those three are the spellings people remember — the *family* is larger,
# and every member is the same kind of injection. `CARGO_BUILD_RUSTFLAGS`
# (and `CARGO_TARGET_<triple>_RUSTFLAGS`, `CARGO_ENCODED_RUSTDOCFLAGS`)
# reach rustc/rustdoc exactly like the names above — verified here:
# `--cap-lints=allow` through CARGO_BUILD_RUSTFLAGS reduces this
# workspace's clippy warning count to zero. `CARGO_PROFILE_*` re-adds
# profile keys the manifest grammar bans, `RUSTC`/`RUSTDOC`/
# `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER` swap the binary being
# measured, `RUSTC_BOOTSTRAP` makes a stable toolchain accept
# `#![feature]` (verified), `RUSTUP_TOOLCHAIN` swaps the toolchain, and an
# ambient CARGO_HOME's config.toml injects the same flags all over again.
# Scrub the family wholesale: anything the gate needs it sets itself.
# `LD_*`/`DYLD_*` join the scrub for the same reason — `LD_PRELOAD` /
# `DYLD_INSERT_LIBRARIES` do not pass a flag, they splice a shared library
# into rustc itself, and every measurement downstream inherits it.
# `GIT_*` too: `GIT_DIR`/`GIT_WORK_TREE` point `git status` at a different
# repository, which blinds the before/after tree sentinel below — verified:
# with a decoy GIT_DIR, a file written into this tree left the sentinel's
# snapshot identical. Git needs no env vars to work in this tree.
for v in $(env | grep -oE '^(CARGO[A-Z_]*|RUST[A-Z_]*|RUSTUP[A-Z_]*|LD[A-Z_]*|DYLD[A-Z_]*|GIT[A-Z_]*)=' | tr -d '='); do
    unset "$v"
done
cd "$(dirname "$0")/.."

# The checks below run repo code — test and example binaries that can write.
# A run that modifies a tracked file, or drops a new untracked one the
# scanners then read, is the suite editing the evidence it is measuring.
# Snapshot now; the last stage compares. Before/after, not emptiness: a
# dirty starting tree is the developer's own business.
tree_status() {
    # Ambient git config can blind the snapshot without touching the tree:
    # `status.showUntrackedFiles = no` (a real user-level setting) hides
    # dropped files, and the untracked cache / an fsmonitor daemon can
    # serve a stale listing — verified: a HOME pointing at a fake
    # .gitconfig kept `?? file` out of `git status --porcelain`. The -c
    # flags force the honest read. `--ignored=matching` additionally lists
    # ignored FILES under tracked dirs — a `*.swp`/`.DS_Store` drop would
    # otherwise never appear in porcelain at all (verified:
    # `!! izanagi/drop.swp`), while folding ignored dirs to `!! dir/` so
    # build churn inside target/ stays invisible instead of always
    # diffing. The two `!!` entries this gate itself creates are filtered
    # (a clean clone gains target/ and Cargo.lock during the run); every
    # other ignored entry must match before/after. Files *inside* an
    # ignored directory still hide — writes under target/, .temp/ or
    # .claude/ interiors are the documented residual.
    #
    # Two failure shapes are handled explicitly, because both otherwise
    # degenerate silently. If `git status` itself fails, the snapshot is
    # empty — and an empty before equals an empty after, so the sentinel
    # would attest "unchanged" while measuring nothing; fail closed. And
    # `grep -v` returns 1 when every line is filtered — on a pristine tree
    # that turns the caller's `tree_before=$(tree_status)` into a set -e
    # abort, so the filter must not own the function's exit status.
    _ts_raw=$(
        git -c status.showUntrackedFiles=all -c core.untrackedCache=false -c core.fsmonitor=false status --porcelain --ignored=matching
    ) || {
        echo "gate: git status failed — the tree sentinel cannot attest anything" >&2
        return 1
    }
    printf '%s\n' "$_ts_raw" | grep -vE '^!! (target/|Cargo\.lock)$' || true
}
if command -v git >/dev/null 2>&1; then
    tree_before=$(tree_status)
else
    echo "gate: git not found — the tree-unchanged check at the end will be skipped"
    tree_before=""
fi

# An isolated CARGO_HOME means an ambient ~/.cargo/config.toml cannot add
# rustflags, change the target, or point cargo at a different registry
# while the gate measures the tree.
export CARGO_HOME="$PWD/target/gate-cargo-home"
mkdir -p "$CARGO_HOME"

KIT_BRIDGE_HASH=353498ec4fbcd160

stage() {
    printf '\n\033[1m== %s\033[0m\n' "$1"
}

stage "toolchain (stable channel, >= workspace MSRV)"
# Every check below is compiled and linted by whatever `cargo` resolves to:
# a per-directory `rustup override`, a rust-toolchain.toml (banned, but the
# ban itself runs on the toolchain it questions), or a dev's default can
# silently swap rustc/rustfmt semantics — `#![feature]` on nightly, fmt
# and clippy differences per release. The suite's own premise is MSRV
# 1.75 (the workspace maximum) on the stable channel; assert it.
rustc_v=$(rustc --version)
case "$rustc_v" in
    *nightly*|*beta*)
        echo "gate: '$rustc_v' — the suite must run on stable; pre-release channels admit #![feature] and differing lint sets"
        exit 1
        ;;
esac
rustc_minor=$(echo "$rustc_v" | sed -E 's/^rustc 1\.([0-9]+).*/\1/')
case "$rustc_minor" in
    ''|*[!0-9]*)
        echo "gate: unparseable rustc version '$rustc_v'"
        exit 1
        ;;
esac
if [ "$rustc_minor" -lt 75 ]; then
    echo "gate: rustc '$rustc_v' is below the workspace MSRV (1.75)"
    exit 1
fi
for tool_v in "$(cargo --version)" "$(rustfmt --version)"; do
    case "$tool_v" in
        *nightly*|*beta*)
            echo "gate: pre-release component '$tool_v' — stable toolchain required"
            exit 1
            ;;
    esac
done

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

stage "pinned determinism hashes (debug and release)"
# Both profiles, because they do not agree about arithmetic: `overflow-checks`
# defaults to on for dev and off for release, so an addition that silently
# wraps in a release build panics in a debug one. The hashes were only ever
# checked in debug, which left the crate's central claim — this hash is stable
# — unverified in the profile a game actually ships in.
#
# Requiring the same hash from both is also a free test for that overflow: if
# any arithmetic in the simulation path wrapped, the release run would produce
# a different trace, or the debug run would panic. It costs one extra compile
# of two test binaries and a tenth of a second to run.
(cd izanagi_kit && cargo test --test determinism --test roguelike_sim)
(cd izanagi_kit && cargo test --release --test determinism --test roguelike_sim)

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
# right. `verify_pipeline_demo` asserts its own claims with assert!/panic! (a
# failed claim is a non-zero exit, which the run check below already catches)
# and `kit_bridge`'s output is grepped for its pinned hash in this same loop;
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
        # Run the freshly built binary directly: `cargo run` would only repeat
        # the freshness check `cargo build --examples` already did, ~40ms and
        # a cargo process spawn per call. Examples take no args, read no files
        # and use no cargo env vars, so the exec is identical for this check.
        bin="target/debug/examples/$name"
        if ! first=$($cap "$bin" 2>&1); then
            printf '%s\n' "$first" | tail -20
            echo "gate: example $label did not complete successfully"
            exit 1
        fi
        if [ -z "$first" ]; then
            echo "gate: example $label printed nothing — an example must show a result"
            exit 1
        fi
        if ! second=$($cap "$bin" 2>&1); then
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
        # kit_bridge's own integration check: the headless run's final hash is
        # pinned, proving engine-hosted and headless simulation agree.
        if [ "$label" = "izanagi::kit_bridge" ]; then
            printf '%s\n' "$first" | tail -1
            if ! printf '%s' "$first" | grep -q "$KIT_BRIDGE_HASH"; then
                echo "gate: kit_bridge output does not contain the pinned hash $KIT_BRIDGE_HASH"
                exit 1
            fi
        fi
        example_count=$((example_count + 1))
    done
done
echo "examples: $example_count ran headless, printed a result, and reproduced it"

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
    # Examples ship too, and are compiled by none of the above. The engine's
    # kit_bridge example used a path dev-dependency; cargo strips path
    # dependencies from a published manifest, because a consumer cannot
    # resolve one, so the tarball carried an example importing a crate that
    # was not there. Three targets, three separate misses — hence checking
    # each rather than trusting that a green build covers them.
    if ! (cd "$pkg_dir" && cargo build --examples --quiet >/dev/null 2>&1); then
        echo "gate: the packaged crate $name cannot build its own examples"
        (cd "$pkg_dir" && cargo build --examples 2>&1 | tail -30)
        exit 1
    fi
    if ! (cd "$pkg_dir" && cargo build --bins --quiet >/dev/null 2>&1); then
        echo "gate: the packaged crate $name cannot build its own binaries"
        (cd "$pkg_dir" && cargo build --bins 2>&1 | tail -30)
        exit 1
    fi
    echo "packaged $name: doctests, tests, examples and bins all pass inside the tarball"
done

# Building is not the same as working — three defects in a row proved that a
# green build says nothing about the target actually functioning. `gamec` is
# the kit's content gate; a consumer gets it from `cargo install izanagi_kit`,
# so it is exercised on the fixtures the tarball itself ships, in both
# directions. A gate that accepts broken content is not a gate.
for kit_dir in target/package/izanagi_kit-*/; do
    [ -d "$kit_dir" ] || continue
    # The fixtures must be in the tarball before their verdicts mean anything.
    # Without this, a fixture that stopped shipping would still make the
    # "rejects the broken one" check pass — a missing file is also a non-zero
    # exit, and the check could not tell the two apart.
    for fixture in examples/dungeon.game examples/broken.game; do
        if [ ! -f "$kit_dir$fixture" ]; then
            echo "gate: $fixture is not in the packaged crate, so gamec cannot be"
            echo "      exercised against it — the checks below would pass vacuously"
            exit 1
        fi
    done
    if ! (cd "$kit_dir" && cargo run -q --bin gamec -- examples/dungeon.game >/dev/null 2>&1); then
        echo "gate: packaged gamec rejects examples/dungeon.game, which is valid"
        (cd "$kit_dir" && cargo run -q --bin gamec -- examples/dungeon.game 2>&1 | tail -20)
        exit 1
    fi
    if (cd "$kit_dir" && cargo run -q --bin gamec -- examples/broken.game >/dev/null 2>&1); then
        echo "gate: packaged gamec ACCEPTS examples/broken.game — a content gate"
        echo "      that passes broken content is worse than none"
        exit 1
    fi
    echo "packaged gamec: accepts the valid fixture, rejects the broken one"
done
rm -rf target/package

stage "working tree untouched by the run"
if command -v git >/dev/null 2>&1; then
    tree_after=$(tree_status)
    if [ "$tree_after" != "$tree_before" ]; then
        echo "gate: the run left the working tree different than it found it:"
        printf '%s\n' "$tree_before" > /tmp/gate_tree_a.$$
        printf '%s\n' "$tree_after" > /tmp/gate_tree_b.$$
        diff /tmp/gate_tree_a.$$ /tmp/gate_tree_b.$$ | head -20
        rm -f /tmp/gate_tree_a.$$ /tmp/gate_tree_b.$$
        exit 1
    fi
    echo "tree: unchanged since the gate started"
else
    echo "tree: skipped (git was not available)"
fi

printf '\n\033[1;32mgate: all stages green\033[0m\n'
