//! Status effects, inventory, and turn scheduling demo.
//!
//! Three actors (Hero, Orc, Mage) fight for `SIM_TICKS` scheduler turns.
//! Demonstrates:
//!   - `StatusSet<K>` — Regen, Haste, Poison applied, ticked, and expired
//!   - `Inventory<T>` — HealthPotion, Antidote, SpeedDraught consumed in-combat
//!   - `Scheduler<u32>` — energy-based speed-weighted turn order
//!   - `Stats` + `melee_attack` / `ranged_attack` — integer combat
//!   - `BarWidget` — HP fill-bar rendering
//!
//! Left panel: final actor state.  Right panel: turn event log.
//!
//! Run with `cargo run --example status_effects_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{
    melee_attack, ranged_attack, BarWidget, Cell, Inventory, Scheduler, Screen, SplitMix64, Stats,
    StatusSet,
};
use std::io::{self, Write};

const HERO: u32 = 0;
const ORC: u32 = 1;
const MAGE: u32 = 2;
const SIM_TICKS: u32 = 60;
const SEED: u64 = 0xCAFE_5EED_BAD0;
const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

// ── item and status discriminants ─────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    HealthPotion,
    Antidote,
    SpeedDraught,
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::HealthPotion => "HP-Pot",
            Item::Antidote => "Antidot",
            Item::SpeedDraught => "Haste!",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Buff {
    Regen,
    Haste,
    Poison,
}

// ── actor ─────────────────────────────────────────────────────────────────────

struct Actor {
    name: &'static str,
    stats: Stats,
    status: StatusSet<Buff>,
    inv: Inventory<Item>,
    base_speed: i32,
    turns: u32,
}

// ── colours ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 14 };
const DIM: Color = Color {
    r: 70,
    g: 70,
    b: 80,
};
const UI: Color = Color {
    r: 170,
    g: 170,
    b: 180,
};
const TITLE: Color = Color {
    r: 120,
    g: 200,
    b: 255,
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
const BUFF_C: Color = Color {
    r: 80,
    g: 200,
    b: 255,
};
const DEBUFF_C: Color = Color {
    r: 220,
    g: 60,
    b: 200,
};
const DEAD_C: Color = Color {
    r: 90,
    g: 90,
    b: 100,
};
const ITEM_C: Color = Color {
    r: 220,
    g: 180,
    b: 80,
};
const LOG_HIT: Color = Color {
    r: 220,
    g: 120,
    b: 80,
};
const LOG_HEAL: Color = Color {
    r: 80,
    g: 220,
    b: 120,
};
const LOG_DEATH: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};

fn hp_color(hp: i32, max: i32) -> Color {
    if hp * 3 > max * 2 {
        HP_OK
    } else if hp * 2 > max {
        HP_WARN
    } else {
        HP_CRIT
    }
}

fn blank() -> Cell {
    Cell {
        glyph: ' ',
        fg: UI,
        bg: BG,
    }
}

fn log_color(s: &str) -> Color {
    if s.contains("slain") || s.contains("died from") {
        LOG_DEATH
    } else if s.contains("heal")
        || s.contains("Potion")
        || s.contains("Regen")
        || s.contains("dispelled")
    {
        LOG_HEAL
    } else if s.contains("dmg") || s.contains("missed") {
        LOG_HIT
    } else {
        UI
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // ── actors ────────────────────────────────────────────────────────────────
    let mut actors: Vec<Actor> = vec![
        Actor {
            name: "Hero",
            stats: Stats::new(40, 8, 3),
            status: StatusSet::new(),
            inv: {
                let mut i = Inventory::new(4);
                i.add(Item::HealthPotion);
                i.add(Item::HealthPotion);
                i.add(Item::Antidote);
                i
            },
            base_speed: 100,
            turns: 0,
        },
        Actor {
            name: "Orc",
            stats: Stats::new(28, 7, 2),
            status: StatusSet::new(),
            inv: Inventory::new(2),
            base_speed: 80,
            turns: 0,
        },
        Actor {
            name: "Mage",
            stats: Stats::new(18, 11, 1),
            status: StatusSet::new(),
            inv: {
                let mut i = Inventory::new(2);
                i.add(Item::SpeedDraught);
                i
            },
            base_speed: 120,
            turns: 0,
        },
    ];

    let mut sched: Scheduler<u32> = Scheduler::new();
    sched.add(HERO, actors[0].base_speed);
    sched.add(ORC, actors[1].base_speed);
    sched.add(MAGE, actors[2].base_speed);

    let mut rng = SplitMix64::new(SEED);
    let mut log: Vec<String> = Vec::new();

    // ── simulation loop ───────────────────────────────────────────────────────
    for _ in 0..SIM_TICKS {
        let Some(id) = sched.next_turn() else { break };
        let ix = id as usize;
        if !actors[ix].stats.is_alive() {
            continue;
        }
        actors[ix].turns += 1;

        match id {
            HERO => {
                let poisoned = actors[0].status.is_active(&Buff::Poison);
                let hp_low = actors[0].stats.hp * 2 <= actors[0].stats.max_hp;
                let antidote = actors[0].inv.find(|i| *i == Item::Antidote);
                let potion = actors[0].inv.find(|i| *i == Item::HealthPotion);
                let orc_alive = actors[1].stats.is_alive();
                let mage_alive = actors[2].stats.is_alive();

                if let Some(slot) = antidote.filter(|_| poisoned) {
                    actors[0].inv.remove(slot);
                    actors[0].status.remove(&Buff::Poison);
                    log.push("Hero uses Antidote — Poison dispelled!".into());
                } else if let Some(slot) = potion.filter(|_| hp_low) {
                    actors[0].inv.remove(slot);
                    actors[0].stats.heal(15);
                    actors[0].status.apply(Buff::Regen, 5, 2);
                    log.push(format!(
                        "Hero: Health Potion (+15, Regen+2×5t) → HP {}/{}",
                        actors[0].stats.hp, actors[0].stats.max_hp,
                    ));
                } else if orc_alive {
                    let atk = actors[0].stats.clone();
                    let dmg = melee_attack(&atk, &mut actors[1].stats);
                    log.push(format!(
                        "Hero → Orc: {}dmg (Orc HP {}/{})",
                        dmg, actors[1].stats.hp, actors[1].stats.max_hp,
                    ));
                    if !actors[1].stats.is_alive() {
                        sched.remove(ORC);
                        log.push("  → Orc slain!".into());
                    }
                } else if mage_alive {
                    let atk = actors[0].stats.clone();
                    let dmg = melee_attack(&atk, &mut actors[2].stats);
                    log.push(format!(
                        "Hero → Mage: {}dmg (Mage HP {}/{})",
                        dmg, actors[2].stats.hp, actors[2].stats.max_hp,
                    ));
                    if !actors[2].stats.is_alive() {
                        sched.remove(MAGE);
                        log.push("  → Mage slain!".into());
                    }
                } else {
                    log.push("Hero: no targets remaining.".into());
                }
            }
            ORC => {
                if actors[0].stats.is_alive() {
                    let atk = actors[1].stats.clone();
                    let dmg = melee_attack(&atk, &mut actors[0].stats);
                    log.push(format!(
                        "Orc → Hero: {}dmg (Hero HP {}/{})",
                        dmg, actors[0].stats.hp, actors[0].stats.max_hp,
                    ));
                    if !actors[0].stats.is_alive() {
                        sched.remove(HERO);
                        log.push("  → Hero slain!".into());
                    }
                }
            }
            MAGE => {
                // Use SpeedDraught on first turn
                if let Some(slot) = actors[2].inv.find(|i| *i == Item::SpeedDraught) {
                    actors[2].inv.remove(slot);
                    actors[2].status.apply(Buff::Haste, 6, 50);
                    let new_spd = actors[2].base_speed + 50;
                    sched.set_speed(MAGE, new_spd);
                    log.push(format!(
                        "Mage: SpeedDraught! Haste+50spd×6t (speed→{})",
                        new_spd,
                    ));
                }
                // Ranged attack with Poison on hit
                if actors[0].stats.is_alive() {
                    let atk = actors[2].stats.clone();
                    match ranged_attack(&mut rng, &atk, &mut actors[0].stats, 70) {
                        Some(dmg) => {
                            actors[0].status.apply(Buff::Poison, 8, -3);
                            log.push(format!(
                                "Mage → Hero: {}dmg + Poison-3×8t (HP {}/{})",
                                dmg, actors[0].stats.hp, actors[0].stats.max_hp,
                            ));
                            if !actors[0].stats.is_alive() {
                                sched.remove(HERO);
                                log.push("  → Hero slain!".into());
                            }
                        }
                        None => log.push("Mage: ranged attack missed!".into()),
                    }
                }
            }
            _ => unreachable!(),
        }

        // ── tick all statuses; apply per-tick effects before decrement ─────────
        for (i, actor) in actors.iter_mut().enumerate() {
            if !actor.stats.is_alive() {
                continue;
            }
            // Regen: heal on each remaining tick (including the last)
            if actor.status.is_active(&Buff::Regen) {
                let mag = actor.status.get(&Buff::Regen).map_or(0, |e| e.magnitude);
                actor.stats.heal(mag);
            }
            // Poison: damage on each remaining tick
            if actor.status.is_active(&Buff::Poison) {
                let mag = actor.status.get(&Buff::Poison).map_or(0, |e| e.magnitude);
                actor.stats.take_damage((-mag).max(0));
                if !actor.stats.is_alive() {
                    if sched.contains(i as u32) {
                        sched.remove(i as u32);
                    }
                    log.push(format!("  {} died from Poison!", actor.name));
                }
            }
            // Decrement durations; collect expired keys
            let expired = actor.status.tick(1);
            // Restore base speed when Haste expires
            if expired.contains(&Buff::Haste) {
                sched.set_speed(i as u32, actor.base_speed);
                log.push(format!(
                    "{}: Haste expired (speed → {})",
                    actor.name, actor.base_speed,
                ));
            }
        }
    }

    // ── render ───────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.fill_rect(0, 0, SCREEN_W, SCREEN_H, blank());

    // Title
    let title = format!(
        " STATUS/INVENTORY/TURN DEMO  seed={:#016x}  ticks={}",
        SEED, SIM_TICKS,
    );
    screen.draw_str(0, 0, &title, TITLE, BG);

    // Vertical separator
    for y in 1..SCREEN_H as i32 {
        screen.set(39, y, '│', DIM, BG);
    }

    // Actor panels — rows 1..6, 8..13, 15..20
    let panel_ys = [1i32, 8, 15];
    let sep_ys = [7i32, 14, 21];
    for (pi, &py) in panel_ys.iter().enumerate() {
        let a = &actors[pi];
        let alive = a.stats.is_alive();
        let nc = if alive { TITLE } else { DEAD_C };

        // Header
        let hdr = format!("── {} spd:{} turns:{} ", a.name, a.base_speed, a.turns);
        let hdr_pad: String = {
            let s = format!("{:─<38}", hdr);
            s.chars().take(38).collect()
        };
        screen.draw_str(0, py, &hdr_pad, nc, BG);

        // HP bar
        let bar = BarWidget::new(a.stats.hp, a.stats.max_hp, 14).render();
        let hp_line = format!("HP{} {}/{}", bar, a.stats.hp, a.stats.max_hp);
        let hc = if alive {
            hp_color(a.stats.hp, a.stats.max_hp)
        } else {
            DEAD_C
        };
        screen.draw_str(
            0,
            py + 1,
            &hp_line.chars().take(38).collect::<String>(),
            hc,
            BG,
        );

        // ATK / DEF / alive
        let stat_line = format!(
            "ATK:{:<2} DEF:{:<2}  [{}]",
            a.stats.attack,
            a.stats.defense,
            if alive { "alive" } else { "DEAD " }
        );
        screen.draw_str(
            0,
            py + 2,
            &stat_line.chars().take(38).collect::<String>(),
            if alive { UI } else { DEAD_C },
            BG,
        );

        // Status effects
        let sts_prefix = "Sts:";
        screen.draw_str(0, py + 3, sts_prefix, DIM, BG);
        let mut sx = sts_prefix.len() as i32;
        if a.status.is_empty() {
            screen.draw_str(sx, py + 3, "none", DIM, BG);
        } else {
            for (buff, eff) in a.status.iter() {
                if sx >= 38 {
                    break;
                }
                let (tag, col) = match buff {
                    Buff::Regen => (
                        format!("Regen+{}({}t) ", eff.magnitude, eff.remaining),
                        BUFF_C,
                    ),
                    Buff::Haste => (
                        format!("Haste+{}({}t) ", eff.magnitude, eff.remaining),
                        BUFF_C,
                    ),
                    Buff::Poison => (
                        format!("Poison{}({}t) ", eff.magnitude, eff.remaining),
                        DEBUFF_C,
                    ),
                };
                let avail = (38 - sx).max(0) as usize;
                let ts: String = tag.chars().take(avail).collect();
                screen.draw_str(sx, py + 3, &ts, col, BG);
                sx += ts.len() as i32;
            }
        }

        // Inventory
        let inv_prefix = "Inv:";
        screen.draw_str(0, py + 4, inv_prefix, DIM, BG);
        let mut ix = inv_prefix.len() as i32;
        if a.inv.is_empty() {
            screen.draw_str(ix, py + 4, "[empty]", DIM, BG);
        } else {
            for (_, item) in a.inv.iter() {
                if ix >= 38 {
                    break;
                }
                let tag = format!("[{}]", item.label());
                let avail = (38 - ix).max(0) as usize;
                let ts: String = tag.chars().take(avail).collect();
                screen.draw_str(ix, py + 4, &ts, ITEM_C, BG);
                ix += ts.len() as i32;
            }
        }

        // Panel separator
        if pi < 2 {
            screen.draw_str(0, sep_ys[pi], &"─".repeat(38), DIM, BG);
        }
    }

    // Summary rows 21..23
    screen.draw_str(0, 21, &"─".repeat(38), DIM, BG);
    let survivors: Vec<&str> = actors
        .iter()
        .filter(|a| a.stats.is_alive())
        .map(|a| a.name)
        .collect();
    let surv_str = if survivors.is_empty() {
        "none".to_string()
    } else {
        survivors.join(", ")
    };
    screen.draw_str(
        0,
        22,
        &format!("Survivors: {}", surv_str)
            .chars()
            .take(38)
            .collect::<String>(),
        HP_OK,
        BG,
    );
    screen.draw_str(
        0,
        23,
        &format!(
            "H:{:>2}t O:{:>2}t M:{:>2}t  log:{}",
            actors[0].turns,
            actors[1].turns,
            actors[2].turns,
            log.len(),
        )
        .chars()
        .take(38)
        .collect::<String>(),
        UI,
        BG,
    );

    // Right panel: turn log
    screen.draw_str(40, 1, "── TURN LOG ──────────────────────────", TITLE, BG);
    let max_log_rows = (SCREEN_H as usize).saturating_sub(2); // rows 2..23
    let start = log.len().saturating_sub(max_log_rows);
    for (i, msg) in log[start..].iter().enumerate() {
        let y = 2 + i as i32;
        if y >= SCREEN_H as i32 {
            break;
        }
        let col = log_color(msg);
        let trimmed: String = msg.chars().take(39).collect();
        screen.draw_str(40, y, &trimmed, col, BG);
    }

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nStatus effects demo.  seed={:#018x}  log_entries={}  survivors={:?}",
        SEED,
        log.len(),
        actors
            .iter()
            .filter(|a| a.stats.is_alive())
            .map(|a| a.name)
            .collect::<Vec<_>>(),
    );
}
