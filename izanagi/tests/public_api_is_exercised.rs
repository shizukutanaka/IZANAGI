//! Every public function the engine ships is exercised by something — and this
//! proves it stays that way.
//!
//! `izanagi_kit` has enforced this since a sweep of its 1534 public functions
//! turned up 24 that nothing in the workspace ever called. The engine — the
//! crate this repository is named after, published as v4.1.0 — had no such
//! sweep. Running the kit's on it found, by coincidence, exactly 24 as well:
//! 9% of a 247-function surface that no test, example or benchmark touched.
//!
//! An untested public function is a promise the crate has never checked it can
//! keep, and once published it cannot be withdrawn without a breaking change.
//! The 23 that were real are exercised below (`log::_emit` was the 24th and is
//! `#[doc(hidden)]` — see `public_functions`).
//!
//! Oracles rather than hand-computed constants, matching the kit's file:
//! easing curves are pinned by their endpoints, `perp` by the definition of
//! perpendicularity, `saturate` by idempotence, `is_solid` by agreement with
//! the sibling it delegates to.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use izanagi::backend::{Backend, NullBackend, TerminalBackend};
use izanagi::debug::Metrics;
use izanagi::ease;
use izanagi::gamepad::{Button, Gamepads};
use izanagi::tilemap::Tilemap;
use izanagi::{Assets, Color, Draw, Input, Rng, Vec2};

// ------------------------------------------------------------------- ease

/// The easing curves that no test named. Every one is a reparameterisation of
/// progress: it must start at 0 and finish at 1. Curves that overshoot in the
/// middle (`elastic_out`, `bounce_out`) still have to land on those endpoints,
/// which is why the contract is stated as endpoints and not as monotonicity —
/// asserting monotonicity here would be asserting something false.
#[test]
fn the_unexercised_easing_curves_pin_their_endpoints() {
    type Curve = fn(f32) -> f32;
    let curves: [(&str, Curve); 6] = [
        ("quad_out", ease::quad_out),
        ("quad_in_out", ease::quad_in_out),
        ("cubic_in", ease::cubic_in),
        ("cubic_in_out", ease::cubic_in_out),
        ("bounce_out", ease::bounce_out),
        ("elastic_out", ease::elastic_out),
    ];
    for (name, f) in curves {
        assert!((f(0.0) - 0.0).abs() < 1e-5, "{name}(0) must be 0, got {}", f(0.0));
        assert!((f(1.0) - 1.0).abs() < 1e-5, "{name}(1) must be 1, got {}", f(1.0));
        // Every sample must be finite. A NaN here would propagate silently
        // through whatever the curve drives.
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            assert!(f(t).is_finite(), "{name}({t}) is not finite");
        }
    }
}

#[test]
fn the_in_out_curves_are_symmetric_about_their_midpoint() {
    // `quad_in_out` and `cubic_in_out` are defined as an ease-in half followed
    // by the mirrored ease-out half. That symmetry — f(t) + f(1-t) == 1 — is
    // the property that distinguishes them from their one-sided siblings, and
    // it holds for no other curve in the module.
    for i in 0..=10 {
        let t = i as f32 / 10.0;
        let q = ease::quad_in_out(t) + ease::quad_in_out(1.0 - t);
        assert!(
            (q - 1.0).abs() < 1e-4,
            "quad_in_out is not symmetric at t={t}: f(t)+f(1-t) = {q}"
        );
        let c = ease::cubic_in_out(t) + ease::cubic_in_out(1.0 - t);
        assert!(
            (c - 1.0).abs() < 1e-4,
            "cubic_in_out is not symmetric at t={t}: f(t)+f(1-t) = {c}"
        );
    }
    // The two must also differ from one another: a cubic pushes further from
    // the midpoint than a quadratic. Without this, one could be a copy of the
    // other and both tests above would still pass.
    assert!(ease::cubic_in_out(0.25) < ease::quad_in_out(0.25));
    assert!(ease::cubic_in_out(0.75) > ease::quad_in_out(0.75));
}

#[test]
fn bounce_stays_inside_the_unit_interval_and_elastic_overshoots_it() {
    // Their measured shapes, which is what makes them worth having as separate
    // curves: `bounce_out` dips and recovers without ever leaving [0, 1], while
    // `elastic_out` deliberately overshoots past 1 and rings back down. Either
    // one silently replaced by a plain ease-out would fail here.
    let bounce: Vec<f32> = (0..=20)
        .map(|i| ease::bounce_out(i as f32 / 20.0))
        .collect();
    assert!(
        bounce.iter().all(|v| (0.0..=1.0).contains(v)),
        "bounce_out must stay within [0, 1]: {bounce:?}"
    );
    assert!(
        bounce.windows(2).any(|w| w[1] < w[0] - 1e-3),
        "bounce_out must actually bounce — no sample decreases: {bounce:?}"
    );

    let elastic: Vec<f32> = (0..=20)
        .map(|i| ease::elastic_out(i as f32 / 20.0))
        .collect();
    assert!(
        elastic.iter().any(|v| *v > 1.05),
        "elastic_out must overshoot past 1: {elastic:?}"
    );
    // Damped: the second half of the ring is nearer to the target than the
    // first. This is the part a broken oscillator gets wrong.
    let early = elastic[1..8]
        .iter()
        .fold(0.0f32, |m, v| m.max((v - 1.0).abs()));
    let late = elastic[13..]
        .iter()
        .fold(0.0f32, |m, v| m.max((v - 1.0).abs()));
    assert!(
        late < early,
        "elastic_out must be damped: late deviation {late} is not below early {early}"
    );
}

#[test]
fn cubic_in_rises_faster_than_quad_in_which_rises_faster_than_linear() {
    // The curves are only useful if they differ in the documented direction: a
    // higher power stays nearer zero for longer. Checking the family ordering
    // catches a copy-paste between them, which pinning each one separately
    // against its own constants would not.
    for i in 1..10 {
        let t = i as f32 / 10.0;
        assert!(
            ease::cubic_in(t) < ease::quad_in(t),
            "cubic_in({t}) should be below quad_in({t})"
        );
        assert!(ease::quad_in(t) < ease::linear(t), "quad_in({t}) should be below linear({t})");
        assert!(ease::quad_out(t) > ease::linear(t), "quad_out({t}) should be above linear({t})");
    }
}

// ------------------------------------------------------------------- math

#[test]
fn perp_is_perpendicular_and_length_preserving() {
    // The definition, not a sampled value: the result is orthogonal to the
    // input and the same length. A sign flip or an axis swap fails one or the
    // other.
    for (x, y) in [(3.0f32, 4.0f32), (-1.5, 2.25), (0.0, 7.0), (5.0, 0.0)] {
        let v = Vec2::new(x, y);
        let p = v.perp();
        assert!(v.dot(p).abs() < 1e-4, "perp({x},{y}) is not perpendicular: dot = {}", v.dot(p));
        assert!((p.len() - v.len()).abs() < 1e-4, "perp({x},{y}) changed the length");
        // Rotating four times returns the original: perp is a quarter turn.
        let round = p.perp().perp().perp();
        assert!((round.x - v.x).abs() < 1e-4 && (round.y - v.y).abs() < 1e-4);
    }
}

#[test]
fn splat_broadcasts_one_scalar_to_both_axes() {
    for v in [0.0f32, 1.0, -3.5] {
        assert_eq!(Vec2::splat(v), Vec2::new(v, v));
    }
}

// ----------------------------------------------------------------- render

#[test]
fn rgba8_agrees_with_rgb8_on_the_opaque_case() {
    // A differential oracle against the sibling constructor that tests already
    // cover. If `rgba8` mis-scales a channel, the two disagree; a shared bug in
    // both would have to be introduced twice.
    for (r, g, b) in [(0u8, 0u8, 0u8), (255, 255, 255), (12, 200, 77)] {
        assert_eq!(Color::rgba8(r, g, b, 255), Color::rgb8(r, g, b));
    }
    let half = Color::rgba8(255, 0, 0, 128);
    assert!(
        (half.a - 128.0 / 255.0).abs() < 1e-6,
        "rgba8 must scale alpha by 1/255 like the colour channels"
    );
}

#[test]
fn saturate_clamps_and_is_idempotent() {
    let wild = Color {
        r: 2.0,
        g: -1.0,
        b: 0.5,
        a: 7.0,
    };
    let once = wild.saturate();
    assert_eq!(
        once,
        Color {
            r: 1.0,
            g: 0.0,
            b: 0.5,
            a: 1.0
        }
    );
    // Idempotence is the property that makes it safe to call anywhere in a
    // pipeline, and it is what a clamp written as a wrap or a scale would fail.
    assert_eq!(once.saturate(), once);
}

// ------------------------------------------------------------------ debug

#[test]
fn last_ms_and_best_ms_report_the_series_that_was_recorded() {
    let mut d = Metrics::new(8);
    // Empty is the boundary the implementation special-cases (min starts at
    // f32::MAX), so it is checked before anything is recorded.
    assert_eq!(d.last_ms(), 0.0, "an empty history has no last frame");
    assert_eq!(d.best_ms(), 0.0, "an empty history has no best frame");

    let series = [0.020f32, 0.008, 0.033, 0.016];
    for dt in series {
        d.record(dt);
    }
    let best = series.iter().copied().fold(f32::MAX, f32::min);
    let last = series[series.len() - 1];
    assert!((d.best_ms() - best * 1000.0).abs() < 1e-3, "best_ms is the minimum dt");
    assert!((d.last_ms() - last * 1000.0).abs() < 1e-3, "last_ms is the most recent dt");
    assert!(d.best_ms() <= d.worst_ms(), "best cannot exceed worst");
}

// ---------------------------------------------------------------- gamepad

#[test]
fn stick_state_round_trips_through_the_polled_api() {
    let mut pads = Gamepads::new();
    pads.on_connect(0, true);
    pads.on_left_stick(0, 0.5, -0.25);
    pads.on_right_stick(0, -1.0, 1.0);

    let l = pads.left_stick(0);
    assert!((l.x - 0.5).abs() < 1e-6 && (l.y + 0.25).abs() < 1e-6);
    let r = pads.right_stick(0);
    assert!((r.x + 1.0).abs() < 1e-6 && (r.y - 1.0).abs() < 1e-6);

    // Out-of-range input is clamped, not stored raw — otherwise a driver
    // reporting 1.2 would push movement past full speed.
    pads.on_left_stick(0, 9.0, -9.0);
    let clamped = pads.left_stick(0);
    assert_eq!((clamped.x, clamped.y), (1.0, -1.0));

    // An id past the end of the array is ignored rather than panicking, and
    // reads from it return the default.
    pads.on_left_stick(99, 1.0, 1.0);
    let absent = pads.left_stick(99);
    assert_eq!((absent.x, absent.y), (0.0, 0.0));
    assert!(!pads.connected(99));
    assert!(!pads.pressed(99, Button::South));
}

// ------------------------------------------------------------------ input

#[test]
fn mouse_position_reflects_the_last_move_event() {
    let mut input = Input::new();
    assert_eq!(input.mouse(), (0.0, 0.0), "a fresh Input starts at the origin");
    input.on_mouse_move(120.0, 45.5);
    assert_eq!(input.mouse(), (120.0, 45.5));
    input.on_mouse_move(-3.0, 0.0);
    assert_eq!(input.mouse(), (-3.0, 0.0), "a move replaces, it does not accumulate");
}

// ---------------------------------------------------------------- backend

#[test]
fn the_terminal_backend_builder_methods_are_chainable_and_take_effect() {
    // `canvas` and `no_quit` are builder methods; the observable effect of
    // `no_quit` is that the backend does not stop on its own, which is checked
    // through the Backend trait rather than through a private field.
    let mut b = TerminalBackend::new().canvas(80.0, 24.0).no_quit();
    assert_eq!(
        (b.canvas_w, b.canvas_h),
        (80.0, 24.0),
        "canvas() must set the canvas the terminal scales draw coordinates by"
    );
    // `no_quit` clears the deadline `quit_after` sets, so polling never asks to
    // stop. Checked through the trait, with a real Input, rather than by
    // reading a private field.
    let mut input = Input::new();
    assert!(b.poll(&mut input), "no_quit() means the backend never asks to stop");
    // Deliberately no init()/shutdown() here: on the terminal backend those
    // write escape sequences to the real stdout, which would clear the
    // developer's screen mid-test-run. NullBackend covers that pair below.
}

#[test]
fn the_null_backend_runs_exactly_max_frames() {
    // Also the only place the `Backend` trait's own methods are named as a set;
    // they carry no `pub` keyword, so the sweep below cannot see them.
    let mut b = NullBackend::new();
    b.max_frames = 3;
    b.fixed_dt = 1.0 / 30.0;
    b.init().expect("the null backend initialises");
    let mut input = Input::new();
    let no_draws: [Draw; 0] = [];
    let no_texts: [String; 0] = [];
    let mut frames = 0;
    while b.poll(&mut input) {
        b.present(Color::BLACK, &no_draws, &no_texts);
        assert!((b.dt() - 1.0 / 30.0).abs() < 1e-6);
        frames += 1;
        assert!(frames <= 4, "NullBackend must stop on its own");
    }
    b.shutdown();
    assert_eq!(frames, 3);
}

// ----------------------------------------------------------------- assets

#[test]
fn set_root_changes_where_load_looks() {
    // The oracle is a file that certainly exists: the crate's own manifest.
    // Pointing the root somewhere else must make the same name fail to load,
    // which is the whole observable behaviour of `set_root`.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut assets = Assets::new();
    assets.set_root(&manifest_dir);
    assert!(
        assets.load("Cargo.toml").is_some(),
        "Cargo.toml must load once the root points at the crate directory"
    );

    let mut elsewhere = Assets::new();
    elsewhere.set_root(manifest_dir.join("no-such-directory"));
    assert!(
        elsewhere.load("Cargo.toml").is_none(),
        "a root with no such file must not resolve the name"
    );
}

// ---------------------------------------------------------------- tilemap

#[test]
fn is_solid_agrees_with_the_world_space_query_that_delegates_to_it() {
    let mut map = Tilemap::new(4, 4, 16.0);
    map.set(1, 2, 7);
    assert!(map.is_solid(1, 2), "a non-zero tile is solid");
    assert!(!map.is_solid(0, 0), "tile id 0 is empty");
    // Out of bounds reads as empty rather than panicking.
    assert!(!map.is_solid(-1, 0));
    assert!(!map.is_solid(99, 99));

    // `is_solid_at` is documented as the world-space form of this query. The
    // two must agree on the centre of every tile, or one of them is wrong.
    for row in 0..4 {
        for col in 0..4 {
            let centre = Vec2::new(col as f32 * 16.0 + 8.0, row as f32 * 16.0 + 8.0);
            assert_eq!(
                map.is_solid(col, row),
                map.is_solid_at(centre),
                "is_solid and is_solid_at disagree at ({col}, {row})"
            );
        }
    }
}

// -------------------------------------------------------------------- rng

#[test]
fn from_entropy_is_the_deliberate_exception_to_replay_safety() {
    // `rng`'s integer core is documented as replay-safe, and `from_entropy` is
    // the one constructor that is not: it seeds from the wall clock, so it
    // cannot appear in a replayable path. Asserting that two instances differ
    // would be flaky at clock resolution, so what is pinned instead is the
    // contrast — `new(seed)` is reproducible, and `from_entropy` produces a
    // usable generator of the same type whose seed nobody chose.
    let a: Vec<u64> = (0..8).map(|_| Rng::new(42).u64()).collect();
    let b: Vec<u64> = (0..8).map(|_| Rng::new(42).u64()).collect();
    assert_eq!(a, b, "a seeded Rng must be reproducible");

    let mut entropic = Rng::from_entropy();
    let drawn: Vec<u64> = (0..8).map(|_| entropic.u64()).collect();
    assert!(
        drawn.windows(2).any(|w| w[0] != w[1]),
        "from_entropy must return a working generator, not a constant"
    );
}

// ------------------------------------------------------- the gate itself

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .to_path_buf()
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Every `pub fn` and every trait method the engine exposes, as
/// `(module, name)`.
///
/// Trait methods carry no `pub` keyword — inside a `pub trait` they are public
/// by definition — so a sweep that looks only for `pub fn` cannot see
/// `Backend::init` and its four siblings. They are collected here too.
///
/// `#[doc(hidden)]` items are skipped. That attribute is the author saying the
/// item is not part of the public promise, which is exactly the question this
/// sweep asks; `log::_emit` is the workspace's only one, and it is in fact
/// exercised through the `info!` macro that expands to it — invisibly to a
/// textual search, which is the other reason not to demand it here.
fn public_functions() -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for path in rust_files(&repo_root().join("izanagi/src")) {
        let module = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if module == "lib" {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap_or_default();
        let impl_end = src.find("#[cfg(test)]").unwrap_or(src.len());
        let mut hidden = false;
        let mut in_pub_trait = false;
        let mut trait_depth: i32 = 0;
        for line in src[..impl_end].lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if in_pub_trait {
                trait_depth += line.matches('{').count() as i32;
                trait_depth -= line.matches('}').count() as i32;
                if trait_depth <= 0 {
                    in_pub_trait = false;
                }
            } else if trimmed.starts_with("pub trait ") || trimmed.starts_with("pub unsafe trait ")
            {
                in_pub_trait = true;
                trait_depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
            }
            if trimmed.starts_with("#[doc(hidden)]") {
                hidden = true;
                continue;
            }
            let candidate = if in_pub_trait && trimmed.starts_with("fn ") {
                Some(&trimmed[3..])
            } else if let Some(rest) = trimmed.strip_prefix("pub fn ") {
                Some(rest)
            } else {
                trimmed.strip_prefix("pub const fn ")
            };
            if let Some(rest) = candidate {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                if !name.is_empty() && !hidden {
                    out.insert((module.clone(), name));
                }
                hidden = false;
            } else if !trimmed.is_empty()
                && !trimmed.starts_with("#[")
                && !trimmed.starts_with("///")
            {
                hidden = false;
            }
        }
    }
    out
}

/// Everything that could exercise the engine: its own inline test modules, its
/// tests, examples and benchmarks, and the kit-side files that drive it.
///
/// Including the inline `#[cfg(test)]` modules matters — leaving them out makes
/// the same sweep report 105 unexercised functions instead of 24, which is a
/// property of the corpus, not of the crate.
fn exercising_code() -> String {
    let root = repo_root();
    let mut blob = String::new();
    for path in rust_files(&root.join("izanagi/src")) {
        let src = fs::read_to_string(&path).unwrap_or_default();
        if let Some(i) = src.find("#[cfg(test)]") {
            blob.push_str(&src[i..]);
        }
    }
    for dir in [
        "izanagi/tests",
        "izanagi/examples",
        "izanagi/benches",
        "izanagi_kit/tests",
        "izanagi_kit/examples",
    ] {
        for path in rust_files(&root.join(dir)) {
            blob.push_str(&fs::read_to_string(&path).unwrap_or_default());
        }
    }
    blob
}

#[test]
fn no_public_function_goes_unexercised() {
    // The gate, mirroring izanagi_kit/tests/public_api_is_exercised.rs. A new
    // public function that nothing calls fails here — the moment to decide
    // whether it should exist, rather than after publication when removing it
    // is a breaking change.
    let code = exercising_code();
    let functions = public_functions();
    assert!(
        functions.len() > 200,
        "expected to find the engine's public surface, found {} — has the \
         source layout changed?",
        functions.len()
    );

    let mut unexercised: Vec<String> = Vec::new();
    for (module, name) in &functions {
        // A bare textual search, matching the kit's. It over-approximates (a
        // same-named method on another type counts), which is the safe
        // direction for a gate: it never demands a test that already exists.
        let called = code.contains(&format!("{name}("))
            || code.contains(&format!("{name}::"))
            || code.contains(&format!("{name}:"));
        if !called {
            unexercised.push(format!("{module}::{name}"));
        }
    }
    assert!(
        unexercised.is_empty(),
        "these public functions are never called by any test, example or \
         benchmark: {unexercised:#?}\n\nEither exercise them or delete them — \
         an untested public function is a promise nothing has checked."
    );
}

#[test]
fn the_sweep_sees_trait_methods_and_skips_doc_hidden() {
    // Both directions, because a sweep only ever checked against code it
    // catches will happily catch everything.
    let found = public_functions();
    for method in ["init", "poll", "present", "dt", "shutdown"] {
        assert!(
            found.contains(&("backend".to_string(), method.to_string())),
            "the sweep missed the `Backend::{method}` trait method — trait \
             methods carry no `pub` keyword and were invisible to it before"
        );
    }
    assert!(
        !found.contains(&("log".to_string(), "_emit".to_string())),
        "`log::_emit` is #[doc(hidden)]; the sweep must not demand a test for \
         an item the author marked as not part of the public promise"
    );
    // And it still sees ordinary public functions in the same file, so the
    // doc(hidden) skip is not swallowing the whole module.
    assert!(found.contains(&("log".to_string(), "set_level".to_string())));
    assert!(found.contains(&("log".to_string(), "level".to_string())));
}
