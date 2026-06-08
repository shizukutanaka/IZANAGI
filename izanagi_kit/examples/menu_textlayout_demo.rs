//! Character-class selection screen demonstrating `Menu<T>` and text-layout helpers.
//!
//! Simulates a roguelike character-creation UI:
//! - `Menu<CharClass>` built with `add_item` / `add_disabled`; Paladin is locked.
//! - Four `move_down()` calls navigate Warrior → Shaman, auto-skipping the disabled
//!   Paladin entry — the skip is driven entirely by `Menu::move_down`.
//! - `wrap_words` fills the right-panel description for the highlighted class.
//! - `center`, `pad_right`, `pad_left`, and `truncate` handle every label and stat cell.
//!
//! Run with `cargo run --example menu_textlayout_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{center, pad_left, pad_right, truncate, wrap_words, Cell, Menu, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const DIV_X: i32 = 26; // vertical divider column
const RIGHT_X: i32 = 27; // right panel start
const RIGHT_W: usize = 53; // 80 - 27
const DESC_X: i32 = 29; // description text indent (2 cols into right panel)
const DESC_W: usize = 50; // wrap width for description text

const SEP_TOP_Y: i32 = 2; // top horizontal separator
const SEP_BOT_Y: i32 = 20; // bottom horizontal separator
const MENU_Y: i32 = 3; // first menu item row
const DESC_TITLE_Y: i32 = 3; // class name in right panel
const DESC_SEP_Y: i32 = 4; // ── separator below class name
const DESC_START_Y: i32 = 5; // first description line

const STATS_Y: i32 = 21; // HP / ATK / DEF / SPD row
const CONFIRM_Y: i32 = 22; // selection confirmation line
const HINT_Y: i32 = 23; // truncated control-hint row

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 10, g: 8, b: 20 };
const TITLE_BG: Color = Color {
    r: 38,
    g: 20,
    b: 75,
};
const TITLE_FG: Color = Color {
    r: 255,
    g: 220,
    b: 100,
};
const SUB_FG: Color = Color {
    r: 128,
    g: 118,
    b: 175,
};
const DIV_FG: Color = Color {
    r: 68,
    g: 52,
    b: 118,
};
const SEL_BG: Color = Color {
    r: 55,
    g: 35,
    b: 110,
};
const SEL_FG: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};
const ITEM_FG: Color = Color {
    r: 195,
    g: 185,
    b: 220,
};
const DIM_FG: Color = Color {
    r: 72,
    g: 68,
    b: 98,
};
const CURSOR_FG: Color = Color {
    r: 255,
    g: 200,
    b: 60,
};
const CLASS_FG: Color = Color {
    r: 120,
    g: 205,
    b: 255,
};
const DESC_FG: Color = Color {
    r: 162,
    g: 168,
    b: 210,
};
const STAT_KEY: Color = Color {
    r: 150,
    g: 135,
    b: 195,
};
const STAT_VAL: Color = Color {
    r: 255,
    g: 225,
    b: 110,
};
const HINT_FG: Color = Color {
    r: 92,
    g: 88,
    b: 126,
};

// ── character classes ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharClass {
    Warrior,
    Ranger,
    Mage,
    Rogue,
    Paladin,
    Shaman,
}

fn class_name(c: CharClass) -> &'static str {
    match c {
        CharClass::Warrior => "Warrior",
        CharClass::Ranger => "Ranger",
        CharClass::Mage => "Mage",
        CharClass::Rogue => "Rogue",
        CharClass::Paladin => "Paladin",
        CharClass::Shaman => "Shaman",
    }
}

fn class_description(c: CharClass) -> &'static str {
    match c {
        CharClass::Warrior => {
            "A stalwart guardian who charges headlong into battle. Unmatched \
             physical endurance and heavy armor allow Warriors to absorb punishment \
             while dealing relentless melee damage. Slow but unstoppable."
        }
        CharClass::Ranger => {
            "Fleet-footed and deadly at range. Rangers weave between enemies, \
             loosing arrows before a blade can touch them. Extended vision and \
             cunningly placed traps turn the terrain against any foe."
        }
        CharClass::Mage => {
            "A scholar of destructive arcana. Mages reshape entire corridors with \
             a single incantation but shatter under one solid blow. Master \
             positioning and distance — or face a very swift end."
        }
        CharClass::Rogue => {
            "Shadow and silence. Rogues deal crippling critical strikes from \
             concealment and vanish before retaliation. Low raw stats are \
             irrelevant — the enemy never gets a turn if the Rogue plays well."
        }
        CharClass::Paladin => "(Coming soon — Paladin is not yet available in this build.)",
        CharClass::Shaman => {
            "A caller of storms and mender of wounds. Shamans split attention \
             between offensive lightning and restorative rituals, providing \
             resilience at the cost of raw burst power."
        }
    }
}

fn class_stats(c: CharClass) -> (i32, i32, i32, i32) {
    // (hp, atk, def, spd)
    match c {
        CharClass::Warrior => (120, 18, 14, 6),
        CharClass::Ranger => (85, 14, 8, 14),
        CharClass::Mage => (60, 22, 4, 10),
        CharClass::Rogue => (75, 16, 6, 16),
        CharClass::Paladin => (0, 0, 0, 0),
        CharClass::Shaman => (95, 12, 10, 9),
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Build menu: five enabled classes, one disabled (Paladin).
    let mut menu: Menu<CharClass> = Menu::new();
    menu.add_item("Warrior", CharClass::Warrior);
    menu.add_item("Ranger", CharClass::Ranger);
    menu.add_item("Mage", CharClass::Mage);
    menu.add_item("Rogue", CharClass::Rogue);
    menu.add_disabled("Paladin  [locked]", CharClass::Paladin);
    menu.add_item("Shaman", CharClass::Shaman);

    // Simulate navigation: 4 × move_down skips the disabled Paladin entry.
    // Warrior(0) → Ranger(1) → Mage(2) → Rogue(3) → Shaman(5) [index 4 skipped]
    menu.move_down();
    menu.move_down();
    menu.move_down();
    menu.move_down();

    let selected = menu.select().expect("non-empty menu has enabled items");
    let (hp, atk, def, spd) = class_stats(selected);
    let desc = class_description(selected);

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);

    screen.clear(Cell {
        glyph: ' ',
        fg: ITEM_FG,
        bg: BG,
    });

    // Row 0: title bar — `center` pads the heading to the full screen width.
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
    let title = center("◈  CHARACTER SELECTION  ◈", 80);
    screen.draw_str(0, 0, &title, TITLE_FG, TITLE_BG);

    // Row 1: column headers with divider.
    screen.draw_str(2, 1, "CLASSES", SUB_FG, BG);
    screen.set(DIV_X, 1, '│', DIV_FG, BG);
    screen.draw_str(RIGHT_X + 2, 1, "DESCRIPTION", SUB_FG, BG);

    // Row 2: top separator with ┬ at the divider column.
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┬' } else { '─' };
        screen.set(x, SEP_TOP_Y, g, DIV_FG, BG);
    }

    // Rows 3-8: menu items.  `pad_right` fills each label to the panel width.
    for (i, item) in menu.iter() {
        let y = MENU_Y + i as i32;
        if y >= SEP_BOT_Y {
            break;
        }
        let is_sel = i == menu.cursor();
        let (row_bg, row_fg) = if is_sel {
            (SEL_BG, SEL_FG)
        } else if item.disabled {
            (BG, DIM_FG)
        } else {
            (BG, ITEM_FG)
        };

        screen.fill_rect(
            0,
            y,
            DIV_X as u32,
            1,
            Cell {
                glyph: ' ',
                fg: row_fg,
                bg: row_bg,
            },
        );

        let cursor_ch = if is_sel { '▶' } else { ' ' };
        let cursor_color = if is_sel { CURSOR_FG } else { row_fg };
        screen.set(1, y, cursor_ch, cursor_color, row_bg);

        // `pad_right` left-aligns the label and pads to fill the panel.
        let label = pad_right(&item.label, (DIV_X - 3) as usize);
        screen.draw_str(3, y, &label, row_fg, row_bg);

        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Remaining divider cells below the menu list.
    for y in (MENU_Y + menu.len() as i32)..SEP_BOT_Y {
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Row 20: bottom separator with ┴ at the divider column.
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, SEP_BOT_Y, g, DIV_FG, BG);
    }

    // Right panel: selected class name — `center` pads it to the panel width.
    let name_line = center(class_name(selected), RIGHT_W);
    screen.draw_str(RIGHT_X, DESC_TITLE_Y, &name_line, CLASS_FG, BG);

    // Right panel: separator under the class name.
    for x in RIGHT_X..(SCREEN_W as i32) {
        screen.set(x, DESC_SEP_Y, '─', DIV_FG, BG);
    }

    // Right panel: description — `wrap_words` breaks at word boundaries.
    let desc_lines = wrap_words(desc, DESC_W);
    for (i, line) in desc_lines.iter().enumerate() {
        let y = DESC_START_Y + i as i32;
        if y >= SEP_BOT_Y {
            break;
        }
        screen.draw_str(DESC_X, y, line, DESC_FG, BG);
    }

    // Row 21: stats — `pad_right` keys left-align; `pad_left` values right-align.
    let stat_pairs: [(&str, i32); 4] = [("HP:", hp), ("ATK:", atk), ("DEF:", def), ("SPD:", spd)];
    let mut sx = 2i32;
    for (label, value) in stat_pairs {
        let key_str = pad_right(label, 4);
        let val_str = pad_left(&value.to_string(), 3);
        screen.draw_str(sx, STATS_Y, &key_str, STAT_KEY, BG);
        sx += key_str.chars().count() as i32;
        screen.draw_str(sx, STATS_Y, &val_str, STAT_VAL, BG);
        sx += val_str.chars().count() as i32 + 3; // 3-char gap between stats
    }

    // Row 22: confirmation line showing cursor position and skipped entry.
    let confirm = format!(
        " ▶  {}  ready   [move_down()×4; Paladin auto-skipped; cursor={}]",
        class_name(selected),
        menu.cursor(),
    );
    screen.draw_str(0, CONFIRM_Y, &confirm, SUB_FG, BG);

    // Row 23: hint bar — `truncate` clips the long hint to fit the screen.
    let full_hint = "[↑↓] Navigate  [Enter] Confirm selection  [Q] Quit  \
                    [?] Help  [R] Reroll stats  [Tab] Compare classes side-by-side";
    let hint = truncate(full_hint, 79);
    screen.draw_str(0, HINT_Y, &hint, HINT_FG, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nMenu + text-layout demo.\n\
         Navigation: Warrior(0) → Ranger(1) → Mage(2) → Rogue(3) → Shaman(5)  \
         [Paladin at index 4 skipped]\n\
         cursor={}  items={}  selected={:?}  desc_lines={}",
        menu.cursor(),
        menu.len(),
        selected,
        desc_lines.len(),
    );
}
