//! Input pipeline demo: KeyMap / InputBuffer / CmdQueue.
//!
//! Shows the three-layer deterministic input pipeline used in roguelikes and
//! lockstep simulations:
//!
//! 1. `InputBuffer<char>` — tracks held keys, fires on initial press and
//!    repeats after `initial_delay` ticks at `repeat_period` cadence.
//! 2. `KeyMap<char, Action>` — translates raw key chars to typed game actions.
//! 3. `CmdQueue<Action>` — accumulates translated actions; the simulation
//!    drains the whole batch at the tick boundary.
//!
//! A 13-tick scripted session is simulated:
//! - Initial presses render in yellow, hold-repeats in orange.
//! - Each tick drains the queue and logs the resulting command(s).
//!
//! Run with `cargo run --example input_pipeline_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{Cell, CmdQueue, InputBuffer, KeyMap, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const DIV_X: i32 = 20;
const RIGHT_X: i32 = 21;

// All keys the demo maps (used to scan held state before each tick).
const ALL_KEYS: &[char] = &['h', 'j', 'k', 'l', '.', 'o', '>', 'q'];

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 6, b: 18 };
const TITLE_BG: Color = Color {
    r: 22,
    g: 12,
    b: 55,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 200,
    b: 255,
};
const DIV_FG: Color = Color {
    r: 55,
    g: 40,
    b: 92,
};
const HDR_FG: Color = Color {
    r: 120,
    g: 110,
    b: 170,
};
const KEY_FG: Color = Color {
    r: 255,
    g: 220,
    b: 100,
};
const ACT_FG: Color = Color {
    r: 80,
    g: 200,
    b: 80,
};
const REP_FG: Color = Color {
    r: 255,
    g: 140,
    b: 60,
};
const STAT_FG: Color = Color {
    r: 140,
    g: 195,
    b: 255,
};
const TICK_FG: Color = Color {
    r: 100,
    g: 95,
    b: 140,
};
const EMPTY_FG: Color = Color {
    r: 55,
    g: 52,
    b: 80,
};

// ── game actions ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    MoveNorth,
    MoveSouth,
    MoveEast,
    MoveWest,
    Wait,
    OpenInventory,
    Descend,
    Quit,
}

impl Action {
    fn short(&self) -> &'static str {
        match self {
            Action::MoveNorth => "North",
            Action::MoveSouth => "South",
            Action::MoveEast => "East",
            Action::MoveWest => "West",
            Action::Wait => "Wait",
            Action::OpenInventory => "Inv",
            Action::Descend => "Desc",
            Action::Quit => "Quit",
        }
    }
}

// ── tick log ─────────────────────────────────────────────────────────────────

struct TickLog {
    tick: u32,
    /// Initial presses that fired this tick.
    initial: Vec<char>,
    /// Hold-repeats that fired this tick.
    repeats: Vec<char>,
    /// Commands drained from the queue after translation.
    cmds: Vec<Action>,
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // ── key map ───────────────────────────────────────────────────────────────
    let mut kmap: KeyMap<char, Action> = KeyMap::new();
    kmap.bind('k', Action::MoveNorth);
    kmap.bind('j', Action::MoveSouth);
    kmap.bind('l', Action::MoveEast);
    kmap.bind('h', Action::MoveWest);
    kmap.bind('.', Action::Wait);
    kmap.bind('o', Action::OpenInventory);
    kmap.bind('>', Action::Descend);
    kmap.bind('q', Action::Quit);

    // ── input buffer (delay=1 tick, repeat every 2 ticks) ────────────────────
    let mut ibuf: InputBuffer<char> = InputBuffer::new(1, 2);

    // ── command queue ─────────────────────────────────────────────────────────
    let mut queue: CmdQueue<Action> = CmdQueue::new();

    // ── scripted session: (presses, releases) applied before each tick ────────
    //
    //  t00  press h            → initial fire h → West
    //  t01  press l, release h → initial fire l → East
    //  t02  hold l             → no repeat yet (held_ticks=2, delay=1 but need count≥1)
    //  t03  hold l             → REPEAT l       → East
    //  t04  press k, release l → initial fire k → North
    //  t05  press '.', hold k  → initial fire . → Wait  (k not ready to repeat)
    //  t06  release '.', hold k→ REPEAT k       → North
    //  t07  press o, release k → initial fire o → Inv
    //  t08  press >, release o → initial fire > → Desc
    //  t09  press j, release > → initial fire j → South
    //  t10  press q, release j → initial fire q → Quit
    //  t11  release q          → no fires
    //  t12  (nothing)          → no fires
    let script: &[(&[char], &[char])] = &[
        (&['h'], &[]),        // t00
        (&['l'], &['h']),     // t01
        (&[], &[]),           // t02
        (&[], &[]),           // t03
        (&['k'], &['l']),     // t04
        (&['.'], &[]),        // t05
        (&[], &['.']),        // t06
        (&['o'], &['k']),     // t07
        (&['>'], &['o']),     // t08
        (&['j'], &['>']),     // t09
        (&['q'], &['j']),     // t10
        (&[], &['q']),        // t11
        (&[], &[]),           // t12
    ];

    // ── simulate ──────────────────────────────────────────────────────────────
    let mut logs: Vec<TickLog> = Vec::new();
    let mut total_cmds = 0u32;

    for (i, &(presses, releases)) in script.iter().enumerate() {
        // Snapshot held keys at the start of this tick (before events).
        let held_before: Vec<char> = ALL_KEYS
            .iter()
            .filter(|&&k| ibuf.is_held(&k))
            .cloned()
            .collect();

        for &r in releases {
            ibuf.release(&r);
        }
        for &p in presses {
            ibuf.press(p);
        }

        let fired = ibuf.tick(1);

        // Classify: a key that was already held and not newly pressed = repeat.
        let mut initial_fires: Vec<char> = Vec::new();
        let mut repeat_fires: Vec<char> = Vec::new();
        for &k in &fired {
            if held_before.contains(&k) && !presses.contains(&k) {
                repeat_fires.push(k);
            } else {
                initial_fires.push(k);
            }
        }

        let actions = kmap.translate_all(&fired);
        queue.push_batch(&actions);
        let cmds = queue.drain();
        total_cmds += cmds.len() as u32;

        logs.push(TickLog {
            tick: i as u32,
            initial: initial_fires,
            repeats: repeat_fires,
            cmds,
        });
    }

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: STAT_FG,
        bg: BG,
    });

    // Row 0: title bar.
    screen.fill_rect(
        0,
        0,
        SCREEN_W,
        1,
        Cell {
            glyph: ' ',
            fg: TITLE_FG,
            bg: TITLE_BG,
        },
    );
    screen.draw_str(
        0,
        0,
        " INPUT PIPELINE   KeyMap / InputBuffer / CmdQueue",
        TITLE_FG,
        TITLE_BG,
    );

    // Row 1: column headers.
    screen.draw_str(1, 1, "KEYMAP BINDINGS", HDR_FG, BG);
    screen.set(DIV_X, 1, '│', DIV_FG, BG);
    screen.draw_str(RIGHT_X, 1, "TICK LOG  yellow=initial  orange=repeat", HDR_FG, BG);

    // Row 2: separator.
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┬' } else { '─' };
        screen.set(x, 2, g, DIV_FG, BG);
    }

    // Left panel: key bindings (rows 3–10).
    let bindings: &[(&str, &str)] = &[
        ("k", "MoveNorth"),
        ("j", "MoveSouth"),
        ("l", "MoveEast"),
        ("h", "MoveWest"),
        (".", "Wait"),
        ("o", "OpenInventory"),
        (">", "Descend"),
        ("q", "Quit"),
    ];
    for (i, &(key, act)) in bindings.iter().enumerate() {
        let y = 3 + i as i32;
        screen.draw_str(2, y, key, KEY_FG, BG);
        screen.draw_str(4, y, "→", DIV_FG, BG);
        screen.draw_str(6, y, act, ACT_FG, BG);
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Left panel: InputBuffer parameters (rows 12–15).
    screen.draw_str(1, 12, "InputBuffer", HDR_FG, BG);
    screen.draw_str(2, 13, "delay  = 1", STAT_FG, BG);
    screen.draw_str(2, 14, "repeat = 2", STAT_FG, BG);
    screen.draw_str(1, 15, "CmdQueue", HDR_FG, BG);
    screen.draw_str(2, 16, "drain/tick", STAT_FG, BG);
    for y in 11..21 {
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Right panel: tick log (rows 3–15, one row per tick).
    for log in &logs {
        let y = 3 + log.tick as i32;
        if y > 15 {
            break;
        }

        // "t00 "
        let tn = format!("t{:02} ", log.tick);
        screen.draw_str(RIGHT_X, y, &tn, TICK_FG, BG);
        let mut cx = RIGHT_X + 4;

        // Fired key(s): initial in yellow, repeat in orange, empty = dim dash.
        let key_col_end = RIGHT_X + 10;
        if log.initial.is_empty() && log.repeats.is_empty() {
            screen.draw_str(cx, y, "──    ", EMPTY_FG, BG);
        } else {
            for &k in &log.initial {
                let s = format!("{} ", k);
                screen.draw_str(cx, y, &s, KEY_FG, BG);
                cx += s.len() as i32;
            }
            for &k in &log.repeats {
                let s = format!("{}* ", k);
                screen.draw_str(cx, y, &s, REP_FG, BG);
                cx += s.len() as i32;
            }
        }
        cx = key_col_end;

        // "→ "
        screen.draw_str(cx, y, "→ ", DIV_FG, BG);
        cx += 2;

        // Drained commands.
        if log.cmds.is_empty() {
            screen.draw_str(cx, y, "[]", EMPTY_FG, BG);
        } else {
            let cmd_str = log
                .cmds
                .iter()
                .map(|a| a.short())
                .collect::<Vec<_>>()
                .join(",");
            screen.draw_str(cx, y, &cmd_str, ACT_FG, BG);
        }
    }

    // Bottom separator (row 21).
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, 21, g, DIV_FG, BG);
    }

    // Stats rows 22–23.
    let moves: usize = logs
        .iter()
        .flat_map(|l| &l.cmds)
        .filter(|a| {
            matches!(
                a,
                Action::MoveNorth | Action::MoveSouth | Action::MoveEast | Action::MoveWest
            )
        })
        .count();
    let repeats_total: usize = logs.iter().map(|l| l.repeats.len()).sum();
    let stat_line = format!(
        " ticks={} total_cmds={} moves={} repeats={} bindings={}",
        logs.len(),
        total_cmds,
        moves,
        repeats_total,
        kmap.len(),
    );
    screen.draw_str(0, 22, &stat_line, STAT_FG, BG);
    screen.draw_str(
        0,
        23,
        " pipeline: InputBuffer──→KeyMap──→CmdQueue──→sim drain",
        DIV_FG,
        BG,
    );

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nInput pipeline demo.\n\
         ticks={} total_cmds={} moves={} repeats={} bindings={}\n\
         InputBuffer(delay=1, repeat=2): first repeat after 3 held ticks,\n\
         then every 2 ticks. CmdQueue drained once per tick boundary.",
        logs.len(),
        total_cmds,
        moves,
        repeats_total,
        kmap.len(),
    );
}
