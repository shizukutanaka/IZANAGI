//! AI behaviour demo — FSM, SpatialHash, Cooldown, and TimerQueue.
//!
//! Four guards patrol a dungeon room. Each guard owns:
//!   - `Fsm<GuardState, GuardEvent>` — Idle → Alert → Chase → Dead
//!   - `Cooldown` — per-guard attack rate limiting (reset every 4 ticks)
//!   - `TimerQueue<()>` — patrol beat; advances to the next waypoint every 5 ticks
//!   - `SpatialHash<u32>` — `query_rect` for O(1) proximity detection
//!
//! Each tick the simulation:
//!   1. Advances patrol timers + ticks attack cooldowns.
//!   2. Runs `query_rect` around the player; fires `PlayerSpotted`/`PlayerLost`
//!      into each guard's FSM.
//!   3. Moves `Chase`-state guards one step toward the player; updates SpatialHash.
//!   4. Guards adjacent to the player attack (if `Cooldown::is_ready`), resetting
//!      the cooldown; the player retaliates, potentially slaying the guard.
//!
//! Left: dungeon map coloured by FSM state.  Right: guard status panel + event log.
//!
//! Run with `cargo run --example ai_behavior_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{Cell, Cooldown, Fsm, Screen, SpatialHash, TimerQueue};
use std::io::{self, Write};

// ── constants ─────────────────────────────────────────────────────────────────

const MAP_W: i32 = 55; // cols 0..54; walls at col 0 and 54
const MAP_H: i32 = 20; // map occupies screen rows 1..20; walls at row 0 and 19
const MAP_Y0: i32 = 1; // map top = screen row 1
const SEP_X: i32 = 55;
const STATUS_X: i32 = 56;
const STATUS_W: usize = 24;
const SIM_TICKS: u32 = 40;
const DETECT_R: i32 = 8;
const PATROL_PERIOD: u32 = 5;
const ATTACK_CD_TICKS: u32 = 4;
const GUARD_ATK: i32 = 3;
const RETALIATE: i32 = 3;
const GUARD_MAX_HP: i32 = 12;
const PLAYER_MAX_HP: i32 = 60;

// ── FSM types ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
enum GuardState {
    Idle,
    Alert,
    Chase,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum GuardEvent {
    PlayerSpotted,
    PlayerLost,
    Killed,
}

fn guard_glyph_color(state: &GuardState) -> (char, Color) {
    match state {
        GuardState::Idle => (
            'g',
            Color {
                r: 60,
                g: 180,
                b: 60,
            },
        ),
        GuardState::Alert => (
            'G',
            Color {
                r: 240,
                g: 200,
                b: 60,
            },
        ),
        GuardState::Chase => (
            'C',
            Color {
                r: 220,
                g: 100,
                b: 40,
            },
        ),
        GuardState::Dead => (
            '%',
            Color {
                r: 90,
                g: 40,
                b: 40,
            },
        ),
    }
}

fn build_fsm() -> Fsm<GuardState, GuardEvent> {
    use GuardEvent::*;
    use GuardState::*;
    let mut fsm = Fsm::new(Idle);
    fsm.add_transition(Idle, PlayerSpotted, Alert);
    fsm.add_transition(Alert, PlayerSpotted, Chase);
    fsm.add_transition(Chase, PlayerLost, Alert);
    fsm.add_transition(Alert, PlayerLost, Idle);
    fsm.add_transition(Idle, Killed, Dead);
    fsm.add_transition(Alert, Killed, Dead);
    fsm.add_transition(Chase, Killed, Dead);
    fsm
}

// ── patrol waypoints ──────────────────────────────────────────────────────────

const WP_A: &[(i32, i32)] = &[(3, 2), (25, 2), (25, 17), (3, 17)];
const WP_B: &[(i32, i32)] = &[(51, 2), (29, 2), (29, 17), (51, 17)];
const WP_C: &[(i32, i32)] = &[(3, 17), (25, 17), (25, 2), (3, 2)];
const WP_D: &[(i32, i32)] = &[(51, 17), (29, 17), (29, 2), (51, 2)];

// ── actor structs ─────────────────────────────────────────────────────────────

struct Guard {
    x: i32,
    y: i32,
    hp: i32,
    fsm: Fsm<GuardState, GuardEvent>,
    attack_cd: Cooldown,
    patrol_timer: TimerQueue<()>,
    waypoints: &'static [(i32, i32)],
    wp_idx: usize,
    id: u32,
    label: char,
    attacks: u32,
}

impl Guard {
    fn new(id: u32, label: char, waypoints: &'static [(i32, i32)]) -> Self {
        let mut patrol_timer = TimerQueue::new();
        patrol_timer.schedule_repeat(PATROL_PERIOD, PATROL_PERIOD, ());
        Guard {
            x: waypoints[0].0,
            y: waypoints[0].1,
            hp: GUARD_MAX_HP,
            fsm: build_fsm(),
            attack_cd: Cooldown::ready(),
            patrol_timer,
            waypoints,
            wp_idx: 0,
            id,
            label,
            attacks: 0,
        }
    }

    fn is_dead(&self) -> bool {
        *self.fsm.state() == GuardState::Dead
    }
}

struct Player {
    x: i32,
    y: i32,
    hp: i32,
    attacks_received: u32,
}

// ── colours ───────────────────────────────────────────────────────────────────

const BG: Color = Color {
    r: 10,
    g: 10,
    b: 15,
};
const WALL_FG: Color = Color {
    r: 80,
    g: 80,
    b: 90,
};
const WALL_BG: Color = Color {
    r: 20,
    g: 20,
    b: 25,
};
const FLOOR_FG: Color = Color {
    r: 25,
    g: 25,
    b: 35,
};
const FLOOR_BG: Color = Color {
    r: 14,
    g: 14,
    b: 20,
};
const DETECT_BG: Color = Color {
    r: 15,
    g: 22,
    b: 35,
};
const PLAYER_C: Color = Color {
    r: 100,
    g: 220,
    b: 240,
};
const TITLE_C: Color = Color {
    r: 120,
    g: 200,
    b: 255,
};
const DIM: Color = Color {
    r: 70,
    g: 70,
    b: 80,
};
const UI: Color = Color {
    r: 160,
    g: 160,
    b: 175,
};
const HP_OK: Color = Color {
    r: 80,
    g: 220,
    b: 80,
};
const HP_WARN: Color = Color {
    r: 240,
    g: 200,
    b: 60,
};
const HP_CRIT: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};
const LOG_HIT: Color = Color {
    r: 220,
    g: 120,
    b: 80,
};
const LOG_DEATH: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};

fn blank() -> Cell {
    Cell {
        glyph: ' ',
        fg: UI,
        bg: BG,
    }
}

fn cheb(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

fn hp_color(hp: i32, max: i32) -> Color {
    if hp * 3 > max * 2 {
        HP_OK
    } else if hp * 2 > max {
        HP_WARN
    } else {
        HP_CRIT
    }
}

fn state_label(s: &GuardState) -> &'static str {
    match s {
        GuardState::Idle => "Idle ",
        GuardState::Alert => "Alert",
        GuardState::Chase => "Chase",
        GuardState::Dead => "Dead ",
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut player = Player {
        x: 27,
        y: 10,
        hp: PLAYER_MAX_HP,
        attacks_received: 0,
    };

    let mut guards = vec![
        Guard::new(1, 'A', WP_A),
        Guard::new(2, 'B', WP_B),
        Guard::new(3, 'C', WP_C),
        Guard::new(4, 'D', WP_D),
    ];

    let mut spatial: SpatialHash<u32> = SpatialHash::new(4);
    spatial.insert(0, player.x, player.y);
    for g in &guards {
        spatial.insert(g.id, g.x, g.y);
    }

    let mut log: Vec<(String, Color)> = Vec::new();
    let mut final_tick = 0u32;

    // ── simulation ────────────────────────────────────────────────────────────
    'sim: for tick in 0..SIM_TICKS {
        final_tick = tick + 1;

        // 1. Advance patrol timers; tick attack cooldowns
        for guard in &mut guards {
            if guard.is_dead() {
                continue;
            }
            let patrol_step = !guard.patrol_timer.advance(1).is_empty();
            guard.attack_cd.tick(1);

            if patrol_step && *guard.fsm.state() == GuardState::Idle {
                guard.wp_idx = (guard.wp_idx + 1) % guard.waypoints.len();
                let (wx, wy) = guard.waypoints[guard.wp_idx];
                spatial.move_entity(guard.id, guard.x, guard.y, wx, wy);
                guard.x = wx;
                guard.y = wy;
            }
        }

        // 2. Detect via SpatialHash query_rect; fire FSM events
        let nearby = spatial.query_rect(
            player.x - DETECT_R,
            player.y - DETECT_R,
            DETECT_R * 2 + 1,
            DETECT_R * 2 + 1,
        );
        for guard in &mut guards {
            if guard.is_dead() {
                continue;
            }
            let spotted = nearby.contains(&guard.id);
            let changed = if spotted {
                guard.fsm.fire(&GuardEvent::PlayerSpotted)
            } else {
                guard.fsm.fire(&GuardEvent::PlayerLost)
            };
            if changed {
                let msg = format!(
                    "t{:02} Guard {} → {}",
                    final_tick,
                    guard.label,
                    state_label(guard.fsm.state())
                );
                log.push((msg, UI));
            }
        }

        // 3. Chase movement: one step toward player
        for guard in &mut guards {
            if guard.is_dead() {
                continue;
            }
            if *guard.fsm.state() == GuardState::Chase {
                let (ox, oy) = (guard.x, guard.y);
                let dx = (player.x - guard.x).signum();
                let dy = (player.y - guard.y).signum();
                guard.x = (guard.x + dx).clamp(1, MAP_W - 2);
                guard.y = (guard.y + dy).clamp(1, MAP_H - 2);
                if (guard.x, guard.y) != (ox, oy) {
                    spatial.move_entity(guard.id, ox, oy, guard.x, guard.y);
                }
            }
        }

        // 4. Attacks (guard → player; player retaliates)
        for guard in &mut guards {
            if guard.is_dead() {
                continue;
            }
            let dist = cheb(guard.x, guard.y, player.x, player.y);
            if dist <= 1 && guard.attack_cd.is_ready() {
                player.hp = (player.hp - GUARD_ATK).max(0);
                player.attacks_received += 1;
                guard.attacks += 1;
                guard.attack_cd.reset(ATTACK_CD_TICKS);

                log.push((
                    format!(
                        "t{:02} Guard {} hits! HP→{}/{}",
                        final_tick, guard.label, player.hp, PLAYER_MAX_HP,
                    ),
                    LOG_HIT,
                ));

                guard.hp = (guard.hp - RETALIATE).max(0);
                if guard.hp == 0 {
                    guard.fsm.fire(&GuardEvent::Killed);
                    spatial.remove(&guard.id, guard.x, guard.y);
                    log.push((
                        format!("t{:02} Guard {} slain!", final_tick, guard.label),
                        LOG_DEATH,
                    ));
                }
            }
        }

        if guards.iter().all(|g| g.is_dead()) {
            break 'sim;
        }
    }

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(80, 24);
    screen.fill_rect(0, 0, 80, 24, blank());

    // Title (left side, 55 chars)
    let title = format!(
        " AI BEHAVIOR DEMO  ticks:{}/{}  cells:{}",
        final_tick,
        SIM_TICKS,
        spatial.cell_count(),
    );
    screen.draw_str(
        0,
        0,
        &title.chars().take(55).collect::<String>(),
        TITLE_C,
        BG,
    );

    // Map background: walls + floor + detection-zone highlight
    for my in 0..MAP_H {
        let sy = MAP_Y0 + my;
        for mx in 0..MAP_W {
            let is_wall = mx == 0 || mx == MAP_W - 1 || my == 0 || my == MAP_H - 1;
            if is_wall {
                screen.set(mx, sy, '#', WALL_FG, WALL_BG);
            } else {
                let dbg = cheb(mx, my, player.x, player.y) <= DETECT_R;
                screen.set(
                    mx,
                    sy,
                    '.',
                    FLOOR_FG,
                    if dbg { DETECT_BG } else { FLOOR_BG },
                );
            }
        }
    }

    // Place guards
    for guard in &guards {
        let (glyph, color) = guard_glyph_color(guard.fsm.state());
        screen.set(guard.x, MAP_Y0 + guard.y, glyph, color, FLOOR_BG);
    }

    // Place player
    screen.set(player.x, MAP_Y0 + player.y, '@', PLAYER_C, DETECT_BG);

    // Separator
    for y in 0..24i32 {
        screen.set(SEP_X, y, '│', DIM, BG);
    }

    // ── status panel ──────────────────────────────────────────────────────────
    let mut sy = 0i32;

    let draw = |screen: &mut Screen, y: i32, text: &str, col: Color| {
        let s: String = text.chars().take(STATUS_W).collect();
        screen.draw_str(STATUS_X, y, &s, col, BG);
    };

    draw(&mut screen, sy, "── AI BEHAVIOR DEMO ────", TITLE_C);
    sy += 1;

    // Guard entries
    draw(&mut screen, sy, "── GUARDS ──────────────", DIM);
    sy += 1;
    for guard in &guards {
        let (_, sc) = guard_glyph_color(guard.fsm.state());
        let hdr = format!(
            "{} [{}] HP:{}/{}",
            guard.label,
            state_label(guard.fsm.state()),
            guard.hp,
            GUARD_MAX_HP
        );
        draw(&mut screen, sy, &hdr, sc);
        sy += 1;
        let cd_str = if guard.attack_cd.is_ready() {
            "ready".to_string()
        } else {
            format!("{}t", guard.attack_cd.remaining)
        };
        let details = format!(
            "  CD:{} dist:{} atk:{}",
            cd_str,
            cheb(guard.x, guard.y, player.x, player.y),
            guard.attacks,
        );
        let dc = if guard.is_dead() { DIM } else { UI };
        draw(&mut screen, sy, &details, dc);
        sy += 1;
    }

    // SpatialHash info
    draw(&mut screen, sy, "── SPATIAL HASH ────────", DIM);
    sy += 1;
    let cell_info = format!("cell_sz:4  cells:{}", spatial.cell_count());
    draw(&mut screen, sy, &cell_info, UI);
    sy += 1;

    // Show who is in detection range of player right now
    let final_nearby = spatial.query_rect(
        player.x - DETECT_R,
        player.y - DETECT_R,
        DETECT_R * 2 + 1,
        DETECT_R * 2 + 1,
    );
    let nearby_labels: Vec<char> = guards
        .iter()
        .filter(|g| final_nearby.contains(&g.id))
        .map(|g| g.label)
        .collect();
    let nearby_str = if nearby_labels.is_empty() {
        "none".to_string()
    } else {
        nearby_labels.iter().collect()
    };
    draw(&mut screen, sy, &format!("nearby[@]: {}", nearby_str), UI);
    sy += 1;

    // Player
    draw(&mut screen, sy, "── PLAYER ──────────────", DIM);
    sy += 1;
    let phc = hp_color(player.hp, PLAYER_MAX_HP);
    draw(
        &mut screen,
        sy,
        &format!("HP:{}/{}", player.hp, PLAYER_MAX_HP),
        phc,
    );
    sy += 1;
    draw(
        &mut screen,
        sy,
        &format!("attacks received: {}", player.attacks_received),
        UI,
    );
    sy += 1;

    // Event log
    draw(&mut screen, sy, "── EVENT LOG ───────────", DIM);
    sy += 1;
    let max_log = (24 - sy).max(0) as usize;
    let log_start = log.len().saturating_sub(max_log);
    for (msg, col) in &log[log_start..] {
        if sy >= 24 {
            break;
        }
        draw(&mut screen, sy, msg, *col);
        sy += 1;
    }

    // Map legend rows 21..23 (left side)
    screen.draw_str(0, 21, "─".repeat(55).as_str(), DIM, BG);
    screen.draw_str(
        0,
        22,
        " g=Idle  G=Alert  C=Chase  %=Dead  @=player",
        DIM,
        BG,
    );
    let alive = guards.iter().filter(|g| !g.is_dead()).count();
    screen.draw_str(
        0,
        23,
        &format!(
            " Guards alive:{}/4  detect_r:{}  patrol_p:{}  atk_cd:{}",
            alive, DETECT_R, PATROL_PERIOD, ATTACK_CD_TICKS
        )
        .chars()
        .take(54)
        .collect::<String>(),
        UI,
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

    let alive_names: Vec<char> = guards
        .iter()
        .filter(|g| !g.is_dead())
        .map(|g| g.label)
        .collect();
    eprintln!(
        "\nAI behavior demo.  ticks={}  log_entries={}  guards_alive={:?}  player_hp={}/{}",
        final_tick,
        log.len(),
        alive_names,
        player.hp,
        PLAYER_MAX_HP,
    );
}
