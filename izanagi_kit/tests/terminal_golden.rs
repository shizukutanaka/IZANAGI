use izanagi_kit::content::Color;
use izanagi_kit::terminal::{Cell, Screen};

const FLOOR: Color = Color::rgb(40, 50, 60);
const BACK: Color = Color::rgb(1, 2, 3);
const FRAME: Color = Color::rgb(70, 80, 90);
const PANEL: Color = Color::rgb(4, 5, 6);
const TEXT: Color = Color::rgb(100, 110, 120);
const ACTOR: Color = Color::rgb(200, 210, 220);

fn assert_cells(screen: &Screen, expected: &[&[(char, Color, Color)]]) {
    assert_eq!(screen.height() as usize, expected.len());
    for (y, row) in expected.iter().enumerate() {
        assert_eq!(screen.width() as usize, row.len());
        for (x, &(glyph, fg, bg)) in row.iter().enumerate() {
            assert_eq!(
                screen.get(x as i32, y as i32),
                Some(&Cell { glyph, fg, bg }),
                "cell ({x},{y})"
            );
        }
    }
}

fn composed_screen() -> Screen {
    let mut screen = Screen::new(8, 5);
    screen.clear(Cell {
        glyph: '.',
        fg: FLOOR,
        bg: BACK,
    });
    screen.fill_rect(
        1,
        1,
        5,
        3,
        Cell {
            glyph: ' ',
            fg: TEXT,
            bg: PANEL,
        },
    );
    screen.draw_box(0, 0, 8, 5, FRAME, BACK);
    screen.draw_str(2, 1, "HP:7", TEXT, PANEL);
    screen.draw_line((1, 2), (6, 2), '-', FRAME, PANEL);
    screen.set(4, 2, '@', ACTOR, BACK);
    screen.draw_h_line(1, 3, 5, '=', TEXT, PANEL);
    screen
}

#[test]
fn composed_screen_matches_every_golden_cell() {
    let expected: &[&[(char, Color, Color)]] = &[
        &[
            ('┌', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('┐', FRAME, BACK),
        ],
        &[
            ('│', FRAME, BACK),
            (' ', TEXT, PANEL),
            ('H', TEXT, PANEL),
            ('P', TEXT, PANEL),
            (':', TEXT, PANEL),
            ('7', TEXT, PANEL),
            ('.', FLOOR, BACK),
            ('│', FRAME, BACK),
        ],
        &[
            ('│', FRAME, BACK),
            ('-', FRAME, PANEL),
            ('-', FRAME, PANEL),
            ('-', FRAME, PANEL),
            ('@', ACTOR, BACK),
            ('-', FRAME, PANEL),
            ('-', FRAME, PANEL),
            ('│', FRAME, BACK),
        ],
        &[
            ('│', FRAME, BACK),
            ('=', TEXT, PANEL),
            ('=', TEXT, PANEL),
            ('=', TEXT, PANEL),
            ('=', TEXT, PANEL),
            ('=', TEXT, PANEL),
            ('.', FLOOR, BACK),
            ('│', FRAME, BACK),
        ],
        &[
            ('└', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('─', FRAME, BACK),
            ('┘', FRAME, BACK),
        ],
    ];
    for mut screen in [composed_screen(), composed_screen()] {
        assert_cells(&screen, expected);
        assert_eq!(screen.diff().len(), 40);
        screen.present();
        assert!(screen.diff().is_empty());
        assert_cells(&screen, expected);
    }
}

fn clipped_screen() -> Screen {
    let mut screen = Screen::new(4, 3);
    screen.clear(Cell {
        glyph: '.',
        fg: FLOOR,
        bg: BACK,
    });
    screen.fill_rect(
        -1,
        -1,
        3,
        3,
        Cell {
            glyph: 'x',
            fg: TEXT,
            bg: PANEL,
        },
    );
    screen.draw_str(-1, 0, "AéB", ACTOR, BACK);
    screen.draw_str(3, 0, "!?", FRAME, PANEL);
    screen.draw_line((-2, 1), (1, 1), '-', FRAME, BACK);
    screen.draw_h_line(3, 1, 3, '+', ACTOR, PANEL);
    screen.draw_double_box(2, 2, 3, 3, FRAME, PANEL);
    screen.put(
        -1,
        0,
        Cell {
            glyph: '!',
            fg: ACTOR,
            bg: PANEL,
        },
    );
    screen.set(4, 2, '!', ACTOR, PANEL);
    screen
}

#[test]
fn partially_off_buffer_drawing_matches_every_golden_cell() {
    let expected: &[&[(char, Color, Color)]] = &[
        &[
            ('é', ACTOR, BACK),
            ('B', ACTOR, BACK),
            ('.', FLOOR, BACK),
            ('!', FRAME, PANEL),
        ],
        &[
            ('-', FRAME, BACK),
            ('-', FRAME, BACK),
            ('.', FLOOR, BACK),
            ('+', ACTOR, PANEL),
        ],
        &[
            ('.', FLOOR, BACK),
            ('.', FLOOR, BACK),
            ('╔', FRAME, PANEL),
            ('═', FRAME, PANEL),
        ],
    ];
    for screen in [clipped_screen(), clipped_screen()] {
        assert_cells(&screen, expected);
        assert_eq!(screen.get(-1, 0), None);
        assert_eq!(screen.get(4, 0), None);
        assert_eq!(screen.get(0, -1), None);
        assert_eq!(screen.get(0, 3), None);
    }
}

#[test]
fn empty_buffers_remain_empty_after_drawing() {
    for (width, height) in [(0, 0), (0, 3), (4, 0)] {
        for _ in 0..2 {
            let mut screen = Screen::new(width, height);
            screen.clear(Cell {
                glyph: '.',
                fg: FLOOR,
                bg: BACK,
            });
            screen.fill_rect(
                -1,
                -1,
                3,
                3,
                Cell {
                    glyph: 'x',
                    fg: TEXT,
                    bg: PANEL,
                },
            );
            screen.draw_str(-1, 0, "abc", TEXT, PANEL);
            screen.draw_box(-1, -1, 3, 3, FRAME, BACK);
            screen.draw_double_box(-1, -1, 3, 3, FRAME, BACK);
            screen.draw_line((-1, -1), (1, 1), '-', FRAME, PANEL);
            screen.draw_h_line(-1, 0, 3, '=', TEXT, PANEL);
            screen.set(0, 0, '@', ACTOR, BACK);
            screen.put(
                0,
                0,
                Cell {
                    glyph: '!',
                    fg: ACTOR,
                    bg: PANEL,
                },
            );
            let expected: Vec<&[(char, Color, Color)]> = vec![&[]; height as usize];
            assert_cells(&screen, &expected);
            assert_eq!((screen.width(), screen.height()), (width, height));
            assert_eq!(screen.get(0, 0), None);
            assert_eq!(screen.get(-1, -1), None);
            assert!(screen.diff().is_empty());
            screen.present();
            assert!(screen.diff().is_empty());
        }
    }
}
