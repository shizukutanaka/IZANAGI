//! Chess Extended Position Description (EPD) — FEN-like 4-field prefix
//! plus `opcode value;` operations (`bm`, `pv`, `id`, `acd`, `acs`, `ce`, …).
//!
//! ```
//! let d = b"r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - bm O-O; id \"Italian\";\n";
//! let e = izanagi_kit::epd::parse(d).unwrap();
//! assert_eq!(e.side_to_move, "w");
//! assert_eq!(e.opcodes, 2);
//! assert!(izanagi_kit::epd::detect(d));
//! ```

/// A parsed `.epd` record census.
#[derive(Debug, Clone)]
pub struct Epd {
    /// Side to move field (`w`/`b`).
    pub side_to_move: String,
    /// Castling field (`KQkq`/`-`).
    pub castling: String,
    /// En-passant square.
    pub en_passant: String,
    /// Halfmove/fullmove counters when present in field 4.
    pub counters: usize,
    /// `opcode value;` operations.
    pub opcodes: usize,
    /// `bm`/`pv` move entries.
    pub moves: usize,
    /// Distinct opcodes seen (`bm`,`pv`,`id`,`acd`,`acs`,`am`,`ce`,`pm`,…).
    pub distinct_opcodes: usize,
    /// `id` opcode present.
    pub has_id: bool,
    /// Occupied squares in the board field.
    pub pieces: usize,
    /// Empty squares.
    pub empty: usize,
}

/// Board field looks like 8 `/`-joined rows of digits and piece letters.
fn board_ok(s: &str) -> bool {
    if s.matches('/').count() != 7 {
        return false;
    }
    let mut squares = 0usize;
    for c in s.chars() {
        match c {
            '/' => {}
            '1'..='8' => squares += (c as usize) - ('0' as usize),
            'p' | 'n' | 'b' | 'r' | 'q' | 'k' | 'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => squares += 1,
            _ => return false,
        }
    }
    squares == 64
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detects an EPD line: valid 8-row board + side `w`/`b` + opcode `;` fields.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut fields = t.split(|c: char| c.is_ascii_whitespace());
    let Some(board) = fields.next() else {
        return false;
    };
    if !board_ok(board) {
        return false;
    }
    let Some(side) = fields.next() else {
        return false;
    };
    side == "w" || side == "b"
}

/// Parses an EPD record; `None` when the board prefix is malformed.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Epd> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut fields = t.split_whitespace();
    let board = fields.next()?;
    let mut pieces = 0usize;
    let mut empty = 0usize;
    for c in board.chars() {
        match c {
            '1'..='8' => empty += (c as usize) - ('0' as usize),
            '/' => {}
            _ => pieces += 1,
        }
    }
    let side = fields.next().unwrap_or("");
    let castling = fields.next().unwrap_or("-").to_string();
    let ep = fields.next().unwrap_or("-").to_string();
    let rest = t
        .get(t.find(side).unwrap_or(0) + side.len()..)
        .unwrap_or("");
    let mut counters = 0;
    for f in fields.by_ref().take(2) {
        if f.bytes().all(|c| c.is_ascii_digit()) && !f.is_empty() {
            counters += 1;
        }
    }
    let mut distinct = 0;
    for op in [
        "bm", "pv", "id", "acd", "acs", "am", "ce", "pm", "dm", "fmvn", "hmvc", "rc", "eco", "nic",
        "v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9",
    ] {
        if rest
            .split(|c: char| !(c.is_ascii_alphanumeric()))
            .any(|w| w == op)
        {
            distinct += 1;
        }
    }
    let mut opcodes = 0;
    let mut moves = 0;
    for seg in rest.split(';') {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        opcodes += 1;
        let words: Vec<&str> = seg.split_whitespace().collect();
        if let Some(i) = words.iter().position(|w| *w == "bm" || *w == "pv") {
            moves += words.len() - i - 1;
        }
    }
    Some(Epd {
        side_to_move: side.to_string(),
        castling,
        en_passant: ep,
        counters,
        opcodes,
        moves,
        distinct_opcodes: distinct,
        has_id: rest
            .split(|c: char| !(c.is_ascii_alphanumeric()))
            .any(|w| w == "id"),
        pieces,
        empty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - bm O-O; id \"Italian Game\"; acd 3;\n";

    #[test]
    fn parses() {
        let e = parse(D).unwrap();
        assert_eq!(e.side_to_move, "w");
        assert_eq!(e.castling, "KQkq");
        assert_eq!(e.opcodes, 3);
        assert!(e.moves >= 1);
        assert!(e.has_id);
        assert!(e.distinct_opcodes >= 2);
        assert_eq!(e.pieces + e.empty, 64);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"8/8/8/8/8/8/8/8 b - - bm Ke4;\n"));
        assert!(!detect(b"8/8/8/8 b - - bm Ke4;\n"));
        assert!(!detect(b"plain text"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"r1bq/pppp w - -").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
