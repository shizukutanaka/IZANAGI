//! Geometry and easing curves demo.
//!
//! Left panel: a two-room dungeon with `geometry::line` and `geometry::line_of_sight`.
//! Right panel: sparkline curves for all six easing functions over `Fixed`.
//!
//! Modules exercised:
//! - `geometry::line` — Bresenham line from viewer to each target.
//! - `geometry::line_of_sight` — single-ray LOS through a wall grid.
//! - `Aabb` — two rooms + overlap / intersection / contains_point queries.
//! - `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`,
//!   `ease_out_cubic`, `linear` — Robert Penner easing over `Fixed::from_ratio`.
//! - `Fixed::raw` — convert Q16.16 value to a block-character sparkline level.
//!
//! Run with `cargo run --example geometry_easing_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{
    ease_in_cubic, ease_in_out_quad, ease_in_quad, ease_out_cubic, ease_out_quad, line,
    line_of_sight, linear, Aabb, Cell, Fixed, Screen,
};
use std::io::{self, Write};

// ── screen layout ─────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const MAP_X: i32 = 0; // LOS map top-left on screen
const MAP_Y: i32 = 3;
const MAP_W: i32 = 60;
const MAP_H: i32 = 17;
const DIV_X: i32 = 60;
const EAS_X: i32 = 61; // easing panel start

// ── LOS map: local coord system (0..MAP_W) × (0..MAP_H) ─────────────────────

// Room A: cols 0..20, rows 0..12
const RA: Aabb = Aabb {
    x: 0,
    y: 0,
    w: 20,
    h: 12,
};
// Room B: cols 28..58, rows 2..15
const RB: Aabb = Aabb {
    x: 28,
    y: 2,
    w: 30,
    h: 13,
};
// Corridor opening: rows 5..8, cols 20..28
const CORR_X0: i32 = 20;
const CORR_X1: i32 = 28;
const CORR_Y0: i32 = 5;
const CORR_Y1: i32 = 8;

// Viewer position (Room A interior)
const VIEWER: (i32, i32) = (10, 6);

// 8 targets: first 5 visible, last 3 blocked
const TARGETS: &[(i32, i32)] = &[
    (3, 3),   // Room A interior
    (17, 8),  // Room A interior
    (23, 6),  // corridor
    (40, 6),  // Room B interior via corridor
    (55, 3),  // Room B far right via corridor
    (40, 1),  // void above Room B → blocked
    (50, 13), // line exits corridor outside opening → blocked
    (2, 13),  // below Room A floor → blocked
];

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 6, b: 16 };
const TITLE_BG: Color = Color {
    r: 28,
    g: 14,
    b: 58,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 200,
    b: 255,
};
const WALL_FG: Color = Color {
    r: 100,
    g: 88,
    b: 128,
};
const FLOOR_FG: Color = Color {
    r: 38,
    g: 34,
    b: 56,
};
const FLOOR_BG: Color = Color {
    r: 12,
    g: 10,
    b: 22,
};
const VIS_FG: Color = Color {
    r: 80,
    g: 200,
    b: 80,
};
const BLOCK_FG: Color = Color {
    r: 200,
    g: 70,
    b: 70,
};
const VIEWER_FG: Color = Color {
    r: 255,
    g: 220,
    b: 60,
};
const TARGET_FG: Color = Color {
    r: 255,
    g: 160,
    b: 60,
};
const DIV_FG: Color = Color {
    r: 55,
    g: 40,
    b: 92,
};
const EAS_HDR: Color = Color {
    r: 120,
    g: 110,
    b: 170,
};
const EAS_FG: Color = Color {
    r: 150,
    g: 200,
    b: 255,
};
const EAS_SPARK: Color = Color {
    r: 80,
    g: 180,
    b: 255,
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

// Type alias to avoid clippy::type_complexity on the easing function slice.
type EasingFn = fn(Fixed) -> Fixed;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Is `(lx, ly)` an opaque wall cell in the LOS map?
fn is_opaque(lx: i32, ly: i32) -> bool {
    let in_a = (RA.x..RA.x + RA.w).contains(&lx) && (RA.y..RA.y + RA.h).contains(&ly);
    let in_b = (RB.x..RB.x + RB.w).contains(&lx) && (RB.y..RB.y + RB.h).contains(&ly);
    let in_corr = (CORR_X0..CORR_X1).contains(&lx) && (CORR_Y0..CORR_Y1).contains(&ly);

    if in_corr {
        return false; // corridor = passable
    }
    if in_a {
        // Border cells of Room A are walls; right side has an opening for the corridor.
        let on_top = ly == RA.y;
        let on_bot = ly == RA.y + RA.h - 1;
        let on_left = lx == RA.x;
        let on_right = lx == RA.x + RA.w - 1 && !(CORR_Y0..CORR_Y1).contains(&ly);
        return on_top || on_bot || on_left || on_right;
    }
    if in_b {
        // Border of Room B; left side has an opening for the corridor.
        let on_top = ly == RB.y;
        let on_bot = ly == RB.y + RB.h - 1;
        let on_right = lx == RB.x + RB.w - 1;
        let on_left = lx == RB.x && !(CORR_Y0..CORR_Y1).contains(&ly);
        return on_top || on_bot || on_right || on_left;
    }
    true // void = opaque
}

/// Build an 8-character sparkline for `f(t)` at t=1/8..8/8.
fn sparkline(f: impl Fn(Fixed) -> Fixed) -> String {
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let mut s = String::new();
    for step in 1..=8u32 {
        let t = Fixed::from_ratio(step as i32, 8);
        let v = f(t);
        let raw = v.raw().clamp(0, 65536) as u64;
        let level = ((raw * 8) / 65536).min(8) as usize;
        s.push(BLOCKS[level]);
    }
    s
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: STAT_FG,
        bg: BG,
    });

    // ── title bar ─────────────────────────────────────────────────────────────
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
        " GEOMETRY & EASING  line / line_of_sight / Aabb / Fixed easing",
        TITLE_FG,
        TITLE_BG,
    );

    // Row 1: column headers.
    screen.draw_str(1, 1, "LOS MAP (8 targets)", DIV_FG, BG);
    screen.set(DIV_X, 1, '│', DIV_FG, BG);
    screen.draw_str(EAS_X, 1, "EASING f(t)", EAS_HDR, BG);

    // Row 2: top separator.
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┬' } else { '─' };
        screen.set(x, 2, g, DIV_FG, BG);
    }

    // ── draw LOS map base (rooms + corridor) ──────────────────────────────────
    for ly in 0..MAP_H {
        for lx in 0..MAP_W {
            let sx = MAP_X + lx;
            let sy = MAP_Y + ly;

            let in_a = (RA.x..RA.x + RA.w).contains(&lx) && (RA.y..RA.y + RA.h).contains(&ly);
            let in_b = (RB.x..RB.x + RB.w).contains(&lx) && (RB.y..RB.y + RB.h).contains(&ly);
            let in_corr = (CORR_X0..CORR_X1).contains(&lx) && (CORR_Y0..CORR_Y1).contains(&ly);

            if in_a || in_b || in_corr {
                let wall = is_opaque(lx, ly);
                let (glyph, fg, bg) = if wall {
                    ('#', WALL_FG, BG)
                } else {
                    ('.', FLOOR_FG, FLOOR_BG)
                };
                screen.set(sx, sy, glyph, fg, bg);
            }
        }
    }

    // ── draw LOS rays ─────────────────────────────────────────────────────────
    let mut n_visible = 0u32;
    let mut n_blocked = 0u32;
    let line_len_sample = line(VIEWER, TARGETS[3]).len();

    for &target in TARGETS {
        let can_see = line_of_sight(VIEWER, target, is_opaque);
        if can_see {
            n_visible += 1;
        } else {
            n_blocked += 1;
        }
        let ray_color = if can_see { VIS_FG } else { BLOCK_FG };

        let cells = line(VIEWER, target);
        for &(lx, ly) in cells.iter().skip(1) {
            let sx = MAP_X + lx;
            let sy = MAP_Y + ly;
            if (0..MAP_W).contains(&lx) && (0..MAP_H).contains(&ly) && (lx, ly) != target {
                let wall = is_opaque(lx, ly);
                let g = if wall { '█' } else { '·' };
                screen.set(sx, sy, g, ray_color, BG);
            }
        }
        // Mark target with 'T'.
        let (tx, ty) = target;
        if (0..MAP_W).contains(&tx) && (0..MAP_H).contains(&ty) {
            screen.set(MAP_X + tx, MAP_Y + ty, 'T', TARGET_FG, BG);
        }
    }

    // Mark viewer '@'.
    screen.set(MAP_X + VIEWER.0, MAP_Y + VIEWER.1, '@', VIEWER_FG, FLOOR_BG);

    // ── divider column ────────────────────────────────────────────────────────
    for y in 1..21 {
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // ── easing panel ─────────────────────────────────────────────────────────
    screen.draw_str(EAS_X, 3, "name     t→1", EAS_HDR, BG);
    screen.draw_str(EAS_X, 4, "─────────────────", DIV_FG, BG);

    let easing_fns: &[(&str, EasingFn)] = &[
        ("linear  ", linear),
        ("in-quad ", ease_in_quad),
        ("out-quad", ease_out_quad),
        ("io-quad ", ease_in_out_quad),
        ("in-cub  ", ease_in_cubic),
        ("out-cub ", ease_out_cubic),
    ];
    for (i, (label, f)) in easing_fns.iter().enumerate() {
        let y = 5 + i as i32;
        screen.draw_str(EAS_X, y, label, EAS_FG, BG);
        let spark = sparkline(*f);
        screen.draw_str(EAS_X + 9, y, &spark, EAS_SPARK, BG);
    }
    screen.draw_str(EAS_X, 12, "Fixed(Q16.16)", EAS_HDR, BG);
    screen.draw_str(EAS_X, 13, "no float.", EAS_HDR, BG);

    // ── bottom separator ──────────────────────────────────────────────────────
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, MAP_Y + MAP_H, g, DIV_FG, BG);
    }

    // ── Aabb stats row ────────────────────────────────────────────────────────
    let bc: Aabb = Aabb::new(32, 4, 16, 6); // test box C inside Room B
    let bd: Aabb = Aabb::new(40, 7, 15, 6); // test box D overlapping C
    let ab_overlap = RA.overlaps(&RB);
    let cd_overlap = bc.overlaps(&bd);
    let viewer_in_a = RA.contains_point(VIEWER.0, VIEWER.1);
    let cd_inter = bc.intersection(&bd);

    let inter_str = if let Some(r) = cd_inter {
        format!("({},{},{},{})", r.x, r.y, r.w, r.h)
    } else {
        "None".to_string()
    };
    let aabb_line = format!(
        " A∩B={}  C∩D={}  C.inter.D={}  viewer∈A={}",
        ab_overlap, cd_overlap, inter_str, viewer_in_a,
    );
    screen.draw_str(0, MAP_Y + MAP_H + 1, &aabb_line, STAT_FG, BG);

    // ── LOS stats row ─────────────────────────────────────────────────────────
    let los_line = format!(
        " viewer=({},{})  targets={}  visible={}  blocked={}",
        VIEWER.0,
        VIEWER.1,
        TARGETS.len(),
        n_visible,
        n_blocked,
    );
    screen.draw_str(0, MAP_Y + MAP_H + 2, &los_line, STAT_FG, BG);

    // ── line info row ─────────────────────────────────────────────────────────
    let line_line = format!(
        " line({VIEWER:?},{:?})={} cells  │  easing: 6 fns × 8 Fixed steps",
        TARGETS[3], line_len_sample,
    );
    screen.draw_str(0, MAP_Y + MAP_H + 3, &line_line, STAT_HI, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nGeometry + easing demo.\n\
         LOS: viewer=({},{})  visible={}/{}  blocked={}/{}\n\
         Aabb: A overlaps B = {}  C overlaps D = {}  intersection = {}\n\
         line({:?},{:?}) = {} cells",
        VIEWER.0,
        VIEWER.1,
        n_visible,
        TARGETS.len(),
        n_blocked,
        TARGETS.len(),
        ab_overlap,
        cd_overlap,
        inter_str,
        VIEWER,
        TARGETS[3],
        line_len_sample,
    );
}
