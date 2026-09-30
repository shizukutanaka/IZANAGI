//! 将棋 CSA 棋譜 — `N+`/`N-` 対局者、`P1`–`P9`/`P+`/`P-`/`PI` 盤面、
//! `+7776FU` 型の手、`%` コマンド、`T` 消費時間、`'` コメントの集計。
//!
//! ```
//! let d = b"N+sente\nN-gote\nP1-KY-KE-GI-KI-OU-KI-GI-KE-KY\nP2 * -HI *  *  *  *  * -KA * \nP3-FU-FU-FU-FU-FU-FU-FU-FU-FU\n+7776FU\n-3334FU\n%CHUDAN\n";
//! let c = izanagi_kit::csa::parse(d).unwrap();
//! assert_eq!(c.players, 2);
//! assert_eq!(c.moves, 2);
//! assert_eq!(c.board_lines, 3);
//! assert!(izanagi_kit::csa::detect(d));
//! ```

/// A parsed CSA kifu census.
#[derive(Debug, Clone)]
pub struct Csa {
    /// `N+`/`N-` player lines.
    pub players: usize,
    /// `P1`–`P9` board rows.
    pub board_lines: usize,
    /// `P+`/`P-`/`PI` alternate board-setup lines.
    pub setup_lines: usize,
    /// `+`/` -` move records (excluding `%` commands).
    pub moves: usize,
    /// Sente (`+`) moves.
    pub sente_moves: usize,
    /// `%` commands (CHUDAN/TORYO/SENNICHITE/…).
    pub commands: usize,
    /// `T` time-usage lines.
    pub time_lines: usize,
    /// `'` comment lines.
    pub comments: usize,
    /// `V`/`$`/`!`/`?`/`&` auxiliary directives.
    pub other_lines: usize,
    /// Promoted-piece destinations (`+7776TO`/`+7776RY` style suffix).
    pub promotes: usize,
    /// Drop moves (`+00KA`, `*00FU`-style source `00`).
    pub drops: usize,
}

fn is_move_line(l: &str) -> bool {
    let b = l.as_bytes();
    b.len() >= 7
        && (b[0] == b'+' || b[0] == b'-')
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4].is_ascii_digit()
        && b[5].is_ascii_uppercase()
        && b[6].is_ascii_uppercase()
}

/// Detects a CSA kifu: `N+`/`N-` player lines or `P1`…`P9` rows or `+`/` -` moves.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut players = 0;
    let mut board = 0;
    let mut moves = 0;
    for line in t.lines() {
        let l = line.trim_end();
        if l.starts_with("N+") || l.starts_with("N-") {
            players += 1;
        } else if l.len() >= 3 && l.as_bytes()[0] == b'P' && l.as_bytes()[1].is_ascii_digit() {
            board += 1;
        } else if is_move_line(l) {
            moves += 1;
        }
    }
    players > 0 || board > 0 || moves > 0
}

/// Parses a CSA kifu; `None` without CSA lines.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Csa> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut c = Csa {
        players: 0,
        board_lines: 0,
        setup_lines: 0,
        moves: 0,
        sente_moves: 0,
        commands: 0,
        time_lines: 0,
        comments: 0,
        other_lines: 0,
        promotes: 0,
        drops: 0,
    };
    for line in t.lines() {
        let l = line.trim_end();
        if l.is_empty() {
            continue;
        }
        match l.as_bytes()[0] {
            b'N' => c.players += 1,
            b'P' if l.len() >= 2 && l.as_bytes()[1].is_ascii_digit() => c.board_lines += 1,
            b'P' if l.len() >= 2
                && (l.as_bytes()[1] == b'+'
                    || l.as_bytes()[1] == b'-'
                    || l.as_bytes()[1] == b'I') =>
            {
                c.setup_lines += 1
            }
            b'%' => c.commands += 1,
            b'T' => c.time_lines += 1,
            b'\'' => c.comments += 1,
            b'V' | b'$' | b'!' | b'?' | b'&' => c.other_lines += 1,
            b'+' | b'-' if is_move_line(l) => {
                c.moves += 1;
                if l.as_bytes()[0] == b'+' {
                    c.sente_moves += 1;
                }
                let piece = &l[5..7];
                if matches!(piece, "TO" | "NY" | "NK" | "NG" | "UM" | "RY") {
                    c.promotes += 1;
                }
                if &l[1..3] == "00" {
                    c.drops += 1;
                }
            }
            _ => c.other_lines += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"N+sente\nN-gote\nP1-KY-KE-GI-KI-OU-KI-GI-KE-KY\nP2 * -HI *  *  *  *  * -KA * \nP3-FU-FU-FU-FU-FU-FU-FU-FU-FU\nP4 *  *  *  *  *  *  *  *  * \nP5 *  *  *  *  *  *  *  *  * \nP6 *  *  *  *  *  *  *  *  * \nP7+FU+FU+FU+FU+FU+FU+FU+FU+FU\nP8 * +KA *  *  *  *  * +HI * \nP9+KY+KE+GI+KI+OU+KI+GI+KE+KY\n+7776FU\nT10\n-3334FU\nT5\n+8822UM\n-0077KA\n'comment\n%CHUDAN\n";

    #[test]
    fn parses() {
        let c = parse(D).unwrap();
        assert_eq!(c.players, 2);
        assert_eq!(c.board_lines, 9);
        assert_eq!(c.moves, 4);
        assert_eq!(c.sente_moves, 2);
        assert_eq!(c.commands, 1);
        assert_eq!(c.time_lines, 2);
        assert_eq!(c.comments, 1);
        assert_eq!(c.promotes, 1);
        assert_eq!(c.drops, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"N+a\nN-b\n"));
        assert!(detect(b"+7776FU\n"));
        assert!(!detect(b"name: x\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}
