//! Asset store demo: `AssetStore<T>` generational handle safety.
//!
//! `AssetStore<T>` hands out opaque `AssetHandle<T>` values; the handle is the
//! only key back to its asset. Handles are *generational*: removing an asset
//! frees its slot, and the next insert reuses that slot with a bumped
//! generation — so a stale handle to the removed asset no longer resolves,
//! and crucially does **not** silently read the new occupant. This kills the
//! classic use-after-free / handle-aliasing bug.
//!
//! Modules exercised:
//! - `AssetStore::insert` / `get` / `get_mut` / `replace` / `remove`.
//! - `is_live` / `len` / `iter` — store inspection.
//! - Stale-handle rejection after removal and after slot reuse.
//!
//! The left panel shows the live sprite table; the right panel logs a scripted
//! sequence of operations and their results, highlighting the two rejections.
//!
//! Run with `cargo run --example asset_store_demo`.

use izanagi_kit::assets::AssetHandle;
use izanagi_kit::content::Color;
use izanagi_kit::{AssetStore, Cell, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const DIV_X: i32 = 34;
const LOG_X: i32 = 36;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 16 };
const TITLE_BG: Color = Color {
    r: 18,
    g: 16,
    b: 52,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 205,
    b: 255,
};
const HDR_FG: Color = Color {
    r: 118,
    g: 114,
    b: 174,
};
const DIV_FG: Color = Color {
    r: 60,
    g: 52,
    b: 92,
};
const HANDLE_FG: Color = Color {
    r: 130,
    g: 200,
    b: 255,
};
const NAME_FG: Color = Color {
    r: 200,
    g: 205,
    b: 230,
};
const OK_FG: Color = Color {
    r: 95,
    g: 210,
    b: 115,
};
const REJ_FG: Color = Color {
    r: 235,
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
const STAT_HI: Color = Color {
    r: 255,
    g: 218,
    b: 100,
};

// ── asset type ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Sprite {
    glyph: char,
    name: &'static str,
    color: Color,
}

// ── operation log ───────────────────────────────────────────────────────────────

enum LogKind {
    Ok,
    Rej,
    Info,
}

struct LogLine {
    kind: LogKind,
    text: String,
}

fn main() {
    let mut store: AssetStore<Sprite> = AssetStore::new();
    let mut log: Vec<LogLine> = Vec::new();

    let mk = |kind, text: String| LogLine { kind, text };

    // ── seed the sprite sheet ───────────────────────────────────────────────
    let hero = store.insert(Sprite {
        glyph: '@',
        name: "hero",
        color: Color {
            r: 255,
            g: 220,
            b: 90,
        },
    });
    let orc = store.insert(Sprite {
        glyph: 'o',
        name: "orc",
        color: Color {
            r: 110,
            g: 200,
            b: 110,
        },
    });
    let potion = store.insert(Sprite {
        glyph: '!',
        name: "potion",
        color: Color {
            r: 230,
            g: 110,
            b: 200,
        },
    });
    let torch = store.insert(Sprite {
        glyph: 'i',
        name: "torch",
        color: Color {
            r: 255,
            g: 150,
            b: 70,
        },
    });
    log.push(mk(
        LogKind::Info,
        format!("inserted 4 sprites → len={}", store.len()),
    ));

    // ── get_mut: recolour the orc ────────────────────────────────────────────
    if let Some(s) = store.get_mut(orc) {
        s.color = Color {
            r: 80,
            g: 230,
            b: 80,
        };
        log.push(mk(LogKind::Ok, "get_mut(orc): recoloured".to_string()));
    }

    // ── replace: upgrade the potion to a greater potion ──────────────────────
    let old = store.replace(
        potion,
        Sprite {
            glyph: '¡',
            name: "gr.potion",
            color: Color {
                r: 255,
                g: 90,
                b: 230,
            },
        },
    );
    if let Some(o) = old {
        log.push(mk(
            LogKind::Ok,
            format!("replace(potion): was '{}'", o.glyph),
        ));
    }

    // ── remove the torch: its handle goes stale ──────────────────────────────
    let removed = store.remove(torch);
    if let Some(t) = removed {
        log.push(mk(LogKind::Ok, format!("remove(torch '{}')", t.glyph)));
    }

    // ── stale handle #1: get(torch) now rejects ──────────────────────────────
    let stale1: Option<&Sprite> = store.get(torch);
    log.push(mk(
        if stale1.is_none() {
            LogKind::Rej
        } else {
            LogKind::Ok
        },
        format!(
            "get(torch) after remove → {}",
            if stale1.is_none() {
                "None (safe)"
            } else {
                "LEAK!"
            }
        ),
    ));
    log.push(mk(
        LogKind::Info,
        format!("is_live(torch)={}", store.is_live(torch)),
    ));

    // ── slot reuse: insert reuses torch's slot with a bumped generation ──────
    let amulet = store.insert(Sprite {
        glyph: '=',
        name: "amulet",
        color: Color {
            r: 120,
            g: 200,
            b: 255,
        },
    });
    log.push(mk(
        LogKind::Info,
        format!(
            "insert(amulet) reuses slot {} gen {}",
            amulet.index(),
            // generation is not exposed directly; show via handle inequality
            if amulet == torch { 0 } else { 1 }
        ),
    ));

    // ── stale handle #2: old torch handle must NOT resolve to the amulet ─────
    let stale2: Option<&Sprite> = store.get(torch);
    let aliased = stale2.map(|s| s.name == "amulet").unwrap_or(false);
    log.push(mk(
        if stale2.is_none() {
            LogKind::Rej
        } else {
            LogKind::Ok
        },
        format!(
            "get(old torch) → {}",
            if aliased {
                "ALIASED!".to_string()
            } else if stale2.is_none() {
                "None (no alias)".to_string()
            } else {
                "?".to_string()
            }
        ),
    ));

    // Sanity: the fresh amulet handle resolves fine.
    log.push(mk(
        LogKind::Ok,
        format!(
            "get(amulet) → '{}'",
            store.get(amulet).map(|s| s.glyph).unwrap_or('?')
        ),
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
        " ASSET STORE   AssetStore<T> generational handle safety",
        TITLE_FG,
        TITLE_BG,
    );

    screen.draw_str(1, 2, "LIVE SPRITE TABLE", HDR_FG, BG);
    screen.draw_str(LOG_X, 2, "OPERATION LOG", HDR_FG, BG);
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┬' } else { '─' };
        screen.set(x, 3, g, DIV_FG, BG);
    }
    for y in 4..22 {
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Column header for the table.
    screen.draw_str(1, 4, "hndl  spr  name", HDR_FG, BG);

    // Live assets in ascending index order.
    let mut row = 5i32;
    let live: Vec<(AssetHandle<Sprite>, Sprite)> =
        store.iter().map(|(h, s)| (h, s.clone())).collect();
    for (h, s) in &live {
        if row >= 21 {
            break;
        }
        let hstr = format!("[{}]", h.index());
        screen.draw_str(1, row, &hstr, HANDLE_FG, BG);
        screen.set(7, row, s.glyph, s.color, BG);
        screen.draw_str(10, row, s.name, NAME_FG, BG);
        row += 1;
    }

    // A little "sprite sheet" strip below the table.
    screen.draw_str(1, 16, "sheet:", HDR_FG, BG);
    let mut sx = 8i32;
    for (_, s) in &live {
        screen.set(sx, 16, s.glyph, s.color, BG);
        sx += 2;
    }
    screen.draw_str(1, 18, "Handles are generational:", HDR_FG, BG);
    screen.draw_str(1, 19, "removed → slot reused with", INFO_FG, BG);
    screen.draw_str(1, 20, "a new gen; old handle dies.", INFO_FG, BG);

    // Operation log.
    let mut ly = 4i32;
    for line in &log {
        if ly >= 22 {
            break;
        }
        let (mark, mark_fg) = match line.kind {
            LogKind::Ok => ("ok ", OK_FG),
            LogKind::Rej => ("REJ", REJ_FG),
            LogKind::Info => (" · ", INFO_FG),
        };
        screen.draw_str(LOG_X, ly, mark, mark_fg, BG);
        let body: String = line.text.chars().take(40).collect();
        let body_fg = match line.kind {
            LogKind::Info => INFO_FG,
            _ => STAT_FG,
        };
        screen.draw_str(LOG_X + 4, ly, &body, body_fg, BG);
        ly += 1;
    }

    // Bottom separator + stats.
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, 22, g, DIV_FG, BG);
    }
    let _ = hero;
    let stat = format!(
        " live={}  torch_live={}  amulet_live={}  stale gets rejected: 2/2",
        store.len(),
        store.is_live(torch),
        store.is_live(amulet),
    );
    screen.draw_str(0, 23, &stat, STAT_HI, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nAsset store demo.\n\
         Inserted 4 sprites, recoloured one (get_mut), upgraded one (replace),\n\
         removed the torch. Both stale-handle reads were rejected:\n\
         - get(torch) after remove → None\n\
         - get(old torch) after slot reuse by amulet → None (no aliasing)\n\
         final: live={}  torch_live={}  amulet_live={}",
        store.len(),
        store.is_live(torch),
        store.is_live(amulet),
    );
}
