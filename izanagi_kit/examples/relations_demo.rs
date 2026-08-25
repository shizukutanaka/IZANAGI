//! Entity relationship tree demo: `Relations` parent/child hierarchy.
//!
//! Builds a small ownership forest — a hero wielding equipment (some socketed)
//! and commanding a summoned familiar — then exercises every `Relations` query:
//!
//! - `attach(child, parent)` — wire the tree; returns `false` on a cycle.
//! - `parent_of` / `children_of` — navigate up and down.
//! - `depth` / `is_root` / `is_leaf` / `is_ancestor` — structural queries.
//! - `remove_entity` — detach a node; its children become roots.
//!
//! The left panel renders the hierarchy as an indented tree (depth from
//! `Relations::depth`); the right panel logs each operation and its result,
//! including a rejected cycle attempt and a re-parenting after removal.
//!
//! Run with `cargo run --example relations_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::entity::{Entity, EntityAllocator};
use izanagi_kit::{Cell, Relations, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const DIV_X: i32 = 38;
const LOG_X: i32 = 40;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 16 };
const TITLE_BG: Color = Color {
    r: 18,
    g: 16,
    b: 54,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 205,
    b: 255,
};
const HDR_FG: Color = Color {
    r: 115,
    g: 115,
    b: 175,
};
const TREE_FG: Color = Color {
    r: 90,
    g: 105,
    b: 150,
};
const ROOT_FG: Color = Color {
    r: 255,
    g: 215,
    b: 110,
};
const NODE_FG: Color = Color {
    r: 150,
    g: 210,
    b: 255,
};
const LEAF_FG: Color = Color {
    r: 130,
    g: 220,
    b: 160,
};
const OK_FG: Color = Color {
    r: 90,
    g: 210,
    b: 110,
};
const FAIL_FG: Color = Color {
    r: 230,
    g: 90,
    b: 90,
};
const INFO_FG: Color = Color {
    r: 140,
    g: 150,
    b: 200,
};
const STAT_FG: Color = Color {
    r: 140,
    g: 195,
    b: 255,
};

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut alloc = EntityAllocator::new();
    let hero = alloc.allocate();
    let sword = alloc.allocate();
    let rune = alloc.allocate();
    let shield = alloc.allocate();
    let familiar = alloc.allocate();
    let spark = alloc.allocate();

    let name = |e: Entity| -> &'static str {
        match e.index() {
            i if i == hero.index() => "Hero",
            i if i == sword.index() => "Sword",
            i if i == rune.index() => "Rune",
            i if i == shield.index() => "Shield",
            i if i == familiar.index() => "Familiar",
            i if i == spark.index() => "Spark",
            _ => "?",
        }
    };

    let mut rel = Relations::new();
    let mut log: Vec<(String, Option<bool>)> = Vec::new();

    // Build the tree; each attach records its boolean result.
    let do_attach = |rel: &mut Relations, log: &mut Vec<(String, Option<bool>)>, c, p| {
        let ok = rel.attach(c, p);
        log.push((format!("attach {}→{}", name(c), name(p)), Some(ok)));
    };
    do_attach(&mut rel, &mut log, sword, hero);
    do_attach(&mut rel, &mut log, shield, hero);
    do_attach(&mut rel, &mut log, familiar, hero);
    do_attach(&mut rel, &mut log, rune, sword); // rune socketed in sword
    do_attach(&mut rel, &mut log, spark, familiar);

    // Cycle attempt: Hero is already a descendant-ancestor of Rune's chain.
    let cyc = rel.attach(hero, rune);
    log.push((
        format!("attach {}→{} (cycle)", name(hero), name(rune)),
        Some(cyc),
    ));

    // Structural queries logged as info lines.
    log.push((format!("depth(Spark)={}", rel.depth(spark)), None));
    log.push((
        format!("ancestor(Hero,Spark)={}", rel.is_ancestor(hero, spark)),
        None,
    ));
    log.push((
        format!("children(Hero)={}", rel.children_of(hero).len()),
        None,
    ));

    // Remove the Familiar: Spark loses its parent and becomes a root.
    rel.remove_entity(familiar);
    log.push((format!("remove {}", name(familiar)), None));
    log.push((format!("Spark root now? {}", rel.is_root(spark)), None));

    // Re-parent the orphaned Spark onto the Hero.
    let re = rel.attach(spark, hero);
    log.push((
        format!("attach {}→{} (reparent)", name(spark), name(hero)),
        Some(re),
    ));

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: STAT_FG,
        bg: BG,
    });

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
        " ENTITY RELATIONS   parent/child forest + cycle guard",
        TITLE_FG,
        TITLE_BG,
    );

    screen.draw_str(2, 2, "OWNERSHIP TREE (final)", HDR_FG, BG);
    screen.draw_str(LOG_X, 2, "OPERATION LOG", HDR_FG, BG);
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┬' } else { '─' };
        screen.set(x, 3, g, TREE_FG, BG);
    }
    for y in 4..22 {
        screen.set(DIV_X, y, '│', TREE_FG, BG);
    }

    // Render the forest as an indented tree. A node is "live" if it still has a
    // parent or any children; live roots (no parent) seed the DFS.
    let all = [hero, sword, rune, shield, familiar, spark];
    let roots: Vec<Entity> = all
        .iter()
        .copied()
        .filter(|&e| {
            let live = rel.parent_of(e).is_some() || !rel.children_of(e).is_empty();
            live && rel.is_root(e)
        })
        .collect();

    let mut row = 4i32;
    // DFS render helper (manual stack to keep ordering deterministic).
    fn render_node(
        screen: &mut Screen,
        rel: &Relations,
        e: Entity,
        row: &mut i32,
        name: &dyn Fn(Entity) -> &'static str,
    ) {
        if *row >= 21 {
            return;
        }
        let depth = rel.depth(e) as i32;
        let indent = 2 + depth * 3;
        let prefix = if depth == 0 { "●" } else { "└─" };
        let px = if depth == 0 { indent } else { indent - 2 };
        screen.draw_str(px, *row, prefix, TREE_FG, BG);

        let fg = if rel.is_root(e) {
            ROOT_FG
        } else if rel.is_leaf(e) {
            LEAF_FG
        } else {
            NODE_FG
        };
        let label = format!(
            "{}  [d{} {}]",
            name(e),
            depth,
            if rel.is_leaf(e) { "leaf" } else { "node" },
        );
        screen.draw_str(indent + 1, *row, &label, fg, BG);
        *row += 1;

        // Children in stable index order.
        let mut kids = rel.children_of(e);
        kids.sort_by_key(|k| k.index());
        for k in kids {
            render_node(screen, rel, k, row, name);
        }
    }

    for r in &roots {
        render_node(&mut screen, &rel, *r, &mut row, &name);
    }

    // Operation log on the right.
    let mut ly = 4i32;
    for (msg, result) in &log {
        if ly >= 22 {
            break;
        }
        let (mark, mark_fg) = match result {
            Some(true) => ("ok ", OK_FG),
            Some(false) => ("REJ", FAIL_FG),
            None => (" · ", INFO_FG),
        };
        screen.draw_str(LOG_X, ly, mark, mark_fg, BG);
        let body_fg = if result.is_none() { INFO_FG } else { STAT_FG };
        let body: String = msg.chars().take(36).collect();
        screen.draw_str(LOG_X + 4, ly, &body, body_fg, BG);
        ly += 1;
    }

    // Bottom separator + stats.
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, 22, g, TREE_FG, BG);
    }
    let stat = format!(
        " relations={}  hero_children={}  spark_depth={}  spark_root_before_reparent=false",
        rel.len(),
        rel.children_of(hero).len(),
        rel.depth(spark),
    );
    screen.draw_str(0, 23, &stat, STAT_FG, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nEntity relations demo.\n\
         Built a 6-entity ownership forest; cycle attach(Hero→Rune) was rejected.\n\
         remove(Familiar) orphaned Spark (became root); reparented onto Hero.\n\
         final relations={}  hero_children={}  spark_depth={}",
        rel.len(),
        rel.children_of(hero).len(),
        rel.depth(spark),
    );
}
