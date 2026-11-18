//! Arena — a complete game, written by an outsider, on the public API only.
//!
//! A top-down arena survival shooter: the hero runs a square patrol route,
//! auto-fires at the nearest foe, and foes pour in from the arena edges in
//! waves. Contact costs health, dead foes drop repair cores, and the run
//! ends when the hero falls. Then the game restarts itself and plays a
//! second, passive run — no movement, no firing — so both endings are on
//! record.
//!
//! Everything is driven by a deterministic script: the "player" is a fixed
//! table of key presses (see `script_keys`), fed into [`Input`] through the
//! same `on_key_down` / `on_key_up` feed methods a backend calls. Combined
//! with `Engine::seed` and the fixed headless timestep, a run is a pure
//! function of the seed, so the numbers this program prints are asserted
//! against pinned values at the end — in the spirit of the five examples
//! that already assert what they print, this one asserts nearly everything.
//!
//! Run with `cargo run --example arena`. There is deliberately no
//! `--terminal` mode and no live input: the script is the player, which is
//! what makes the output reproducible enough to pin.
//!
//! ## Public API surface used
//!
//! - [`Engine`] — the frame loop, `seed`, `frame`, `dt`, `quit`.
//! - ECS ([`World`], [`Entity`]) — hero, foes, shots and cores are entities;
//!   components are plain structs; systems collect via `query` / `query2`
//!   and mutate via `get_mut` / `despawn`. A stale pre-restart handle is
//!   shown to be rejected by `alive` after [`World::clear`].
//! - [`States`] — Title → Playing → GameOver phase stack.
//! - [`Input`] — scripted key feed plus `down` / `pressed` queries.
//! - [`Rng`] — the integer half only (`int_range`), which `lib.rs` marks
//!   replay-safe; spawn points and drop rolls never touch the float half.
//! - `collide` — `swept_aabb` for shots (they move 10+ px a frame and would
//!   tunnel), `circle_vs_circle` for contact, `aabb_vs_aabb` for pickups.
//! - `sprite` — [`Animation`] / [`Frame`] / [`Sprite::from_grid`] walk
//!   cycles for hero and foes, drawn through [`Render::sprite`].
//! - `tilemap` — the arena: border walls and four pillar blocks are solid
//!   tiles; movement is axis-separated against `is_solid_at`.
//! - `audio` + `audio_pcm` — four clips synthesised with `sine_wave`,
//!   played on game events, and mixed by hand every frame (see gaps).
//! - `event` — [`Events`] carries kills / hits / wave clears to the score
//!   keeper, which folds them into the run's trace hash.
//! - `save` — a mid-run snapshot at wave 2 is encoded, written to a real
//!   file, read back, scrambled, restored, and proven bit-identical.
//!
//! ## Gaps an outsider hits (the point of the exercise)
//!
//! - [`Rng`] exposes no state accessor, so a save file cannot capture the
//!   RNG stream: after a load, future spawns diverge from the no-save
//!   timeline. A game that needs save-anywhere must own its generator
//!   state, the way `izanagi_kit`'s `SplitMix64` allows — the engine's
//!   `Rng` cannot be snapshotted.
//! - [`Save`] frames bytes with magic + version + length but carries **no
//!   checksum**: a corrupted payload parses cleanly. This example asserts
//!   the two failures the format *can* detect (bad magic, truncation) and
//!   detects payload damage itself only because it kept the original.
//! - [`Audio`] voices are only retired by `mix_into`, and the headless
//!   loop never calls it — a game that just calls `play` leaks voice
//!   slots forever. This example mixes one 60 fps buffer per frame, the
//!   way a real backend would, and asserts the mixed peak is non-silent.
//! - Feeding scripted input works, but only because the backend-facing
//!   feed methods on [`Input`] happen to be public; nothing in the docs
//!   presents them as the way to drive a replay or a demo.
//! - [`Animation`] advances on an `f32` dt, so an exact frame index is a
//!   float-boundary value: the probe below pins one measured mid-frame,
//!   away from any duration edge, and the doc says so rather than
//!   pretending frame indices are integers all the way down.

#![forbid(unsafe_code)]

use izanagi::audio_pcm::sine_wave;
use izanagi::collide;
use izanagi::event::Events;
use izanagi::save::Save;
use izanagi::sprite::{Animation, Frame, Sprite};
use izanagi::state::States;
use izanagi::tilemap::Tilemap;
use izanagi::{Color, Engine, Entity, Key, Rect, Vec2, World};

// ─── Arena geometry ─────────────────────────────────────────────────────────

const ARENA_W: f32 = 800.0;
const ARENA_H: f32 = 600.0;
const TILE: f32 = 40.0;
const COLS: u32 = 20;
const ROWS: u32 = 15;

// ─── Tuning ─────────────────────────────────────────────────────────────────

const HERO_RADIUS: f32 = 14.0;
const HERO_SPEED: f32 = 200.0;
const HERO_HP: i32 = 5;
const FOE_RADIUS: f32 = 13.0;
const FOE_HP: i32 = 2;
const SHOT_SPEED: f32 = 880.0;
const SHOT_DAMAGE: i32 = 1;
const FIRE_COOLDOWN: u64 = 12;
const CONTACT_INVULN: u64 = 16;
const SPAWN_EVERY: u64 = 10;
const INTERMISSION: u64 = 30;
const CORE_LIFETIME: u64 = 480;
const SCORE_KILL: u32 = 100;
const SCORE_CORE: u32 = 25;
const SAVE_VERSION: u16 = 1;
const SEED: u64 = 0xA2E4A;

// ─── Components ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct Pos(Vec2);

#[derive(Clone, Copy, Debug)]
struct Vel(Vec2);

#[derive(Clone, Copy, Debug)]
struct Hero {
    hp: i32,
    invuln_until: u64,
}

#[derive(Clone, Copy, Debug)]
struct Foe {
    hp: i32,
    speed: f32,
}

#[derive(Clone, Copy, Debug)]
struct Shot {
    damage: i32,
}

#[derive(Clone, Copy, Debug)]
struct Core {
    expires: u64,
}

#[derive(Clone, Debug)]
struct Anim(Animation);

// ─── Phases and events ──────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Title,
    Playing,
    GameOver,
}

#[derive(Clone, Debug)]
enum GameEvent {
    Kill { frame: u64, at: Vec2 },
    HeroHit { frame: u64, hp: i32 },
    WaveClear { frame: u64, wave: u32 },
    Pickup { frame: u64 },
    GameOver { frame: u64, score: u32 },
}

// ─── Trace hash ─────────────────────────────────────────────────────────────

/// FNV-1a over the run's event trace and final state. The hash is the
/// example's pinned fingerprint: any change to spawn order, collision
/// outcomes or scoring changes it, and the assert at the end says so.
fn fold(h: u64, v: u64) -> u64 {
    (h ^ v).wrapping_mul(0x0000_0100_0000_01b3)
}

/// Quantise a position to centi-pixels for the trace — integers only, so
/// the hash never depends on float formatting.
fn q(v: f32) -> u64 {
    (v * 100.0).round() as i64 as u64
}

// ─── Snapshot (save / load) ─────────────────────────────────────────────────

/// The mid-run state a save file carries. Floats travel as bit patterns so
/// the round-trip comparison is exact, and foes/cores are sorted by
/// position: entity indices are recycled on respawn, so index order is
/// *not* part of the saved state.
#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    score: u32,
    kills: u32,
    wave: u32,
    pending: u32,
    hero_hp: i32,
    hero_pos: (u32, u32),
    foes: Vec<(u32, u32, i32, u32)>,
    cores: Vec<(u32, u32, u64)>,
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn u32(&mut self) -> Option<u32> {
        let b = self.bytes.get(self.at..self.at + 4)?;
        self.at += 4;
        Some(u32::from_le_bytes(b.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        let b = self.bytes.get(self.at..self.at + 8)?;
        self.at += 8;
        Some(u64::from_le_bytes(b.try_into().ok()?))
    }

    fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
}

fn encode_snapshot(s: &Snapshot) -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, s.score);
    put_u32(&mut out, s.kills);
    put_u32(&mut out, s.wave);
    put_u32(&mut out, s.pending);
    put_u32(&mut out, s.hero_hp as u32);
    put_u32(&mut out, s.hero_pos.0);
    put_u32(&mut out, s.hero_pos.1);
    put_u32(&mut out, s.foes.len() as u32);
    for f in &s.foes {
        put_u32(&mut out, f.0);
        put_u32(&mut out, f.1);
        put_u32(&mut out, f.2 as u32);
        put_u32(&mut out, f.3);
    }
    put_u32(&mut out, s.cores.len() as u32);
    for c in &s.cores {
        put_u32(&mut out, c.0);
        put_u32(&mut out, c.1);
        put_u64(&mut out, c.2);
    }
    out
}

fn decode_snapshot(bytes: &[u8]) -> Option<Snapshot> {
    let mut r = Reader { bytes, at: 0 };
    let score = r.u32()?;
    let kills = r.u32()?;
    let wave = r.u32()?;
    let pending = r.u32()?;
    let hero_hp = r.i32()?;
    let hero_pos = (r.u32()?, r.u32()?);
    let n_foes = r.u32()? as usize;
    let mut foes = Vec::with_capacity(n_foes.min(4096));
    for _ in 0..n_foes {
        foes.push((r.u32()?, r.u32()?, r.i32()?, r.u32()?));
    }
    let n_cores = r.u32()? as usize;
    let mut cores = Vec::with_capacity(n_cores.min(4096));
    for _ in 0..n_cores {
        cores.push((r.u32()?, r.u32()?, r.u64()?));
    }
    if r.at != bytes.len() {
        return None;
    }
    Some(Snapshot {
        score,
        kills,
        wave,
        pending,
        hero_hp,
        hero_pos,
        foes,
        cores,
    })
}

// ─── Animations ─────────────────────────────────────────────────────────────

fn hero_anim() -> Animation {
    Animation::new(
        (0..3)
            .map(|i| Frame {
                sprite: Sprite::from_grid(i, 0, 16, 16),
                duration: 0.13,
            })
            .collect(),
        true,
    )
}

fn foe_anim() -> Animation {
    Animation::new(
        (0..2)
            .map(|i| Frame {
                sprite: Sprite::from_grid(i, 1, 16, 16),
                duration: 0.21,
            })
            .collect(),
        true,
    )
}

// ─── Arena map ──────────────────────────────────────────────────────────────

fn build_arena() -> Tilemap {
    let mut map = Tilemap::new(COLS, ROWS, TILE);
    map.fill(0, 0, COLS, 1, 1);
    map.fill(0, ROWS as i32 - 1, COLS, 1, 1);
    map.fill(0, 0, 1, ROWS, 1);
    map.fill(COLS as i32 - 1, 0, 1, ROWS, 1);
    // Four pillar blocks, kept clear of the centre patrol square.
    map.fill(3, 3, 2, 2, 1);
    map.fill(15, 3, 2, 2, 1);
    map.fill(3, 10, 2, 2, 1);
    map.fill(15, 10, 2, 2, 1);
    map
}

/// Move a circle-ish body by `delta`, axis-separated, against solid tiles.
/// Corner probes at 80% of the radius keep the body out of walls without
/// a physics engine — the same idiom `world.rs` uses for its hitbox.
fn move_body(map: &Tilemap, pos: Vec2, delta: Vec2, radius: f32) -> Vec2 {
    let hw = radius * 0.8;
    let blocked = |p: Vec2| {
        map.is_solid_at(Vec2::new(p.x - hw, p.y - hw))
            || map.is_solid_at(Vec2::new(p.x + hw, p.y - hw))
            || map.is_solid_at(Vec2::new(p.x - hw, p.y + hw))
            || map.is_solid_at(Vec2::new(p.x + hw, p.y + hw))
    };
    let mut out = pos;
    let step_x = Vec2::new(out.x + delta.x, out.y);
    if !blocked(step_x) {
        out = step_x;
    }
    let step_y = Vec2::new(out.x, out.y + delta.y);
    if !blocked(step_y) {
        out = step_y;
    }
    out.x = out.x.clamp(radius, ARENA_W - radius);
    out.y = out.y.clamp(radius, ARENA_H - radius);
    out
}

// ─── Run statistics ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct RunStats {
    score: u32,
    kills: u32,
    wave: u32,
    end_frame: u64,
}

// ─── The game ───────────────────────────────────────────────────────────────

struct Game {
    states: States<Phase>,
    prev_phase: Phase,
    phase_frame: u64,
    run: u32,
    map: Tilemap,
    hero: Option<Entity>,
    held: Vec<Key>,
    events: Events<GameEvent>,
    score: u32,
    kills: u32,
    wave: u32,
    pending: u32,
    spawn_timer: u64,
    wave_frame: u64,
    intermission: Option<u64>,
    next_shot: u64,
    save_done: bool,
    save_verified: bool,
    save_foes: usize,
    save_bytes: usize,
    trace: u64,
    sfx_plays: u32,
    shots_fired: u32,
    mix_peak: f32,
    mix_buf: Vec<f32>,
    anim_probe: Option<usize>,
    title_exit_frame: Option<u64>,
    stale_handle_rejected: bool,
    clip_count: usize,
    draws_last: usize,
    last_frame: u64,
    run1: Option<RunStats>,
    run2: Option<RunStats>,
}

impl Game {
    fn new() -> Self {
        Self {
            states: States::new(Phase::Title),
            prev_phase: Phase::Title,
            phase_frame: 0,
            run: 1,
            map: build_arena(),
            hero: None,
            held: Vec::new(),
            events: Events::new(),
            score: 0,
            kills: 0,
            wave: 0,
            pending: 0,
            spawn_timer: 0,
            wave_frame: 0,
            intermission: None,
            next_shot: 0,
            save_done: false,
            save_verified: false,
            save_foes: 0,
            save_bytes: 0,
            trace: 0xcbf2_9ce4_8422_2325,
            sfx_plays: 0,
            shots_fired: 0,
            mix_peak: 0.0,
            mix_buf: vec![0.0; 1470],
            anim_probe: None,
            title_exit_frame: None,
            stale_handle_rejected: false,
            clip_count: 0,
            draws_last: 0,
            last_frame: 0,
            run1: None,
            run2: None,
        }
    }

    // ── script ──────────────────────────────────────────────────────────────

    /// The deterministic player. Returns the keys held on this frame.
    fn script_keys(&self) -> Vec<Key> {
        match self.states.current() {
            Phase::Title => {
                if self.phase_frame == 20 {
                    vec![Key::Enter]
                } else {
                    Vec::new()
                }
            }
            Phase::Playing => {
                if self.run == 2 {
                    return Vec::new();
                }
                let leg = (self.phase_frame / 40) % 4;
                let dir = [Key::Right, Key::Down, Key::Left, Key::Up][leg as usize];
                vec![dir, Key::Space]
            }
            Phase::GameOver => {
                if self.run == 1 && self.phase_frame == 18 {
                    vec![Key::Enter]
                } else if self.run == 2 && self.phase_frame == 10 {
                    vec![Key::Escape]
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Feed the script's key set into the engine input, diffing against
    /// what is already held so `pressed` edges land on the right frame.
    fn feed_script(&mut self, e: &mut Engine) {
        let want = self.script_keys();
        for key in want.iter() {
            if !self.held.contains(key) {
                e.input.on_key_down(*key);
            }
        }
        for key in self.held.iter() {
            if !want.contains(key) {
                e.input.on_key_up(*key);
            }
        }
        self.held = want;
    }

    // ── setup ───────────────────────────────────────────────────────────────

    fn setup_audio(&mut self, e: &mut Engine) {
        e.audio.add_clip("shoot", sine_wave(660.0, 0.08, 44100));
        e.audio.add_clip("boom", sine_wave(196.0, 0.25, 44100));
        e.audio.add_clip("hurt", sine_wave(140.0, 0.30, 44100));
        e.audio.add_clip("pickup", sine_wave(990.0, 0.12, 44100));
        e.audio.set_master(0.9);
        self.clip_count = e.audio.clip_count();
    }

    fn play(&mut self, e: &mut Engine, name: &str, volume: f32) {
        e.audio.play(name, volume);
        self.sfx_plays += 1;
    }

    fn setup_run(&mut self, e: &mut Engine) {
        let hero = e.world.spawn();
        e.world
            .insert(hero, Pos(Vec2::new(ARENA_W / 2.0, ARENA_H / 2.0)));
        e.world.insert(hero, Vel(Vec2::ZERO));
        e.world.insert(
            hero,
            Hero {
                hp: HERO_HP,
                invuln_until: 0,
            },
        );
        e.world.insert(hero, Anim(hero_anim()));
        self.hero = Some(hero);
        self.score = 0;
        self.kills = 0;
        self.next_shot = 0;
        self.intermission = None;
        self.start_wave(e, 1);
    }

    fn start_wave(&mut self, e: &mut Engine, wave: u32) {
        self.wave = wave;
        self.wave_frame = 0;
        self.spawn_timer = 0;
        let total = 3 + 2 * wave;
        let burst = total.min(4);
        for _ in 0..burst {
            self.spawn_foe(e);
        }
        self.pending = total - burst;
        self.trace = fold(self.trace, 6);
        self.trace = fold(self.trace, wave as u64);
        println!("arena: wave {wave} ({} foes)", total);
    }

    fn spawn_foe(&mut self, e: &mut Engine) {
        let side = e.rng.int_range(0, 4);
        let pos = match side {
            0 => Vec2::new(e.rng.int_range(80, 720) as f32, 60.0),
            1 => Vec2::new(e.rng.int_range(80, 720) as f32, ARENA_H - 60.0),
            2 => Vec2::new(60.0, e.rng.int_range(80, 520) as f32),
            _ => Vec2::new(ARENA_W - 60.0, e.rng.int_range(80, 520) as f32),
        };
        let speed = 170.0 + 30.0 * self.wave as f32 + e.rng.int_range(0, 16) as f32;
        let foe = e.world.spawn();
        e.world.insert(foe, Pos(pos));
        e.world.insert(foe, Vel(Vec2::ZERO));
        e.world.insert(
            foe,
            Foe {
                hp: FOE_HP + self.wave as i32 - 1,
                speed,
            },
        );
        e.world.insert(foe, Anim(foe_anim()));
    }

    fn spawn_core(&mut self, e: &mut Engine, at: Vec2, frame: u64) {
        let core = e.world.spawn();
        e.world.insert(core, Pos(at));
        e.world.insert(
            core,
            Core {
                expires: frame + CORE_LIFETIME,
            },
        );
    }

    // ── snapshot ────────────────────────────────────────────────────────────

    fn take_snapshot(&self, world: &World) -> Snapshot {
        let hero = self.hero.expect("hero exists during play");
        let hero_pos = world.get::<Pos>(hero).expect("hero has Pos").0;
        let hero_hp = world.get::<Hero>(hero).expect("hero has Hero").hp;
        let mut foes: Vec<(u32, u32, i32, u32)> = world
            .query2::<Pos, Foe>()
            .iter()
            .map(|(_, p, f)| (p.0.x.to_bits(), p.0.y.to_bits(), f.hp, f.speed.to_bits()))
            .collect();
        foes.sort();
        let mut cores: Vec<(u32, u32, u64)> = world
            .query2::<Pos, Core>()
            .iter()
            .map(|(_, p, c)| (p.0.x.to_bits(), p.0.y.to_bits(), c.expires))
            .collect();
        cores.sort();
        Snapshot {
            score: self.score,
            kills: self.kills,
            wave: self.wave,
            pending: self.pending,
            hero_hp,
            hero_pos: (hero_pos.x.to_bits(), hero_pos.y.to_bits()),
            foes,
            cores,
        }
    }

    fn apply_snapshot(&mut self, e: &mut Engine, snap: &Snapshot) {
        // Transient entities are not saved: shots in flight are dropped,
        // foes and cores are respawned from the snapshot rows.
        let shots: Vec<Entity> = e.world.query::<Shot>().iter().map(|(en, _)| *en).collect();
        for en in shots {
            e.world.despawn(en);
        }
        let foes: Vec<Entity> = e.world.query::<Foe>().iter().map(|(en, _)| *en).collect();
        for en in foes {
            e.world.despawn(en);
        }
        let cores: Vec<Entity> = e.world.query::<Core>().iter().map(|(en, _)| *en).collect();
        for en in cores {
            e.world.despawn(en);
        }
        for f in &snap.foes {
            let foe = e.world.spawn();
            e.world
                .insert(foe, Pos(Vec2::new(f32::from_bits(f.0), f32::from_bits(f.1))));
            e.world.insert(foe, Vel(Vec2::ZERO));
            e.world.insert(
                foe,
                Foe {
                    hp: f.2,
                    speed: f32::from_bits(f.3),
                },
            );
            e.world.insert(foe, Anim(foe_anim()));
        }
        for c in &snap.cores {
            let core = e.world.spawn();
            e.world
                .insert(core, Pos(Vec2::new(f32::from_bits(c.0), f32::from_bits(c.1))));
            e.world.insert(core, Core { expires: c.2 });
        }
        let hero = self.hero.expect("hero exists during play");
        if let Some(p) = e.world.get_mut::<Pos>(hero) {
            p.0 = Vec2::new(f32::from_bits(snap.hero_pos.0), f32::from_bits(snap.hero_pos.1));
        }
        if let Some(h) = e.world.get_mut::<Hero>(hero) {
            h.hp = snap.hero_hp;
        }
        self.score = snap.score;
        self.kills = snap.kills;
        self.wave = snap.wave;
        self.pending = snap.pending;
    }

    /// The wave-2 checkpoint: save, write to a real file, read it back,
    /// corrupt-check the framing, scramble the live state, restore, and
    /// prove the restored state is bit-identical to the snapshot.
    fn save_load_checkpoint(&mut self, e: &mut Engine) {
        let snap = self.take_snapshot(&e.world);
        let payload = encode_snapshot(&snap);
        let encoded = Save::encode(SAVE_VERSION, &payload);

        // Through a real file, the way a game would actually persist it.
        let path = std::env::temp_dir().join("izanagi_arena_save.dat");
        Save::write(&path, SAVE_VERSION, &payload).expect("save write");
        let (disk_version, disk_payload) = Save::read(&path).expect("save read");
        assert_eq!(disk_version, SAVE_VERSION);
        assert_eq!(disk_payload, payload);
        let _ = std::fs::remove_file(&path);

        // The in-memory framing agrees, byte for byte.
        let (version, parsed) = Save::parse(&encoded).expect("save parse");
        assert_eq!(version, SAVE_VERSION);
        assert_eq!(parsed, payload);

        // The failures the format *can* detect: bad magic, truncation.
        // (A flipped payload byte parses fine — Save has no checksum;
        // see the header doc.)
        let mut bad_magic = encoded.clone();
        bad_magic[0] ^= 0xFF;
        assert!(Save::parse(&bad_magic).is_err());
        assert!(Save::parse(&encoded[..8]).is_err());

        let decoded = decode_snapshot(&parsed).expect("snapshot decode");
        assert_eq!(decoded, snap);

        // Scramble the live state, then restore and compare.
        let hero = self.hero.expect("hero exists during play");
        if let Some(p) = e.world.get_mut::<Pos>(hero) {
            p.0 = Vec2::new(p.0.x + 37.0, p.0.y - 21.0);
        }
        if let Some((first_foe, _)) = e.world.query::<Foe>().first() {
            let first_foe = *first_foe;
            e.world.despawn(first_foe);
        }
        self.score += 999;
        self.apply_snapshot(e, &decoded);
        let restored = self.take_snapshot(&e.world);
        assert_eq!(restored, snap);

        self.save_verified = true;
        self.save_foes = snap.foes.len();
        self.save_bytes = payload.len();
        self.trace = fold(self.trace, 7);
        self.trace = fold(self.trace, snap.score as u64);
        self.trace = fold(self.trace, snap.foes.len() as u64);
        println!(
            "arena: save/load round-trip ok ({} foes, {} payload bytes)",
            snap.foes.len(),
            payload.len()
        );
    }

    // ── per-frame ───────────────────────────────────────────────────────────

    fn tick(&mut self, e: &mut Engine) {
        let frame = e.frame();
        self.last_frame = frame;
        if frame == 0 {
            e.render.resize(ARENA_W as u32, ARENA_H as u32);
            e.render.set_clear(Color::rgb8(14, 16, 24));
            self.setup_audio(e);
        }

        // Phase bookkeeping: entering a phase runs its setup once.
        let current = self.states.current().clone();
        if current != self.prev_phase {
            self.phase_frame = 0;
            if current == Phase::Playing && self.hero.is_none() {
                self.setup_run(e);
            }
            self.prev_phase = current;
        } else {
            self.phase_frame += 1;
        }

        self.feed_script(e);

        match self.states.current().clone() {
            Phase::Title => {
                if e.input.pressed(Key::Enter) {
                    self.title_exit_frame = Some(frame);
                    self.trace = fold(self.trace, 9);
                    self.trace = fold(self.trace, frame);
                    println!("arena: title -> playing (frame {frame})");
                    self.states.replace(Phase::Playing);
                }
            }
            Phase::Playing => self.play_tick(e),
            Phase::GameOver => {
                if self.run == 1 && e.input.pressed(Key::Enter) {
                    self.restart(e);
                } else if self.run == 2 && e.input.pressed(Key::Escape) {
                    e.quit();
                }
            }
        }

        self.drain_events(e);
        self.draw(e);
        self.states.end_frame();

        // A real backend consumes audio every frame; headless, the game
        // must mix for itself or voices are never retired.
        e.audio.mix_into(&mut self.mix_buf, 735);
        for s in self.mix_buf.iter() {
            if s.abs() > self.mix_peak {
                self.mix_peak = s.abs();
            }
        }
        self.draws_last = e.render.len();
    }

    fn restart(&mut self, e: &mut Engine) {
        let old_hero = self.hero.take().expect("hero exists at game over");
        e.world.clear();
        // World::clear rolls generations forward, so the run-1 handle is
        // rejected rather than silently pointing at a run-2 entity.
        self.stale_handle_rejected = !e.world.alive(old_hero);
        self.run = 2;
        self.save_done = false;
        self.trace = fold(self.trace, 8);
        self.trace = fold(self.trace, e.frame());
        println!("arena: restart (run 2)");
        self.states.replace(Phase::Playing);
    }

    fn play_tick(&mut self, e: &mut Engine) {
        let frame = e.frame();
        let dt = e.dt();
        let hero = self.hero.expect("hero exists during play");

        // ── wave flow ──────────────────────────────────────────────────────
        if let Some(left) = self.intermission {
            if left <= 1 {
                self.intermission = None;
                let next = self.wave + 1;
                self.start_wave(e, next);
            } else {
                self.intermission = Some(left - 1);
            }
        } else {
            self.wave_frame += 1;
            if self.pending > 0 {
                self.spawn_timer += 1;
                if self.spawn_timer >= SPAWN_EVERY {
                    self.spawn_timer = 0;
                    self.pending -= 1;
                    self.spawn_foe(e);
                }
            } else if e.world.count::<Foe>() == 0 {
                self.score += 50 * self.wave;
                self.events.send(GameEvent::WaveClear {
                    frame,
                    wave: self.wave,
                });
                self.intermission = Some(INTERMISSION);
            }
            if self.wave == 2 && !self.save_done && self.wave_frame == 40 {
                self.save_done = true;
                self.save_load_checkpoint(e);
            }
        }

        // ── hero movement (scripted keys) ───────────────────────────────────
        let mut dir = Vec2::ZERO;
        if e.input.down(Key::Left) || e.input.down(Key::A) {
            dir.x -= 1.0;
        }
        if e.input.down(Key::Right) || e.input.down(Key::D) {
            dir.x += 1.0;
        }
        if e.input.down(Key::Up) || e.input.down(Key::W) {
            dir.y -= 1.0;
        }
        if e.input.down(Key::Down) || e.input.down(Key::S) {
            dir.y += 1.0;
        }
        let hero_pos = e.world.get::<Pos>(hero).expect("hero has Pos").0;
        if dir.len_sq() > 0.0 {
            let delta = dir.normalize() * HERO_SPEED * dt;
            let moved = move_body(&self.map, hero_pos, delta, HERO_RADIUS);
            if let Some(p) = e.world.get_mut::<Pos>(hero) {
                p.0 = moved;
            }
            if let Some(v) = e.world.get_mut::<Vel>(hero) {
                v.0 = dir.normalize() * HERO_SPEED;
            }
        } else if let Some(v) = e.world.get_mut::<Vel>(hero) {
            v.0 = Vec2::ZERO;
        }
        let hero_pos = e.world.get::<Pos>(hero).expect("hero has Pos").0;

        // ── firing: nearest foe, swept shots ────────────────────────────────
        if e.input.down(Key::Space) && frame >= self.next_shot {
            let target = e
                .world
                .query2::<Pos, Foe>()
                .iter()
                .map(|(_, p, _)| p.0)
                .min_by(|a, b| {
                    hero_pos
                        .distance_sq(*a)
                        .partial_cmp(&hero_pos.distance_sq(*b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            if let Some(target) = target {
                let shot = e.world.spawn();
                let vel = (target - hero_pos).normalize() * SHOT_SPEED;
                e.world.insert(shot, Pos(hero_pos));
                e.world.insert(shot, Vel(vel));
                e.world.insert(
                    shot,
                    Shot {
                        damage: SHOT_DAMAGE,
                    },
                );
                self.shots_fired += 1;
                self.next_shot = frame + FIRE_COOLDOWN;
                self.play(e, "shoot", 0.35);
            }
        }
        self.update_shots(e, dt);

        // ── foes chase the hero ─────────────────────────────────────────────
        let foe_entities: Vec<Entity> = e.world.query::<Foe>().iter().map(|(en, _)| *en).collect();
        for foe in foe_entities {
            let (pos, speed) = match (e.world.get::<Pos>(foe), e.world.get::<Foe>(foe)) {
                (Some(p), Some(f)) => (p.0, f.speed),
                _ => continue,
            };
            let delta = (hero_pos - pos).normalize() * speed * dt;
            let moved = move_body(&self.map, pos, delta, FOE_RADIUS);
            if let Some(p) = e.world.get_mut::<Pos>(foe) {
                p.0 = moved;
            }
        }

        // ── contact ─────────────────────────────────────────────────────────
        let invuln_until = e
            .world
            .get::<Hero>(hero)
            .expect("hero has Hero")
            .invuln_until;
        if frame >= invuln_until {
            let foe_entities: Vec<Entity> =
                e.world.query::<Foe>().iter().map(|(en, _)| *en).collect();
            for foe in foe_entities {
                let fpos = match e.world.get::<Pos>(foe) {
                    Some(p) => p.0,
                    None => continue,
                };
                if collide::circle_vs_circle(hero_pos, HERO_RADIUS, fpos, FOE_RADIUS) {
                    e.world.despawn(foe);
                    let hp = {
                        let h = e.world.get_mut::<Hero>(hero).expect("hero has Hero");
                        h.hp -= 1;
                        h.invuln_until = frame + CONTACT_INVULN;
                        h.hp
                    };
                    self.events.send(GameEvent::HeroHit { frame, hp });
                    self.play(e, "hurt", 0.6);
                    if hp <= 0 {
                        self.game_over(e);
                    }
                    break;
                }
            }
        }

        // ── cores: expiry and pickup ────────────────────────────────────────
        let core_entities: Vec<Entity> =
            e.world.query::<Core>().iter().map(|(en, _)| *en).collect();
        let hero_rect = Rect::new(
            hero_pos.x - HERO_RADIUS,
            hero_pos.y - HERO_RADIUS,
            HERO_RADIUS * 2.0,
            HERO_RADIUS * 2.0,
        );
        for core in core_entities {
            let (cpos, expires) = match (e.world.get::<Pos>(core), e.world.get::<Core>(core)) {
                (Some(p), Some(c)) => (p.0, c.expires),
                _ => continue,
            };
            if frame > expires {
                e.world.despawn(core);
                continue;
            }
            let core_rect = Rect::new(cpos.x - 8.0, cpos.y - 8.0, 16.0, 16.0);
            if collide::aabb_vs_aabb(&hero_rect, &core_rect) {
                e.world.despawn(core);
                let h = e.world.get_mut::<Hero>(hero).expect("hero has Hero");
                h.hp = (h.hp + 1).min(HERO_HP);
                self.score += SCORE_CORE;
                self.events.send(GameEvent::Pickup { frame });
                self.play(e, "pickup", 0.5);
            }
        }

        // ── animation probe: hero walk-cycle index, mid-frame ───────────────
        if self.run == 1 && self.anim_probe.is_none() && self.phase_frame == 20 {
            // Frame 0.13s at a fixed 1/60 dt: advances land near frames 8
            // and 16, so frame 20 sits mid-frame at index 2 — not on a
            // float boundary (see the header doc on Animation).
            let idx = e
                .world
                .get::<Anim>(hero)
                .expect("hero has Anim")
                .0
                .frame_index();
            self.anim_probe = Some(idx);
        }
    }

    fn update_shots(&mut self, e: &mut Engine, dt: f32) {
        let shots: Vec<(Entity, Vec2, Vec2, i32)> = e
            .world
            .query2::<Pos, Vel>()
            .iter()
            .filter_map(|(en, p, v)| e.world.get::<Shot>(*en).map(|s| (*en, p.0, v.0, s.damage)))
            .collect();
        for (shot, pos, vel, dmg) in shots {
            if !e.world.alive(shot) {
                continue;
            }
            let motion = vel * dt;
            let box_now = Rect::new(pos.x - 3.0, pos.y - 3.0, 6.0, 6.0);
            // Earliest swept hit among living foes.
            let mut best: Option<(f32, Entity)> = None;
            for (foe, fpos, _) in e.world.query2::<Pos, Foe>() {
                let target = Rect::new(
                    fpos.0.x - FOE_RADIUS,
                    fpos.0.y - FOE_RADIUS,
                    FOE_RADIUS * 2.0,
                    FOE_RADIUS * 2.0,
                );
                if let Some(hit) = collide::swept_aabb(&box_now, motion, &target) {
                    let better = match best {
                        None => true,
                        Some((t, _)) => hit.t < t,
                    };
                    if better {
                        best = Some((hit.t, foe));
                    }
                }
            }
            if let Some((_, foe)) = best {
                e.world.despawn(shot);
                let died = {
                    let f = e.world.get_mut::<Foe>(foe).expect("foe has Foe");
                    f.hp -= dmg;
                    f.hp <= 0
                };
                if died {
                    let at = e.world.get::<Pos>(foe).expect("foe has Pos").0;
                    e.world.despawn(foe);
                    self.kills += 1;
                    self.score += SCORE_KILL;
                    self.events.send(GameEvent::Kill {
                        frame: e.frame(),
                        at,
                    });
                    self.play(e, "boom", 0.6);
                    if e.rng.int_range(0, 4) == 0 {
                        let frame = e.frame();
                        self.spawn_core(e, at, frame);
                    }
                }
                continue;
            }
            let next = pos + motion;
            if self.map.is_solid_at(next)
                || next.x < 0.0
                || next.x > ARENA_W
                || next.y < 0.0
                || next.y > ARENA_H
            {
                e.world.despawn(shot);
                continue;
            }
            if let Some(p) = e.world.get_mut::<Pos>(shot) {
                p.0 = next;
            }
        }
    }

    fn game_over(&mut self, e: &mut Engine) {
        let stats = RunStats {
            score: self.score,
            kills: self.kills,
            wave: self.wave,
            end_frame: e.frame(),
        };
        if self.run == 1 {
            self.run1 = Some(stats);
        } else {
            self.run2 = Some(stats);
        }
        self.events.send(GameEvent::GameOver {
            frame: e.frame(),
            score: self.score,
        });
        self.play(e, "boom", 0.8);
        println!(
            "arena: game over run {} (frame {}, score {}, kills {})",
            self.run, stats.end_frame, stats.score, stats.kills
        );
        self.states.replace(Phase::GameOver);
    }

    fn drain_events(&mut self, e: &Engine) {
        for ev in self.events.drain() {
            match ev {
                GameEvent::Kill { frame, at } => {
                    self.trace = fold(self.trace, 1);
                    self.trace = fold(self.trace, frame);
                    self.trace = fold(self.trace, q(at.x));
                    self.trace = fold(self.trace, q(at.y));
                }
                GameEvent::HeroHit { frame, hp } => {
                    self.trace = fold(self.trace, 2);
                    self.trace = fold(self.trace, frame);
                    self.trace = fold(self.trace, hp as u64);
                }
                GameEvent::WaveClear { frame, wave } => {
                    self.trace = fold(self.trace, 3);
                    self.trace = fold(self.trace, frame);
                    self.trace = fold(self.trace, wave as u64);
                    self.trace = fold(self.trace, self.score as u64);
                    println!("arena: wave {wave} clear (frame {frame}, score {})", self.score);
                }
                GameEvent::Pickup { frame } => {
                    self.trace = fold(self.trace, 4);
                    self.trace = fold(self.trace, frame);
                    self.trace = fold(self.trace, self.score as u64);
                }
                GameEvent::GameOver { frame, score } => {
                    self.trace = fold(self.trace, 5);
                    self.trace = fold(self.trace, frame);
                    self.trace = fold(self.trace, score as u64);
                    let _ = e;
                }
            }
        }
    }

    // ── drawing ─────────────────────────────────────────────────────────────

    fn draw(&mut self, e: &mut Engine) {
        let dt = e.dt();
        // Tiles.
        let view = Rect::new(0.0, 0.0, ARENA_W, ARENA_H);
        for (col, row, id) in self.map.visible_tiles(&view) {
            let wp = self.map.tile_world_pos(col, row);
            let color = if id == 1 {
                Color::rgb8(52, 58, 82)
            } else {
                Color::rgb8(30, 34, 48)
            };
            e.render.rect(wp.x, wp.y, TILE - 1.0, TILE - 1.0, color);
        }

        match self.states.current() {
            Phase::Title => {
                e.render.text(
                    ARENA_W / 2.0 - 90.0,
                    ARENA_H / 2.0 - 40.0,
                    48.0,
                    Color::WHITE,
                    "ARENA",
                );
                e.render.text(
                    ARENA_W / 2.0 - 110.0,
                    ARENA_H / 2.0 + 20.0,
                    20.0,
                    Color::rgb8(160, 170, 200),
                    "press Enter to play",
                );
            }
            Phase::Playing | Phase::GameOver => {
                // Cores.
                let cores: Vec<Vec2> = e
                    .world
                    .query2::<Pos, Core>()
                    .iter()
                    .map(|(_, p, _)| p.0)
                    .collect();
                for c in cores {
                    e.render
                        .circle_fill(c.x, c.y, 7.0, Color::rgb8(90, 230, 140));
                }
                // Foes (animated sprites).
                let foes: Vec<Entity> = e.world.query::<Foe>().iter().map(|(en, _)| *en).collect();
                for foe in foes {
                    let pos = match e.world.get::<Pos>(foe) {
                        Some(p) => p.0,
                        None => continue,
                    };
                    let sprite = match e.world.get_mut::<Anim>(foe) {
                        Some(a) => a.0.tick(dt),
                        None => continue,
                    };
                    e.render.sprite(
                        pos.x - 16.0,
                        pos.y - 16.0,
                        32.0,
                        32.0,
                        0,
                        sprite.x,
                        sprite.y,
                        sprite.w,
                        sprite.h,
                        Color::rgb8(255, 120, 120),
                    );
                }
                // Shots.
                let shots: Vec<Vec2> = e
                    .world
                    .query2::<Pos, Shot>()
                    .iter()
                    .map(|(_, p, _)| p.0)
                    .collect();
                for s in shots {
                    e.render
                        .rect(s.x - 3.0, s.y - 3.0, 6.0, 6.0, Color::rgb8(255, 240, 170));
                }
                // Hero (animated sprite).
                if let Some(hero) = self.hero {
                    let pos = e.world.get::<Pos>(hero).map(|p| p.0);
                    if let (Some(pos), Some(a)) = (pos, e.world.get_mut::<Anim>(hero)) {
                        let sprite = a.0.tick(dt);
                        let tint = if e.frame()
                            < e.world
                                .get::<Hero>(hero)
                                .map(|h| h.invuln_until)
                                .unwrap_or(0)
                        {
                            Color::rgb8(255, 255, 255)
                        } else {
                            Color::rgb8(140, 200, 255)
                        };
                        e.render.sprite(
                            pos.x - 16.0,
                            pos.y - 16.0,
                            32.0,
                            32.0,
                            0,
                            sprite.x,
                            sprite.y,
                            sprite.w,
                            sprite.h,
                            tint,
                        );
                    }
                }
                // HUD.
                let hp = self
                    .hero
                    .and_then(|h| e.world.get::<Hero>(h))
                    .map(|h| h.hp)
                    .unwrap_or(0);
                e.render.text(
                    12.0,
                    10.0,
                    20.0,
                    Color::WHITE,
                    format!(
                        "score {}  hp {}  wave {}  foes {}",
                        self.score,
                        hp,
                        self.wave,
                        e.world.count::<Foe>()
                    ),
                );
                if *self.states.current() == Phase::GameOver {
                    e.render.text(
                        ARENA_W / 2.0 - 130.0,
                        ARENA_H / 2.0 - 20.0,
                        40.0,
                        Color::rgb8(255, 120, 120),
                        "GAME OVER",
                    );
                    e.render.text(
                        ARENA_W / 2.0 - 120.0,
                        ARENA_H / 2.0 + 30.0,
                        20.0,
                        Color::WHITE,
                        format!("final score {}", self.score),
                    );
                }
            }
        }
    }
}

// ─── Entry point ────────────────────────────────────────────────────────────

fn main() {
    let mut game = Game::new();
    Engine::new()
        .seed(SEED)
        .run(|e| game.tick(e))
        .expect("engine run");

    // ── the verdict: every printed number below is asserted ────────────────
    let run1 = game.run1.expect("run 1 must end in game over");
    let run2 = game.run2.expect("run 2 must end in game over");
    assert_eq!(game.title_exit_frame, Some(19));
    assert!(game.stale_handle_rejected);
    assert_eq!(game.clip_count, 4);
    assert!(game.save_verified);
    assert_eq!(game.anim_probe, Some(2));
    assert!(game.mix_peak > 0.3 && game.mix_peak < 1.5);
    assert!(game.last_frame < 600);

    println!(
        "arena: totals — run1 score {} kills {} wave {} / run2 score {} kills {} wave {}",
        run1.score, run1.kills, run1.wave, run2.score, run2.kills, run2.wave
    );
    println!(
        "arena: sfx {} plays, shots {}, audio peak {:.3}, last frame {}, draws {}",
        game.sfx_plays, game.shots_fired, game.mix_peak, game.last_frame, game.draws_last
    );
    println!("arena: final state hash {:#018x}", game.trace);
    println!("ARENA OK — two full runs, save/load verified, all numbers asserted");
}
