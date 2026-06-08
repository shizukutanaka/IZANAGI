//! Camera viewport and world-navigation demo.
//!
//! A 120×36 world tile map is viewed through a 60×18 `Camera` viewport.  A
//! simulated player walks a ten-step scripted path; the camera re-centres on
//! the player each step and clamps to the world boundary.
//!
//! Modules exercised:
//! - `TileMap<u8>` — world storage: border walls, corridor walls via `fill_rect` / `set`.
//! - `PassabilityGrid::from_tilemap` — blocker layer; `blocked_count` / `passable_count`.
//! - `Camera` — `new` (clamped centre), `recenter`, `world_to_screen`, `world_rect`.
//! - `Changed<(i32,i32)>` + `ChangeTracker` — dirty-flag tracking per sim tick.
//! - `MsgLog` — bounded ring log of human-readable movement events.
//! - `Profiler` + `EventLog<MoveEvent>` — work-unit timing and structured events.
//!
//! Run with `cargo run --example camera_viewport_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{
    Camera, Cell, ChangeTracker, Changed, EventLog, MsgLog, PassabilityGrid, Profiler, Screen,
    TileMap,
};
use std::io::{self, Write};

// ── world size ────────────────────────────────────────────────────────────────

const WORLD_W: u32 = 120;
const WORLD_H: u32 = 36;
const VP_W: u32 = 60; // viewport width in tiles
const VP_H: u32 = 18; // viewport height in tiles

// ── screen layout ─────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const VP_X: i32 = 0; // viewport top-left on screen
const VP_Y: i32 = 2;
const DIV_X: i32 = 60; // divider between viewport and log panel
const LOG_X: i32 = 61; // log panel start column
const LOG_COLS: usize = 19; // 80 - 61 = 19 usable log columns

// ── tile types ────────────────────────────────────────────────────────────────

const FLOOR: u8 = 0;
const WALL: u8 = 1;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 6, b: 16 };
const TITLE_BG: Color = Color {
    r: 30,
    g: 15,
    b: 60,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 200,
    b: 255,
};
const WALL_FG: Color = Color {
    r: 100,
    g: 90,
    b: 130,
};
const WALL_BG: Color = Color {
    r: 20,
    g: 16,
    b: 36,
};
const FLOOR_FG: Color = Color {
    r: 38,
    g: 34,
    b: 58,
};
const FLOOR_BG: Color = Color {
    r: 12,
    g: 10,
    b: 22,
};
const PLAYER_FG: Color = Color {
    r: 255,
    g: 220,
    b: 60,
};
const PLAYER_BG: Color = Color {
    r: 22,
    g: 18,
    b: 45,
};
const DIV_FG: Color = Color {
    r: 55,
    g: 42,
    b: 95,
};
const LOG_HDR: Color = Color {
    r: 120,
    g: 110,
    b: 170,
};
const LOG_FG: Color = Color {
    r: 148,
    g: 143,
    b: 192,
};
const STAT_FG: Color = Color {
    r: 140,
    g: 200,
    b: 255,
};
const STAT_VAL: Color = Color {
    r: 255,
    g: 218,
    b: 100,
};
const PASS_FG: Color = Color {
    r: 80,
    g: 180,
    b: 80,
};
const BLCK_FG: Color = Color {
    r: 180,
    g: 80,
    b: 80,
};

// ── structured event for EventLog ────────────────────────────────────────────

#[derive(Clone, Debug)]
enum MoveEvent {
    Stepped { from: (i32, i32), to: (i32, i32) },
    CameraClamp { top_left: (i32, i32) },
}

// ── world tile map ────────────────────────────────────────────────────────────

fn build_world() -> TileMap<u8> {
    let mut world: TileMap<u8> = TileMap::new(WORLD_W, WORLD_H, FLOOR);

    // Border walls (4 edges).
    world.fill_rect(0, 0, WORLD_W as i32, 1, WALL); // top
    world.fill_rect(0, WORLD_H as i32 - 1, WORLD_W as i32, 1, WALL); // bottom
    world.fill_rect(0, 0, 1, WORLD_H as i32, WALL); // left
    world.fill_rect(WORLD_W as i32 - 1, 0, 1, WORLD_H as i32, WALL); // right

    // Horizontal corridor walls at y=12 and y=24; gap every 20 cols at x%20==10.
    for x in 0..WORLD_W as i32 {
        if x % 20 != 10 {
            world.set(x, 12, WALL);
            world.set(x, 24, WALL);
        }
    }

    // Vertical corridor walls at x=40 and x=80; gap every 10 rows at y%10==5.
    for y in 0..WORLD_H as i32 {
        if y % 10 != 5 {
            world.set(40, y, WALL);
            world.set(80, y, WALL);
        }
    }

    world
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Profiler: keep 8 ticks of rolling history (work units, not real time).
    let mut prof = Profiler::new(8);
    prof.begin_tick();

    // ── world + passability ───────────────────────────────────────────────────

    let world = build_world();
    let world_cells = WORLD_W as u64 * WORLD_H as u64;
    prof.record("world_gen", world_cells);

    // PassabilityGrid: wall where TileMap tile == WALL.
    let pass = PassabilityGrid::from_tilemap(&world, |t| *t == WALL);
    prof.record("pass_build", pass.len() as u64);

    let blocked = pass.blocked_count();
    let passable = pass.passable_count();

    // ── scripted path ─────────────────────────────────────────────────────────
    // Stays on floor tiles; uses corridor gaps at x=80,y=15 and y=8 to cross walls.
    // Camera clamps at right boundary (top_left_x→60) and top boundary (top_left_y→0).
    const PATH: &[(i32, i32)] = &[
        (60, 18),  // start: centre of world
        (70, 18),  // right →
        (80, 15),  // right + up (gap in x=80 wall at y%10==5)
        (90, 15),  // right →
        (100, 15), // right →
        (110, 15), // near right boundary (camera.top_left_x → 60, clamped)
        (110, 8),  // up ↑ (between y=0 and y=12 corridor wall)
        (110, 2),  // near top (camera.top_left_y → 0, clamped)
        (106, 2),  // left ←
        (102, 2),  // left ← (camera still clamped at top-right corner)
    ];

    // Change tracking: wrap player position in Changed<> to record dirty ticks.
    let mut tracker = ChangeTracker::new();
    let mut player: Changed<(i32, i32)> = Changed::new(PATH[0]);

    let mut log = MsgLog::new(18);
    let mut events: EventLog<MoveEvent> = EventLog::new(24);

    // Initial camera centred on start position.
    let mut cam = Camera::new(PATH[0].0, PATH[0].1, VP_W, VP_H, WORLD_W, WORLD_H);

    log.push(format!("start ({},{})", PATH[0].0, PATH[0].1));
    events.push(
        tracker.current(),
        MoveEvent::Stepped {
            from: PATH[0],
            to: PATH[0],
        },
    );

    // Simulate remaining steps.
    for &next_pos in &PATH[1..] {
        let prev_pos = player.value;
        tracker.advance();
        let tick = tracker.current();

        player.value = next_pos;
        player.mark(tick); // dirty-flag: this component changed at `tick`

        let prev_tl = (cam.top_left_x, cam.top_left_y);
        cam.recenter(next_pos.0, next_pos.1, WORLD_W, WORLD_H);
        let new_tl = (cam.top_left_x, cam.top_left_y);

        log.push(format!(
            "t{} ({},{})→({},{})",
            tick, prev_pos.0, prev_pos.1, next_pos.0, next_pos.1
        ));
        events.push(
            tick,
            MoveEvent::Stepped {
                from: prev_pos,
                to: next_pos,
            },
        );

        // Detect camera boundary clamp: top_left changed but player moved further.
        let clamped = new_tl != prev_tl
            && (new_tl.0 == 0
                || new_tl.0 == (WORLD_W - VP_W) as i32
                || new_tl.1 == 0
                || new_tl.1 == (WORLD_H - VP_H) as i32);
        if clamped {
            events.push(tick, MoveEvent::CameraClamp { top_left: new_tl });
        }
    }

    let final_pos = player.value;
    prof.record("path_sim", PATH.len() as u64);

    // ── count structured events ───────────────────────────────────────────────

    let n_steps = events
        .iter()
        .filter(|e| matches!(e.event, MoveEvent::Stepped { .. }))
        .count();
    let n_clamps = events
        .iter()
        .filter(|e| matches!(e.event, MoveEvent::CameraClamp { .. }))
        .count();

    // ── render ────────────────────────────────────────────────────────────────

    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: LOG_FG,
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
    let title = format!(
        " CAMERA VIEWPORT  world={}×{}  vp={}×{}  player=({},{})",
        WORLD_W, WORLD_H, VP_W, VP_H, final_pos.0, final_pos.1
    );
    screen.draw_str(0, 0, &title, TITLE_FG, TITLE_BG);

    // Row 1: panel headers + divider.
    screen.draw_str(1, 1, "WORLD VIEW", DIV_FG, BG);
    screen.set(DIV_X, 1, '│', DIV_FG, BG);
    screen.draw_str(LOG_X, 1, "MOVE LOG", LOG_HDR, BG);

    // Rows 2–19: world viewport via Camera::world_to_screen.
    let (wx0, wy0, wx1, wy1) = cam.world_rect();
    for wy in wy0..wy1 {
        for wx in wx0..wx1 {
            let tile = world.get(wx, wy).copied().unwrap_or(WALL);
            let is_player = (wx, wy) == (final_pos.0, final_pos.1);
            let (glyph, fg, bg) = if is_player {
                ('@', PLAYER_FG, PLAYER_BG)
            } else if tile == WALL {
                ('#', WALL_FG, WALL_BG)
            } else {
                ('.', FLOOR_FG, FLOOR_BG)
            };
            if let Some((sx, sy)) = cam.world_to_screen(wx, wy) {
                screen.set(VP_X + sx as i32, VP_Y + sy as i32, glyph, fg, bg);
            }
        }
    }

    prof.record("render", (VP_W * VP_H) as u64);

    // Divider column (rows 1–20).
    for y in 1..(VP_Y + VP_H as i32 + 1) {
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Rows 2–19: message log (right panel), most-recent VP_H entries.
    for (i, msg) in log.recent(VP_H as usize).enumerate() {
        let y = VP_Y + i as i32;
        let line: String = msg.chars().take(LOG_COLS - 1).collect();
        screen.draw_str(LOG_X, y, &line, LOG_FG, BG);
    }

    // Row 20: bottom separator ─┴─.
    let sep_y = VP_Y + VP_H as i32;
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, sep_y, g, DIV_FG, BG);
    }

    // Row 21: camera state and dirty-flag info.
    let tl = (cam.top_left_x, cam.top_left_y);
    let cam_line = format!(
        " cam=({},{})..({},{})  player=({},{})  tick={}  changed@t={}",
        tl.0,
        tl.1,
        tl.0 + VP_W as i32,
        tl.1 + VP_H as i32,
        final_pos.0,
        final_pos.1,
        tracker.current(),
        player.changed_at,
    );
    screen.draw_str(0, sep_y + 1, &cam_line, STAT_FG, BG);

    // Row 22: passability stats + event summary.
    let total = pass.len();
    let pct_blk = blocked * 100 / total.max(1);
    screen.draw_str(1, sep_y + 2, "passable=", STAT_FG, BG);
    screen.draw_str(10, sep_y + 2, &passable.to_string(), PASS_FG, BG);
    let pct_pass_str = format!(" ({}%)", 100 - pct_blk);
    screen.draw_str(13, sep_y + 2, &pct_pass_str, PASS_FG, BG);
    screen.draw_str(20, sep_y + 2, "  blocked=", STAT_FG, BG);
    screen.draw_str(30, sep_y + 2, &blocked.to_string(), BLCK_FG, BG);
    let pct_blk_str = format!(" ({}%)", pct_blk);
    screen.draw_str(33, sep_y + 2, &pct_blk_str, BLCK_FG, BG);
    let evt_str = format!("  steps={}  clamps={}", n_steps, n_clamps);
    screen.draw_str(39, sep_y + 2, &evt_str, STAT_FG, BG);

    // Row 23: profiler summary.
    let prof_line = format!(
        " prof: world={}  pass={}  sim={}  render={}  tick={}",
        prof.this_tick("world_gen"),
        prof.this_tick("pass_build"),
        prof.this_tick("path_sim"),
        prof.this_tick("render"),
        prof.tick(),
    );
    screen.draw_str(0, sep_y + 3, &prof_line, STAT_VAL, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    // Print structured event log to stderr (reads Stepped.from/.to and CameraClamp.top_left).
    eprintln!(
        "\nCamera viewport demo.  World={}×{}  viewport={}×{}",
        WORLD_W, WORLD_H, VP_W, VP_H
    );
    for entry in events.iter() {
        match &entry.event {
            MoveEvent::Stepped { from, to } => {
                eprintln!(
                    "  t{}: step ({},{}) → ({},{})",
                    entry.tick, from.0, from.1, to.0, to.1
                );
            }
            MoveEvent::CameraClamp { top_left } => {
                eprintln!(
                    "  t{}: camera clamped top_left=({},{})",
                    entry.tick, top_left.0, top_left.1
                );
            }
        }
    }
    eprintln!(
        "Camera final ({},{})  player ({},{})  passable={}  blocked={}  steps={}  clamps={}",
        cam.top_left_x,
        cam.top_left_y,
        final_pos.0,
        final_pos.1,
        passable,
        blocked,
        n_steps,
        n_clamps,
    );
}
