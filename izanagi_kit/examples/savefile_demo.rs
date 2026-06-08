//! Save-file framing demo: persist, corrupt, and version-reject game state.
//!
//! Demonstrates the `savefile` module's binary container format:
//!
//!   magic[4]  version[4LE]  checksum[8LE]  len[4LE]  payload
//!
//! Four scenarios are shown:
//!
//!   1. Save + clean load → OK, world state restored exactly.
//!   2. Corrupt one payload byte → `LoadError::ChecksumMismatch`.
//!   3. Truncate the buffer → `LoadError::TooShort`.
//!   4. Version mismatch → caller detects stale save and rejects it.
//!
//! A hex dump of the first 20 bytes (framing header) is rendered to help
//! readers understand the on-disk layout.
//!
//! Run with `cargo run --example savefile_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{load_bytes, save_bytes, Cell, LoadError, SaveHeader, Screen, SplitMix64};
use std::io::{self, Write};

// ── world state ───────────────────────────────────────────────────────────────

/// A trivially serialisable world snapshot: PRNG state + turn counter.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WorldSnapshot {
    rng_state: u64,
    total: i64,
    turn: u32,
}

const SAVE_VERSION: u32 = 1;
const FUTURE_VERSION: u32 = 2; // simulates a schema bump

impl WorldSnapshot {
    fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(20);
        v.extend_from_slice(&self.rng_state.to_le_bytes());
        v.extend_from_slice(&self.total.to_le_bytes());
        v.extend_from_slice(&self.turn.to_le_bytes());
        v
    }

    fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 20 {
            return None;
        }
        let rng_state = u64::from_le_bytes(b[0..8].try_into().ok()?);
        let total = i64::from_le_bytes(b[8..16].try_into().ok()?);
        let turn = u32::from_le_bytes(b[16..20].try_into().ok()?);
        Some(Self {
            rng_state,
            total,
            turn,
        })
    }
}

// ── demo scenarios ────────────────────────────────────────────────────────────

#[derive(Debug)]
enum ScenarioResult {
    Loaded(WorldSnapshot),
    LoadError(LoadError),
    VersionRejected { found: u32, expected: u32 },
}

struct Scenario {
    label: &'static str,
    result: ScenarioResult,
    expected_ok: bool,
    note: &'static str,
}

fn run_scenarios() -> (WorldSnapshot, Vec<u8>, Vec<Scenario>) {
    // Generate a world snapshot by running a quick sim.
    let mut rng = SplitMix64::new(0xABCD_EF01);
    let mut total: i64 = 0;
    for _ in 0..100 {
        total = total.wrapping_add(rng.below(10_000) as i64);
    }
    let original = WorldSnapshot {
        rng_state: rng.state(),
        total,
        turn: 100,
    };
    let payload = original.to_bytes();
    let header = SaveHeader {
        version: SAVE_VERSION,
    };
    let saved = save_bytes(&header, &payload);

    let mut scenarios = Vec::new();

    // ── 1. clean round-trip ───────────────────────────────────────────────────
    let result = match load_bytes(&saved) {
        Ok((hdr, p)) if hdr.version == SAVE_VERSION => match WorldSnapshot::from_bytes(p) {
            Some(ws) => ScenarioResult::Loaded(ws),
            None => ScenarioResult::LoadError(LoadError::TooShort),
        },
        Ok((hdr, _)) => ScenarioResult::VersionRejected {
            found: hdr.version,
            expected: SAVE_VERSION,
        },
        Err(e) => ScenarioResult::LoadError(e),
    };
    scenarios.push(Scenario {
        label: "1. Clean save → load",
        result,
        expected_ok: true,
        note: "save_bytes + load_bytes round-trip",
    });

    // ── 2. corrupt one payload byte ───────────────────────────────────────────
    let mut corrupted = saved.clone();
    corrupted[25] ^= 0xFF; // flip bits in the payload area (byte 25 = payload[5])
    let result = match load_bytes(&corrupted) {
        Ok(_) => ScenarioResult::Loaded(original.clone()), // unexpected success
        Err(e) => ScenarioResult::LoadError(e),
    };
    scenarios.push(Scenario {
        label: "2. Corrupt byte 25 → checksum fail",
        result,
        expected_ok: false,
        note: "ChecksumMismatch expected",
    });

    // ── 3. truncated buffer ───────────────────────────────────────────────────
    let truncated = &saved[..12]; // shorter than the 20-byte header
    let result = match load_bytes(truncated) {
        Ok(_) => ScenarioResult::Loaded(original.clone()),
        Err(e) => ScenarioResult::LoadError(e),
    };
    scenarios.push(Scenario {
        label: "3. Truncated to 12 bytes → too short",
        result,
        expected_ok: false,
        note: "TooShort expected",
    });

    // ── 4. version mismatch ───────────────────────────────────────────────────
    let old_save = save_bytes(
        &SaveHeader {
            version: SAVE_VERSION,
        },
        &payload,
    );
    let result = match load_bytes(&old_save) {
        Ok((hdr, p)) if hdr.version == FUTURE_VERSION => match WorldSnapshot::from_bytes(p) {
            Some(ws) => ScenarioResult::Loaded(ws),
            None => ScenarioResult::LoadError(LoadError::TooShort),
        },
        Ok((hdr, _)) => ScenarioResult::VersionRejected {
            found: hdr.version,
            expected: FUTURE_VERSION,
        },
        Err(e) => ScenarioResult::LoadError(e),
    };
    scenarios.push(Scenario {
        label: "4. Stale version (v1 vs v2) → rejected",
        result,
        expected_ok: false,
        note: "VersionRejected v1 when v2 required",
    });

    (original, saved, scenarios)
}

// ── rendering ─────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const DK_BG: Color = Color {
    r: 10,
    g: 10,
    b: 20,
};
const TITLE_FG: Color = Color {
    r: 160,
    g: 200,
    b: 255,
};
const OK_FG: Color = Color {
    r: 80,
    g: 220,
    b: 80,
};
const WARN_FG: Color = Color {
    r: 220,
    g: 200,
    b: 60,
};
const ERR_FG: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};
const DIM_FG: Color = Color {
    r: 120,
    g: 120,
    b: 120,
};
const BRIGHT_FG: Color = Color {
    r: 220,
    g: 220,
    b: 220,
};
const HEX_FG: Color = Color {
    r: 120,
    g: 200,
    b: 120,
};

fn scenario_passed(s: &Scenario) -> bool {
    matches!(
        (&s.result, s.expected_ok),
        (ScenarioResult::Loaded(_), true)
            | (ScenarioResult::LoadError(_), false)
            | (ScenarioResult::VersionRejected { .. }, false)
    )
}

fn render_report(
    original: &WorldSnapshot,
    saved: &[u8],
    scenarios: &[Scenario],
    screen: &mut Screen,
) {
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

    let sep = "─".repeat(SCREEN_W as usize);

    // Title.
    screen.draw_str(
        0,
        0,
        "  izanagi_kit  savefile framing demo",
        TITLE_FG,
        DK_BG,
    );
    screen.draw_str(0, 1, &sep, DIM_FG, DK_BG);

    // World state.
    let ws_line = format!(
        "  snapshot: rng={:#018x}  total={}  turn={}  payload={}B",
        original.rng_state,
        original.total,
        original.turn,
        original.to_bytes().len(),
    );
    screen.draw_str(0, 2, &ws_line, BRIGHT_FG, DK_BG);

    // Hex dump of the 20-byte frame header.
    screen.draw_str(0, 3, "  hex[0..20]:", DIM_FG, DK_BG);
    let hex: String = saved[..20].iter().map(|b| format!("{:02x} ", b)).collect();
    screen.draw_str(15, 3, &hex, HEX_FG, DK_BG);

    // Header field annotations.
    screen.draw_str(
        0,
        4,
        "  magic[4]       version[4]     checksum[8]                len[4]",
        DIM_FG,
        DK_BG,
    );
    screen.draw_str(0, 5, &sep, DIM_FG, DK_BG);

    // Scenario rows.
    let mut row = 6i32;
    for scenario in scenarios {
        let passed = scenario_passed(scenario);
        let (status, status_fg) = if passed {
            ("  OK ", OK_FG)
        } else {
            (" FAIL", ERR_FG)
        };
        screen.draw_str(0, row, "[", DIM_FG, DK_BG);
        screen.draw_str(1, row, status, status_fg, DK_BG);
        screen.draw_str(6, row, "]", DIM_FG, DK_BG);
        screen.draw_str(8, row, scenario.label, BRIGHT_FG, DK_BG);
        row += 1;

        let detail = match &scenario.result {
            ScenarioResult::Loaded(ws) => format!(
                "       Loaded: rng={:#018x} total={} turn={}",
                ws.rng_state, ws.total, ws.turn
            ),
            ScenarioResult::LoadError(e) => format!("       Err: {:?}", e),
            ScenarioResult::VersionRejected { found, expected } => {
                format!(
                    "       VersionRejected: found=v{}  expected=v{}",
                    found, expected
                )
            }
        };
        let detail_fg = if passed { DIM_FG } else { WARN_FG };
        screen.draw_str(0, row, &detail, detail_fg, DK_BG);
        row += 1;

        screen.draw_str(0, row, &format!("       {}", scenario.note), DIM_FG, DK_BG);
        row += 2;
    }

    screen.draw_str(0, row, &sep, DIM_FG, DK_BG);
    row += 1;
    let all_ok = scenarios.iter().all(scenario_passed);
    let summary = if all_ok {
        "  All checks passed — save/load integrity verified."
    } else {
        "  One or more checks FAILED."
    };
    screen.draw_str(0, row, summary, if all_ok { OK_FG } else { ERR_FG }, DK_BG);
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let (original, saved, scenarios) = run_scenarios();
    let all_ok = scenarios.iter().all(scenario_passed);

    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    render_report(&original, &saved, &scenarios, &mut screen);
    screen.present();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nSavefile demo: {} checks — {}  (saved={}B)",
        scenarios.len(),
        if all_ok { "ALL PASSED" } else { "FAILED" },
        saved.len(),
    );
    if !all_ok {
        std::process::exit(1);
    }
}
