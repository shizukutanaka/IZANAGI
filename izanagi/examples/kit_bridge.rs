//! kit_bridge — the engine (frontend) rendering an izanagi_kit simulation
//! (backend), with proof that crossing the bridge preserves determinism.
//!
//! The two crates in this workspace split one product cleanly in half:
//!
//! - `izanagi_kit` owns *simulation truth*: integer-only, seed-driven,
//!   bit-identical across platforms (mapgen, FOV, pathfinding, world hash).
//! - `izanagi` owns *presentation*: the frame loop, input polling, and the
//!   draw list handed to whatever `Backend` is plugged in.
//!
//! This example wires them together the intended way. A deterministic
//! roguelike turn — player A*-walking from the first room to the last,
//! field-of-view recomputed each step — advances in the kit; each engine
//! frame then translates the kit's cell screen into engine draw calls.
//! Floats exist only on the render side; nothing flows back into the sim.
//!
//! The determinism claim is asserted, not narrated: the same sim runs once
//! headless (no engine at all) and once inside the engine frame loop, and
//! the two per-turn world-hash traces must match bit-for-bit. If rendering
//! ever perturbed simulation state, this example would panic in CI.
//!
//! Run with: `cargo run -p izanagi --example kit_bridge`

use izanagi::backend::NullBackend;
use izanagi::{Color, Engine};
use izanagi_kit::fov::compute_fov;
use izanagi_kit::mapgen::{generate_dungeon, Dungeon, GenParams};
use izanagi_kit::pathfinding::astar;
use izanagi_kit::terminal::{Cell, Screen};
use izanagi_kit::{content, hash_state, Fnv1a, SplitMix64};

const MAP_W: u32 = 48;
const MAP_H: u32 = 20;
const FOV_RADIUS: i32 = 8;
const SEED: u64 = 0x1DA_57E9;
/// Engine frames per sim turn: the sim is turn-based; the engine ticks at
/// 60 fps, so one step every 6 frames ≈ 10 turns/second of watchable pace.
const FRAMES_PER_TURN: u64 = 6;
/// Pixel size of one kit cell on the engine's render target.
const CELL_PX: f32 = 8.0;

/// The deterministic half: everything in here is integer math on kit types.
struct KitSim {
    dungeon: Dungeon,
    path: Vec<(i32, i32)>,
    step: usize,
    visible: Vec<bool>,
    screen: Screen,
    turn: u32,
}

impl KitSim {
    fn new(seed: u64) -> KitSim {
        // One master seed; independent named streams per subsystem so a
        // future extra draw in one system can never shift another.
        let master = SplitMix64::new(seed);
        let mut map_rng = master.split(1);

        let dungeon = generate_dungeon(MAP_W, MAP_H, &mut map_rng, GenParams::default());
        let start = dungeon.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
        let goal = dungeon.rooms.last().map(|r| r.center()).unwrap_or(start);
        let path = astar(start, goal, |x, y| dungeon.is_wall(x, y)).unwrap_or(vec![start]);

        let mut sim = KitSim {
            dungeon,
            path,
            step: 0,
            visible: vec![false; (MAP_W * MAP_H) as usize],
            screen: Screen::new(MAP_W, MAP_H + 1),
            turn: 0,
        };
        sim.refresh_fov();
        sim.render_to_screen();
        sim
    }

    fn player(&self) -> (i32, i32) {
        self.path[self.step.min(self.path.len() - 1)]
    }

    fn goal_reached(&self) -> bool {
        self.step + 1 >= self.path.len()
    }

    /// One turn: advance along the A* path, then recompute FOV and the
    /// cell screen. Pure integer state transitions — nothing here can
    /// differ between platforms or runs.
    fn tick(&mut self) {
        if !self.goal_reached() {
            self.step += 1;
        }
        self.turn += 1;
        self.refresh_fov();
        self.render_to_screen();
    }

    fn refresh_fov(&mut self) {
        self.visible.iter_mut().for_each(|v| *v = false);
        let origin = self.player();
        let (dungeon, visible) = (&self.dungeon, &mut self.visible);
        compute_fov(
            origin,
            FOV_RADIUS,
            |x, y| dungeon.is_wall(x, y),
            |x, y| {
                if x >= 0 && y >= 0 && (x as u32) < MAP_W && (y as u32) < MAP_H {
                    visible[(y as u32 * MAP_W + x as u32) as usize] = true;
                }
            },
        );
    }

    /// Draw the world into the kit's own cell buffer. The engine never
    /// reads sim structs directly — only this screen. That makes the
    /// screen the entire bridge surface.
    fn render_to_screen(&mut self) {
        let ink = |r, g, b| content::Color { r, g, b };
        self.screen.clear(Cell::blank());
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let lit = self.visible[(y as u32 * MAP_W + x as u32) as usize];
                let (glyph, fg) = if self.dungeon.is_wall(x, y) {
                    (
                        '#',
                        if lit {
                            ink(200, 180, 120)
                        } else {
                            ink(70, 70, 80)
                        },
                    )
                } else {
                    (
                        '.',
                        if lit {
                            ink(140, 140, 100)
                        } else {
                            ink(40, 40, 48)
                        },
                    )
                };
                self.screen.set(x, y, glyph, fg, ink(0, 0, 0));
            }
        }
        let goal = *self.path.last().unwrap();
        self.screen
            .set(goal.0, goal.1, '>', ink(80, 220, 120), ink(0, 0, 0));
        let (px, py) = self.player();
        self.screen
            .set(px, py, '@', ink(255, 255, 255), ink(0, 0, 0));
        let hud = format!("turn {:03}  pos ({px:02},{py:02})", self.turn);
        self.screen
            .draw_str(0, MAP_H as i32, &hud, ink(180, 180, 180), ink(0, 0, 0));
    }

    /// The per-turn world hash: screen cells + player + turn counter. This
    /// is what must match between the headless and engine-hosted runs.
    fn world_hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_u64(hash_state(&self.screen));
        let (px, py) = self.player();
        h.write_i32(px);
        h.write_i32(py);
        h.write_u32(self.turn);
        h.finish()
    }
}

/// Translate one kit cell screen into engine draw calls. All floats live
/// here, past the bridge, where they can no longer affect determinism.
fn present(screen: &Screen, render: &mut izanagi::Render) {
    for y in 0..screen.height() {
        for x in 0..screen.width() {
            if let Some(cell) = screen.get(x as i32, y as i32) {
                if cell.glyph == ' ' {
                    continue;
                }
                let c = Color::rgb8(cell.fg.r, cell.fg.g, cell.fg.b);
                render.rect(
                    x as f32 * CELL_PX,
                    y as f32 * CELL_PX,
                    CELL_PX - 1.0,
                    CELL_PX - 1.0,
                    c,
                );
            }
        }
    }
}

fn main() {
    // ── Pass 1: the sim alone, no engine anywhere near it. ──────────────
    let mut reference = KitSim::new(SEED);
    let mut reference_trace = vec![reference.world_hash()];
    while !reference.goal_reached() {
        reference.tick();
        reference_trace.push(reference.world_hash());
    }

    // ── Pass 2: the same sim hosted inside the engine frame loop. ───────
    let mut sim = KitSim::new(SEED);
    let mut engine_trace = vec![sim.world_hash()];
    let frames = (reference_trace.len() as u64 + 2) * FRAMES_PER_TURN;

    Engine::with_backend(Box::new(NullBackend::new().with_frames(frames)))
        .run(|e| {
            if e.frame() == 0 {
                e.render
                    .resize(MAP_W * CELL_PX as u32, (MAP_H + 1) * CELL_PX as u32);
            }
            if e.frame() % FRAMES_PER_TURN == 0 && e.frame() > 0 && !sim.goal_reached() {
                sim.tick();
                engine_trace.push(sim.world_hash());
            }
            present(&sim.screen, &mut e.render);
            let hud = format!("kit_bridge  turn {}  hash {:016x}", sim.turn, sim.world_hash());
            e.render
                .text(0.0, (MAP_H + 1) as f32 * CELL_PX, 12.0, Color::WHITE, hud);
            if sim.goal_reached() && e.frame() > sim.turn as u64 * FRAMES_PER_TURN + 2 {
                e.quit();
            }
        })
        .unwrap();

    // ── The bridge contract: rendering must not have touched the sim. ───
    assert_eq!(
        reference_trace, engine_trace,
        "world-hash trace diverged between headless and engine-hosted runs"
    );

    println!(
        "kit_bridge OK — {} turns, {} rooms, final hash {:016x} (headless == engine-hosted)",
        sim.turn,
        sim.dungeon.room_count(),
        sim.world_hash()
    );
}
