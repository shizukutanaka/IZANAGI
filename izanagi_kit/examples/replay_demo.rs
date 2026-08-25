//! Replay & desync-detection demo.
//!
//! Demonstrates the four replay primitives from the `replay` module:
//!
//!   1. `record_trace`    — run a sim and capture the hash at every tick.
//!   2. `check_trace`     — re-run and compare against a recorded trace.
//!   3. `first_divergence`— compare two peer traces and find the earliest split.
//!   4. `resimulate`      — rollback: clone a snapshot and replay inputs.
//!
//! The sim is a seeded counter: each tick advances a `SplitMix64` PRNG and
//! accumulates its output into a running value. Any mutation to the initial
//! seed — or a corrupted tick mid-run — is caught immediately.
//!
//! Rendered output: an 80×20 ANSI report card showing each test's result
//! (green OK / red FAIL), the divergence tick when applicable, and a
//! rollback verification row.
//!
//! Run with `cargo run --example replay_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::world_hash::{hash_state, DetHash, Fnv1a};
use izanagi_kit::{check_trace, Divergence};
use izanagi_kit::{first_divergence, record_trace, resimulate, Cell, Screen, SplitMix64};
use std::io::{self, Write};

// ── simulated state ───────────────────────────────────────────────────────────

/// Minimal deterministic simulation state: seeded PRNG + running accumulator.
#[derive(Clone)]
struct Counter {
    rng: SplitMix64,
    total: i64,
}

impl Counter {
    fn new(seed: u64) -> Self {
        Self {
            rng: SplitMix64::new(seed),
            total: 0,
        }
    }
}

impl DetHash for Counter {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_u64(self.rng.state());
        h.write_u64(self.total as u64);
    }
}

/// One simulation tick: draw a value from the PRNG and accumulate.
fn tick(state: &mut Counter, _input: &()) {
    let draw = state.rng.below(1_000_000) as i64;
    state.total = state.total.wrapping_add(draw);
}

// ── demo cases ────────────────────────────────────────────────────────────────

const TICKS: usize = 200;
const TAMPER_AT: usize = 73; // inject desync here

struct TestResult {
    label: &'static str,
    passed: bool,
    divergence: Option<Divergence>,
    note: &'static str,
}

fn run_tests() -> Vec<TestResult> {
    let inputs: Vec<()> = vec![(); TICKS];
    let seed_a: u64 = 0xC0DE_CAFE;
    let seed_b: u64 = 0xDEAD_BEEF;

    // ── 1. record the reference trace ─────────────────────────────────────────
    let mut s = Counter::new(seed_a);
    let ref_trace = record_trace(&mut s, &inputs, tick);

    // ── 2. clean replay: same seed → identical trace ──────────────────────────
    let mut s2 = Counter::new(seed_a);
    let clean_result = check_trace(&mut s2, &inputs, &ref_trace, tick);

    // ── 3. wrong seed: diverges at tick 0 ────────────────────────────────────
    let mut s3 = Counter::new(seed_b);
    let wrong_seed = check_trace(&mut s3, &inputs, &ref_trace, tick);

    // ── 4. tampered trace: corrupt one hash mid-run ───────────────────────────
    let mut tampered = ref_trace.clone();
    tampered[TAMPER_AT] ^= 0xFF; // flip some bits at tick TAMPER_AT
    let tamper_div = first_divergence(&tampered, &ref_trace);

    // ── 5. rollback/resimulate: snapshot at mid-run, replay tail ─────────────
    let snapshot_at = 50;
    let mut s_snap = Counter::new(seed_a);
    // Advance to snapshot point.
    for _ in 0..snapshot_at {
        tick(&mut s_snap, &());
    }
    let snapshot = s_snap.clone();
    let snap_hash_before = hash_state(&snapshot);

    // Resimulate from snapshot over the remaining inputs.
    let tail_inputs: Vec<()> = vec![(); TICKS - snapshot_at];
    let resim = resimulate(&snapshot, &tail_inputs, tick);
    let resim_hash = hash_state(&resim);

    // Run the same tail directly on s_snap for comparison.
    let mut s_direct = snapshot.clone();
    for _ in 0..(TICKS - snapshot_at) {
        tick(&mut s_direct, &());
    }
    let direct_hash = hash_state(&s_direct);

    let rollback_ok = resim_hash == direct_hash;
    let snapshot_unchanged = hash_state(&snapshot) == snap_hash_before;

    vec![
        TestResult {
            label: "1. Same seed → bit-identical trace",
            passed: clean_result.is_ok(),
            divergence: clean_result.err(),
            note: "record_trace / check_trace",
        },
        TestResult {
            label: "2. Different seed → diverges at tick 0",
            passed: wrong_seed.is_err() && wrong_seed.as_ref().err().map(|d| d.tick) == Some(0),
            divergence: wrong_seed.err(),
            note: "check_trace detects seed mismatch",
        },
        TestResult {
            label: "3. Tampered hash → diverges at correct tick",
            passed: tamper_div.is_err()
                && tamper_div.as_ref().err().map(|d| d.tick) == Some(TAMPER_AT),
            divergence: tamper_div.err(),
            note: "first_divergence pinpoints corruption",
        },
        TestResult {
            label: "4. Rollback: resimulate matches direct run",
            passed: rollback_ok && snapshot_unchanged,
            divergence: None,
            note: if rollback_ok && snapshot_unchanged {
                "resimulate ✓  snapshot immutable ✓"
            } else {
                "MISMATCH"
            },
        },
    ]
}

// ── rendering ─────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 20;

const DK_BG: Color = Color {
    r: 15,
    g: 15,
    b: 25,
};
const TITLE_FG: Color = Color {
    r: 160,
    g: 180,
    b: 255,
};
const OK_FG: Color = Color {
    r: 80,
    g: 220,
    b: 80,
};
const ERR_FG: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};
const DIM_FG: Color = Color {
    r: 130,
    g: 130,
    b: 130,
};
const BRIGHT_FG: Color = Color {
    r: 220,
    g: 220,
    b: 220,
};
const YELLOW: Color = Color {
    r: 220,
    g: 200,
    b: 60,
};

fn render_report(results: &[TestResult], screen: &mut Screen) {
    // Background.
    screen.fill_rect(
        0,
        0,
        SCREEN_W,
        SCREEN_H,
        Cell {
            glyph: ' ',
            fg: DIM_FG,
            bg: DK_BG,
        },
    );

    // Title.
    let title = "  izanagi_kit  replay & desync-detection demo";
    screen.draw_str(0, 0, title, TITLE_FG, DK_BG);
    let sep = "─".repeat(SCREEN_W as usize);
    screen.draw_str(0, 1, &sep, DIM_FG, DK_BG);

    // Config line.
    let cfg = format!(
        "  seed_a=0xC0DE_CAFE  seed_b=0xDEAD_BEEF  ticks={}  tamper_at={}",
        TICKS, TAMPER_AT
    );
    screen.draw_str(0, 2, &cfg, DIM_FG, DK_BG);
    screen.draw_str(0, 3, &sep, DIM_FG, DK_BG);

    // Test rows.
    let mut row = 4i32;
    for result in results {
        let (status, status_fg) = if result.passed {
            ("  OK ", OK_FG)
        } else {
            (" FAIL", ERR_FG)
        };
        screen.draw_str(0, row, "[", DIM_FG, DK_BG);
        screen.draw_str(1, row, status, status_fg, DK_BG);
        screen.draw_str(6, row, "]", DIM_FG, DK_BG);
        screen.draw_str(8, row, result.label, BRIGHT_FG, DK_BG);

        row += 1;
        // Divergence details.
        if let Some(div) = result.divergence {
            let detail = format!(
                "       divergence tick={:>3}  expected={:#018x}  actual={:#018x}",
                div.tick, div.expected, div.actual,
            );
            screen.draw_str(0, row, &detail, YELLOW, DK_BG);
            row += 1;
        }
        // Note.
        let note = format!("       {}", result.note);
        screen.draw_str(0, row, &note, DIM_FG, DK_BG);
        row += 2;
    }

    screen.draw_str(0, row, &sep, DIM_FG, DK_BG);
    row += 1;
    let all_ok = results.iter().all(|r| r.passed);
    let summary = if all_ok {
        "  All checks passed — determinism guarantee holds."
    } else {
        "  One or more checks FAILED."
    };
    let summary_fg = if all_ok { OK_FG } else { ERR_FG };
    screen.draw_str(0, row, summary, summary_fg, DK_BG);
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let results = run_tests();
    let all_ok = results.iter().all(|r| r.passed);

    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    render_report(&results, &mut screen);
    screen.present();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    // Summary to stderr for CI legibility.
    eprintln!(
        "\nReplay demo: {} checks — {}",
        results.len(),
        if all_ok { "ALL PASSED" } else { "FAILED" }
    );
    if !all_ok {
        std::process::exit(1);
    }
}
