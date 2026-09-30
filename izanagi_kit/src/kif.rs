//! KIF 棋譜(UTF-8 形) — `開始日時:`/`終了日時:`/`手合割:`/`先手:`/`後手:` ヘッダ、
//! `N 歩(77)` 型の手番行、`*` コメント、`手数----` トレーラの集計。
//!
//! ```
//! let d = "開始日時: 2024/01/01\n手合割: 平手\n先手: sente\n後手: gote\n手数----指手---------消費時間--\n   1 ７六歩(77)\n   2 ３四歩(33)\n   3 投了\nまで2手で先手の勝ち\n";
//! let k = izanagi_kit::kif::parse(d.as_bytes()).unwrap();
//! assert_eq!(k.headers, 4);
//! assert_eq!(k.moves, 3);
//! assert_eq!(k.result_moves, 1);
//! assert!(izanagi_kit::kif::detect(d.as_bytes()));
//! ```

/// A parsed KIF kifu census.
#[derive(Debug, Clone)]
pub struct Kif {
    /// `key:`-style header lines (開始日時/終了日時/手合割/先手/後手/棋戦/場所/持時間/…).
    pub headers: usize,
    /// Numbered move lines (`  12 …`).
    pub moves: usize,
    /// Moves ending in a terminal word (投了/中断/持将棋/千日手/反則勝ち/切れ負け).
    pub result_moves: usize,
    /// Lines containing `(77)`-style origin coordinates.
    pub coord_moves: usize,
    /// Moves whose destination is 打 (drop) or 成/不成 promotion.
    pub special_moves: usize,
    /// `*` comment lines inside the move list.
    pub comments: usize,
    /// `までN手で…` result trailer lines.
    pub result_lines: usize,
    /// `手数----` separator lines.
    pub separators: usize,
    /// `変化:` branch markers.
    pub variations: usize,
}

const HEADERS: &[&str] = &[
    "開始日時",
    "終了日時",
    "表題",
    "棋戦",
    "場所",
    "持時間",
    "消費時間",
    "手合割",
    "先手",
    "後手",
    "先手省略",
    "後手省略",
    "戦型",
    "作品名",
    "作者",
    "発表誌",
];
const RESULT_WORDS: &[&str] = &[
    "投了",
    "中断",
    "持将棋",
    "千日手",
    "反則勝",
    "切れ負け",
    "宣言勝",
];

fn is_header(l: &str) -> bool {
    HEADERS.iter().any(|h| l.starts_with(h))
}

fn is_move_line(l: &str) -> bool {
    let t = l.trim_start();
    let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
    !digits.is_empty() && t[digits.len()..].starts_with(char::is_whitespace)
}

/// Detects a KIF kifu: KIF headers or a `手数----` separator or numbered moves with `(NN)` origins.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut heads = 0;
    let mut sep = 0;
    let mut moves = 0;
    for line in t.lines() {
        let l = line.trim_end();
        if is_header(l) {
            heads += 1;
        } else if l.contains("手数") && l.contains('-') {
            sep += 1;
        } else if is_move_line(l) && (l.contains('(') || RESULT_WORDS.iter().any(|w| l.contains(w)))
        {
            moves += 1;
        }
    }
    heads + sep + moves > 0
}

/// Parses a KIF kifu; `None` without KIF lines.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Kif> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut k = Kif {
        headers: 0,
        moves: 0,
        result_moves: 0,
        coord_moves: 0,
        special_moves: 0,
        comments: 0,
        result_lines: 0,
        separators: 0,
        variations: 0,
    };
    for line in t.lines() {
        let l = line.trim_end();
        if l.is_empty() {
            continue;
        }
        if is_header(l) {
            k.headers += 1;
        } else if l.contains("手数") && l.contains('-') {
            k.separators += 1;
        } else if l.starts_with('*') {
            k.comments += 1;
        } else if l.starts_with("変化") {
            k.variations += 1;
        } else if l.starts_with("まで") && l.contains('手') {
            k.result_lines += 1;
        } else if is_move_line(l) {
            k.moves += 1;
            if RESULT_WORDS.iter().any(|w| l.contains(w)) {
                k.result_moves += 1;
            }
            if l.contains('(') && l.contains(')') {
                k.coord_moves += 1;
            }
            if l.contains("打") || l.contains("成") {
                k.special_moves += 1;
            }
        }
    }
    Some(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        "開始日時: 2024/01/01\n終了日時: 2024/01/01\n棋戦: 竜王戦\n場所: 東京\n手合割: 平手\n先手: sente\n後手: gote\n手数----指手---------消費時間--\n   1 ７六歩(77)\n   2 ３四歩(33)\n   3 ２二角成(88)\n   4 同　銀(31)\n   5 ５五角打\n   6 投了\nまで5手で先手の勝ち\n".to_string()
    }

    #[test]
    fn parses() {
        let d = fixture();
        let k = parse(d.as_bytes()).unwrap();
        assert_eq!(k.headers, 7);
        assert_eq!(k.moves, 6);
        assert_eq!(k.result_moves, 1);
        assert_eq!(k.coord_moves, 4);
        assert_eq!(k.special_moves, 2);
        assert_eq!(k.separators, 1);
        assert_eq!(k.result_lines, 1);
    }

    #[test]
    fn detect_works() {
        let d = fixture();
        assert!(detect(d.as_bytes()));
        assert!(detect("先手: a\n後手: b\n".as_bytes()));
        assert!(!detect(b"name: x\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}
