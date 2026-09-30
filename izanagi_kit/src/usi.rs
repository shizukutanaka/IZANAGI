//! USI(Universal Shogi Interface) プロトコル — `usi`/`usiok`/`isready`/`readyok`
//! ハンドシェイク、`position`/`go`/`info`/`bestmove` コマンドと `info` フィールド集計。
//!
//! ```
//! let d = b"usi\nid name demo\nid author jane\noption name Threads type spin default 1 min 1 max 256\nusiok\nisready\nreadyok\nposition startpos moves 7g7f 3c3d\ngo btime 300000 wtime 300000 binc 0\ninfo depth 12 seldepth 18 score cp 34 nodes 12345 pv 7g7f 3c3d\nbestmove 7g7f ponder 3c3d\n";
//! let u = izanagi_kit::usi::parse(d).unwrap();
//! assert_eq!(u.info_fields, 6);
//! assert_eq!(u.position_moves, 2);
//! assert_eq!(u.bestmove, "7g7f");
//! assert!(izanagi_kit::usi::detect(d));
//! ```

/// A parsed USI session census.
#[derive(Debug, Clone)]
pub struct Usi {
    /// `usi`/`usiok` handshake lines.
    pub handshake: usize,
    /// `id name`/`id author` lines.
    pub id_lines: usize,
    /// `option name X type Y` lines.
    pub options: usize,
    /// `isready`/`readyok`/`usinewgame`/`quit`/`stop`/`gameover`/`ponderhit` lines.
    pub controls: usize,
    /// `position startpos|sfen … moves …` lines.
    pub positions: usize,
    /// Moves listed in `position … moves …` tails.
    pub position_moves: usize,
    /// `go …` lines.
    pub go: usize,
    /// `info …` lines.
    pub info_lines: usize,
    /// Distinct `info` field keys seen (depth/score/nodes/pv/time/hashfull/…).
    pub info_fields: usize,
    /// `bestmove` value (`7g7f`/`resign`/`win`/`null`).
    pub bestmove: String,
    /// `checkmate`/`ponder` tokens in the bestmove line.
    pub bestmove_extras: usize,
}

const INFO_KEYS: &[&str] = &[
    "depth",
    "seldepth",
    "time",
    "nodes",
    "pv",
    "multipv",
    "score",
    "cp",
    "mate",
    "lowerbound",
    "upperbound",
    "currmove",
    "hashfull",
    "nps",
    "string",
];

/// Detects a USI session: `usi`/`usiok`/`readyok` or `position`/`bestmove` lines.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let l = l.trim();
        matches!(l, "usi" | "usiok" | "readyok" | "usinewgame")
            || l.starts_with("position startpos")
            || l.starts_with("position sfen ")
            || l.starts_with("bestmove ")
            || l.starts_with("info depth")
    })
}

/// Parses a USI session; `None` without USI lines.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Usi> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut u = Usi {
        handshake: 0,
        id_lines: 0,
        options: 0,
        controls: 0,
        positions: 0,
        position_moves: 0,
        go: 0,
        info_lines: 0,
        info_fields: 0,
        bestmove: String::new(),
        bestmove_extras: 0,
    };
    let mut info_seen = 0usize;
    for line in t.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if matches!(l, "usi" | "usiok") {
            u.handshake += 1;
        } else if l.starts_with("id ") {
            u.id_lines += 1;
        } else if l.starts_with("option ") {
            u.options += 1;
        } else if l.starts_with("position ") {
            u.positions += 1;
            if let Some(mi) = l.find(" moves ") {
                u.position_moves += l[mi + 7..].split_whitespace().count();
            }
        } else if l.starts_with("go ") || l == "go" {
            u.go += 1;
        } else if l.starts_with("info ") {
            u.info_lines += 1;
            for k in INFO_KEYS {
                if l.split_whitespace().any(|w| w == *k) {
                    info_seen |= 1 << INFO_KEYS.iter().position(|x| x == k).unwrap_or(0);
                }
            }
        } else if l.starts_with("bestmove ") {
            u.bestmove = l.split_whitespace().nth(1).unwrap_or("").to_string();
            u.bestmove_extras += l
                .split_whitespace()
                .filter(|w| matches!(*w, "ponder" | "checkmate"))
                .count();
        } else if matches!(
            l,
            "isready" | "readyok" | "usinewgame" | "quit" | "stop" | "ponderhit"
        ) || l.starts_with("gameover")
        {
            u.controls += 1;
        }
    }
    u.info_fields = info_seen.count_ones() as usize;
    Some(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"usi\nid name demo\nid author jane\noption name Threads type spin default 1 min 1 max 256\noption name BookFile type string default public\x2ebook\nusiok\nisready\nreadyok\nusinewgame\nposition startpos moves 7g7f 3c3d 2g2f\ngo btime 300000 wtime 300000\ninfo depth 12 seldepth 18 score cp 34 nodes 12345 pv 7g7f 3c3d\ninfo depth 13 score cp 40 pv 2g2f\nbestmove 7g7f ponder 3c3d\nquit\n";

    #[test]
    fn parses() {
        let u = parse(D).unwrap();
        assert_eq!(u.handshake, 2);
        assert_eq!(u.id_lines, 2);
        assert_eq!(u.options, 2);
        assert_eq!(u.positions, 1);
        assert_eq!(u.position_moves, 3);
        assert_eq!(u.go, 1);
        assert_eq!(u.info_lines, 2);
        assert_eq!(u.info_fields, 6);
        assert_eq!(u.bestmove, "7g7f");
        assert_eq!(u.bestmove_extras, 1);
        assert_eq!(u.controls, 4);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"usi\nusiok\n"));
        assert!(detect(b"position startpos\n"));
        assert!(!detect(b"go now\n"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"key: value\n").is_none());
    }
}
