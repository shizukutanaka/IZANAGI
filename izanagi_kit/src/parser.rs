//! Line-based content parser. Zero-dependency, panic-free, bounded.
//!
//! Grammar (keyword-led lines; `//` comments and blank lines ignored):
//!
//! ```text
//!   prefab <name>
//!     glyph <char>
//!     color <#RRGGBB>
//!     stat  <key> <int>
//!     flag  <name>
//!   tile  <name> <glyph> <#RRGGBB>
//!   level <name> <W>x<H>
//!     row   <cells...>          (exactly H of these, each W glyphs wide)
//!     spawn <prefab> <x> <y>
//! ```
//!
//! Block keywords (`prefab`, `level`) open a context; the indented child lines
//! attach to the most recent open block. Malformed lines become diagnostics
//! rather than aborting, so a single typo never hides the rest of the errors.
//!
//! A formal EBNF grammar with the exact lexical/boundary rules (line length,
//! dimension bounds, color/glyph/int/uint token shapes) lives in `SPEC.md`
//! §9.1 and is kept in 1:1 correspondence with this module.

use crate::content::{parse_color, Content, Diagnostic, Level, Prefab, Spawn, Tile};

/// Hard limits guard against malicious/huge inputs (DoS / OOM defense).
const MAX_LINE_LEN: usize = 1024;
const MAX_NAME_LEN: usize = 32;
const MAX_DIM: u32 = 256;

enum Block {
    None,
    Prefab(usize),
    Level(usize),
}

/// A token plus its 1-based column in the original (untrimmed) line, so
/// diagnostics can render a caret under the exact offending token.
type Token<'a> = (&'a str, usize);

/// Splits a line into whitespace-separated tokens, tracking each token's
/// 1-based starting column. Columns count chars (not bytes) so the caret aligns
/// with what the author sees.
fn tokenize(raw: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut start_byte = None;
    let mut start_col = 0usize;
    let mut col = 0usize; // 1-based char column of the current char
    for (byte_idx, ch) in raw.char_indices() {
        col += 1;
        if ch.is_whitespace() {
            if let Some(sb) = start_byte.take() {
                tokens.push((&raw[sb..byte_idx], start_col));
            }
        } else if start_byte.is_none() {
            start_byte = Some(byte_idx);
            start_col = col;
        }
    }
    if let Some(sb) = start_byte {
        tokens.push((&raw[sb..], start_col));
    }
    tokens
}

/// Parses source text into [`Content`] plus structural diagnostics. Never
/// panics on any input.
pub fn parse(source: &str) -> (Content, Vec<Diagnostic>) {
    let mut content = Content::default();
    let mut diags = Vec::new();
    let mut block = Block::None;

    for (i, raw) in source.lines().enumerate() {
        let line_no = i + 1;

        if raw.len() > MAX_LINE_LEN {
            diags.push(Diagnostic::error(line_no, "line exceeds 1024 bytes"));
            continue;
        }
        let trimmed = raw.trim_start();
        if trimmed.trim_end().is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let tokens = tokenize(raw);
        let (keyword, kw_col) = match tokens.first() {
            Some(&t) => t,
            None => continue,
        };
        // End-of-line column for "missing argument" carets.
        let eol_col = raw.chars().count() + 1;
        let args: Vec<Token> = tokens[1..].to_vec();
        let arg_col = |n: usize| args.get(n).map(|&(_, c)| c).unwrap_or(eol_col);
        let arg_str = |n: usize| args.get(n).map(|&(s, _)| s);

        match keyword {
            "prefab" => match arg_str(0) {
                Some(name) if name.len() <= MAX_NAME_LEN => {
                    content.prefabs.push(Prefab::new(name.to_string()));
                    block = Block::Prefab(content.prefabs.len() - 1);
                }
                Some(_) => diags.push(Diagnostic::error_at(
                    line_no,
                    arg_col(0),
                    "prefab name too long",
                )),
                None => diags.push(Diagnostic::error_at(
                    line_no,
                    eol_col,
                    "prefab needs a name",
                )),
            },

            "tile" => {
                if args.len() == 3 {
                    parse_tile(line_no, &args, &mut content, &mut diags);
                } else {
                    diags.push(Diagnostic::error_at(
                        line_no,
                        kw_col,
                        "tile needs: <name> <glyph> <#hex>",
                    ));
                }
                block = Block::None;
            }

            "level" => {
                // Bind both arguments once. The previous form re-derived them
                // with `unwrap` behind a length guard several lines away, which
                // was correct but relied on the reader checking the guard.
                let named = arg_str(0).filter(|n| n.len() <= MAX_NAME_LEN);
                match (args.len(), named, arg_str(1)) {
                    (2, Some(name), Some(dim)) => match parse_dim(dim) {
                        Ok((w, h)) => {
                            content.levels.push(Level {
                                name: name.to_string(),
                                width: w,
                                height: h,
                                rows: Vec::new(),
                                spawns: Vec::new(),
                            });
                            block = Block::Level(content.levels.len() - 1);
                        }
                        Err(e) => {
                            diags.push(Diagnostic::error_at(line_no, arg_col(1), e));
                            block = Block::None;
                        }
                    },
                    _ => {
                        diags.push(Diagnostic::error_at(
                            line_no,
                            kw_col,
                            "level needs: <name> <W>x<H>",
                        ));
                        block = Block::None;
                    }
                }
            }

            "glyph" | "color" | "stat" | "flag" => {
                if let Block::Prefab(idx) = block {
                    apply_prefab_attr(
                        line_no,
                        keyword,
                        kw_col,
                        &args,
                        &mut content.prefabs[idx],
                        &mut diags,
                    );
                } else {
                    diags.push(Diagnostic::error_at(
                        line_no,
                        kw_col,
                        format!("'{keyword}' outside a prefab block"),
                    ));
                }
            }

            "row" | "spawn" => {
                if let Block::Level(idx) = block {
                    apply_level_attr(
                        line_no,
                        keyword,
                        kw_col,
                        &args,
                        &mut content.levels[idx],
                        &mut diags,
                    );
                } else {
                    diags.push(Diagnostic::error_at(
                        line_no,
                        kw_col,
                        format!("'{keyword}' outside a level block"),
                    ));
                }
            }

            other => {
                diags.push(Diagnostic::warning_at(
                    line_no,
                    kw_col,
                    format!("unknown keyword '{other}' (line ignored)"),
                ));
            }
        }
    }

    (content, diags)
}

/// Count error-severity diagnostics in a slice. Convenience for CI pipelines
/// and tool integrations that need a quick pass/fail tally without iterating.
pub fn error_count(diags: &[Diagnostic]) -> usize {
    diags.iter().filter(|d| d.is_error()).count()
}

/// Count warning-severity diagnostics in a slice. The complement of
/// [`error_count`]; together they account for all diagnostics.
pub fn warning_count(diags: &[Diagnostic]) -> usize {
    diags.iter().filter(|d| !d.is_error()).count()
}

fn parse_dim(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .split_once('x')
        .ok_or_else(|| format!("dimension must be WxH, got {s:?}"))?;
    let w: u32 = w.parse().map_err(|_| format!("bad width in {s:?}"))?;
    let h: u32 = h.parse().map_err(|_| format!("bad height in {s:?}"))?;
    if w == 0 || h == 0 || w > MAX_DIM || h > MAX_DIM {
        return Err(format!("dimension out of range 1..={MAX_DIM}: {s:?}"));
    }
    Ok((w, h))
}

fn single_char(s: &str) -> Option<char> {
    let mut it = s.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

fn parse_tile(line: usize, args: &[Token], content: &mut Content, diags: &mut Vec<Diagnostic>) {
    let (name, _) = args[0];
    let (glyph, glyph_col) = args[1];
    let (color, color_col) = args[2];
    let g = match single_char(glyph) {
        Some(c) => c,
        None => {
            diags.push(Diagnostic::error_at(
                line,
                glyph_col,
                "tile glyph must be exactly one character",
            ));
            return;
        }
    };
    let c = match parse_color(color) {
        Ok(c) => c,
        Err(e) => {
            diags.push(Diagnostic::error_at(line, color_col, e));
            return;
        }
    };
    content.tiles.push(Tile {
        name: name.to_string(),
        glyph: g,
        color: c,
    });
}

fn apply_prefab_attr(
    line: usize,
    kw: &str,
    kw_col: usize,
    args: &[Token],
    p: &mut Prefab,
    diags: &mut Vec<Diagnostic>,
) {
    match (kw, args) {
        ("glyph", [(g, gc)]) => match single_char(g) {
            Some(c) => p.glyph = c,
            None => diags.push(Diagnostic::error_at(
                line,
                *gc,
                "glyph must be one character",
            )),
        },
        ("color", [(c, cc)]) => match parse_color(c) {
            Ok(col) => p.color = col,
            Err(e) => diags.push(Diagnostic::error_at(line, *cc, e)),
        },
        ("stat", [(k, _), (v, vc)]) => match v.parse::<i32>() {
            Ok(n) => {
                p.stats.insert((*k).to_string(), n);
            }
            Err(_) => diags.push(Diagnostic::error_at(
                line,
                *vc,
                format!("stat '{k}' value not an i32: {v:?}"),
            )),
        },
        ("flag", [(f, _)]) => p.flags.push((*f).to_string()),
        _ => diags.push(Diagnostic::error_at(
            line,
            kw_col,
            format!("malformed '{kw}'"),
        )),
    }
}

fn apply_level_attr(
    line: usize,
    kw: &str,
    kw_col: usize,
    args: &[Token],
    lv: &mut Level,
    diags: &mut Vec<Diagnostic>,
) {
    match kw {
        "row" => match args {
            [(cells, _)] => lv.rows.push((*cells).to_string()),
            _ => diags.push(Diagnostic::error_at(
                line,
                kw_col,
                "row needs exactly one cell string (no spaces)",
            )),
        },
        "spawn" => match args {
            [(prefab, _), (x, xc), (y, yc)] => match (x.parse::<u32>(), y.parse::<u32>()) {
                (Ok(x), Ok(y)) => lv.spawns.push(Spawn {
                    prefab: (*prefab).to_string(),
                    x,
                    y,
                }),
                (Err(_), _) => diags.push(Diagnostic::error_at(
                    line,
                    *xc,
                    "spawn x must be a non-negative integer",
                )),
                (_, Err(_)) => diags.push(Diagnostic::error_at(
                    line,
                    *yc,
                    "spawn y must be a non-negative integer",
                )),
            },
            _ => diags.push(Diagnostic::error_at(
                line,
                kw_col,
                "spawn needs: <prefab> <x> <y>",
            )),
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
// demo
prefab goblin
  glyph g
  color #f85149
  stat hp 10
  stat atk 3
  flag hostile
tile floor . #3A3A3A
level cave 5x3
  row #####
  row #...#
  row #####
  spawn goblin 2 1
";

    #[test]
    fn test_parse_counts() {
        let (c, d) = parse(SAMPLE);
        assert!(d.iter().all(|x| !x.is_error()), "no errors expected: {d:?}");
        assert_eq!(c.prefabs.len(), 1);
        assert_eq!(c.tiles.len(), 1);
        assert_eq!(c.levels.len(), 1);
    }

    #[test]
    fn test_parse_prefab_fields() {
        let (c, _) = parse(SAMPLE);
        let g = c.prefab("goblin").unwrap();
        assert_eq!(g.glyph, 'g');
        assert_eq!(g.stats.get("hp"), Some(&10));
        assert!(g.flags.contains(&"hostile".to_string()));
    }

    #[test]
    fn test_parse_level_grid_and_spawn() {
        let (c, _) = parse(SAMPLE);
        let lv = c.level("cave").unwrap();
        assert_eq!((lv.width, lv.height), (5, 3));
        assert_eq!(lv.rows.len(), 3);
        assert_eq!(lv.spawns.len(), 1);
        assert_eq!(lv.spawns[0].prefab, "goblin");
    }

    #[test]
    fn test_attr_outside_block_is_error() {
        let (_, d) = parse("glyph x");
        assert!(d.iter().any(|x| x.is_error()));
    }

    #[test]
    fn test_unknown_keyword_is_warning_not_error() {
        let (_, d) = parse("wobble foo");
        assert!(d
            .iter()
            .any(|x| !x.is_error() && x.message.contains("unknown")));
    }

    #[test]
    fn test_bad_dimension_rejected() {
        let (_, d) = parse("level big 9999x9999");
        assert!(d.iter().any(|x| x.is_error()));
    }

    #[test]
    fn test_tokenize_tracks_columns() {
        // Columns are 1-based and count the leading indent.
        let toks = tokenize("  glyph @@");
        assert_eq!(toks, vec![("glyph", 3), ("@@", 9)]);
    }

    #[test]
    fn test_tokenize_handles_trailing_and_multiple_spaces() {
        let toks = tokenize("spawn   goblin  2 1  ");
        assert_eq!(
            toks,
            vec![("spawn", 1), ("goblin", 9), ("2", 17), ("1", 19)]
        );
    }

    #[test]
    fn test_error_carries_column_of_offending_token() {
        // "  glyph @@" — the bad token @@ starts at column 9.
        let (_, d) = parse("prefab a\n  glyph @@\n");
        let err = d.iter().find(|x| x.is_error()).expect("expected an error");
        assert_eq!(err.line, 2);
        assert_eq!(err.col, 9, "caret must point at the @@ token");
    }

    #[test]
    fn test_bad_color_column_points_at_color() {
        // "  color #ZZ0000" — color token at column 9.
        let (_, d) = parse("prefab a\n  color #ZZ0000\n");
        let err = d.iter().find(|x| x.is_error()).expect("expected an error");
        assert_eq!(err.col, 9);
    }

    #[test]
    fn test_render_produces_caret_under_token() {
        let src = "prefab a\n  glyph @@\n";
        let (_, d) = parse(src);
        let err = d.iter().find(|x| x.is_error()).unwrap();
        let rendered = err.render("x.game", src);
        // The caret line should have the ^ at column 9 (8 spaces then ^).
        assert!(rendered.contains("\n        ^"), "rendered:\n{rendered}");
    }

    #[test]
    fn test_missing_arg_points_at_end_of_line() {
        let (_, d) = parse("prefab");
        let err = d.iter().find(|x| x.is_error()).unwrap();
        assert_eq!(err.col, 7, "caret at end-of-line for missing name");
    }

    #[test]
    fn test_error_count_empty_slice_is_zero() {
        assert_eq!(error_count(&[]), 0);
    }

    #[test]
    fn test_error_count_counts_only_errors() {
        // "wobble" gives a warning; "prefab" with no name gives an error.
        let (_, d) = parse("wobble foo\nprefab");
        assert_eq!(error_count(&d), 1);
    }

    #[test]
    fn test_error_count_all_warnings_is_zero() {
        let (_, d) = parse("wobble a\nwobble b");
        assert!(d.iter().all(|x| !x.is_error()));
        assert_eq!(error_count(&d), 0);
    }

    #[test]
    fn test_warning_count_empty_slice_is_zero() {
        assert_eq!(warning_count(&[]), 0);
    }

    #[test]
    fn test_warning_count_counts_only_warnings() {
        // "wobble" gives a warning; "prefab" with no name gives an error.
        let (_, d) = parse("wobble foo\nprefab");
        assert_eq!(warning_count(&d), 1);
        assert_eq!(error_count(&d), 1);
    }

    #[test]
    fn test_warning_count_all_errors_is_zero() {
        let (_, d) = parse("prefab\nprefab");
        assert!(d.iter().all(|x| x.is_error()));
        assert_eq!(warning_count(&d), 0);
    }
}
