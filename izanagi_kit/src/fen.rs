//! Forsyth–Edwards Notation — chess positions:
//! `pieces w|b castling ep halfmove fullmove`, e.g. the start position
//! `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`.
//! [`pgn`](crate::pgn)'s board sibling. The board is stored as 64
//! cells of ASCII piece letters (`p n b r q k`, uppercase = White) or
//! `0`; index 0 = a8, 63 = h1.
//!
//! ```
//! use izanagi_kit::fen::{parse, emit};
//! let f = parse("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
//! assert!(f.white && f.castling == 0b1111);
//! assert_eq!(emit(&f), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
//! ```

use std::string::String;

/// Castling availability bits.
pub const CASTLE_K: u8 = 1;
/// `q` side, white.
pub const CASTLE_Q: u8 = 2;
/// `k` side, black.
pub const CASTLE_BK: u8 = 4;
/// `q` side, black.
pub const CASTLE_BQ: u8 = 8;

/// A parsed FEN position.
#[derive(Clone, Debug, PartialEq)]
pub struct Fen {
    /// 64 cells, row 0 = rank 8; ASCII piece letter or `0`.
    pub board: [u8; 64],
    /// White to move.
    pub white: bool,
    /// Castling rights: [`CASTLE_K`]|[`CASTLE_Q`]|[`CASTLE_BK`]|[`CASTLE_BQ`].
    pub castling: u8,
    /// En-passant target square index (`a8`=0 … `h1`=63), if any.
    pub ep: Option<u8>,
    /// Halfmove clock (50-move rule counter).
    pub halfmove: u32,
    /// Fullmove number (≥ 1).
    pub fullmove: u32,
}

fn is_piece(b: u8) -> bool {
    matches!(
        b,
        b'p' | b'n' | b'b' | b'r' | b'q' | b'k' | b'P' | b'N' | b'B' | b'R' | b'Q' | b'K'
    )
}

fn u32_of(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Parse a FEN record; `None` on any malformed field.
pub fn parse(src: &str) -> Option<Fen> {
    let f: Vec<&str> = src.trim().split(' ').filter(|t| !t.is_empty()).collect();
    if f.len() != 6 {
        return None;
    }
    let mut board = [0u8; 64];
    let mut cell = 0usize;
    for (ri, rank) in f[0].split('/').enumerate() {
        if ri > 7 {
            return None;
        }
        let mut files = 0usize;
        for b in rank.bytes() {
            match b {
                b'1'..=b'8' => files += (b - b'0') as usize,
                _ if is_piece(b) => {
                    *board.get_mut(ri * 8 + files)? = b;
                    files += 1;
                }
                _ => return None,
            }
        }
        if files != 8 {
            return None;
        }
        cell += 8;
    }
    if cell != 64 {
        return None;
    }
    let white = match f[1] {
        "w" => true,
        "b" => false,
        _ => return None,
    };
    let mut castling = 0u8;
    if f[2] != "-" {
        for b in f[2].bytes() {
            let bit = match b {
                b'K' => CASTLE_K,
                b'Q' => CASTLE_Q,
                b'k' => CASTLE_BK,
                b'q' => CASTLE_BQ,
                _ => return None,
            };
            if castling & bit != 0 {
                return None; // duplicate right
            }
            castling |= bit;
        }
    }
    let ep = if f[3] == "-" {
        None
    } else {
        let b = f[3].as_bytes();
        if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) {
            return None;
        }
        // algebraic a1..h8 → index (rank 8 is row 0)
        let row = b'8' - b[1];
        Some(row * 8 + (b[0] - b'a'))
    };
    let halfmove = u32_of(f[4])?;
    let fullmove = u32_of(f[5])?;
    if fullmove == 0 {
        return None;
    }
    Some(Fen {
        board,
        white,
        castling,
        ep,
        halfmove,
        fullmove,
    })
}

/// Canonical emission.
pub fn emit(f: &Fen) -> String {
    let mut s = String::new();
    for r in 0..8 {
        let mut empty = 0u8;
        for c in 0..8 {
            let b = f.board[r * 8 + c];
            if b == 0 {
                empty += 1;
            } else {
                if empty > 0 {
                    s.push((b'0' + empty) as char);
                    empty = 0;
                }
                s.push(b as char);
            }
        }
        if empty > 0 {
            s.push((b'0' + empty) as char);
        }
        if r < 7 {
            s.push('/');
        }
    }
    s.push(' ');
    s.push(if f.white { 'w' } else { 'b' });
    s.push(' ');
    if f.castling == 0 {
        s.push('-');
    } else {
        for (bit, ch) in [
            (CASTLE_K, 'K'),
            (CASTLE_Q, 'Q'),
            (CASTLE_BK, 'k'),
            (CASTLE_BQ, 'q'),
        ] {
            if f.castling & bit != 0 {
                s.push(ch);
            }
        }
    }
    s.push(' ');
    match f.ep {
        Some(sq) => {
            s.push((b'a' + sq % 8) as char);
            s.push((b'8' - sq / 8) as char);
        }
        None => s.push('-'),
    }
    s.push_str(&std::format!(" {} {}", f.halfmove, f.fullmove));
    s
}

/// Count of a piece letter on the board (`'k'`/`'K'` etc.).
pub fn count(f: &Fen, piece: u8) -> u32 {
    f.board.iter().filter(|&&b| b == piece).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn start_position() {
        let f = parse(START).unwrap();
        assert!(f.white);
        assert_eq!(f.castling, 0b1111);
        assert_eq!(f.ep, None);
        assert_eq!(f.fullmove, 1);
        assert_eq!(count(&f, b'P'), 8);
        assert_eq!(count(&f, b'k'), 1);
        // a8 = 'r', e1 = 'K'
        assert_eq!(f.board[0], b'r');
        assert_eq!(f.board[60], b'K');
    }

    #[test]
    fn ep_and_black() {
        let f = parse("rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR b KQkq c6 0 2").unwrap();
        assert!(!f.white);
        // c6 → row '8'-'6'=2, file 'c'-'a'=2 → 18
        assert_eq!(f.ep, Some(18));
    }

    #[test]
    fn emit_roundtrip() {
        for s in [
            START,
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR b KQkq c6 0 2",
            "8/8/8/8/8/8/8/4K3 w - - 0 1",
            "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 5 30",
        ] {
            let f = parse(s).unwrap();
            assert_eq!(emit(&f), s);
            assert_eq!(parse(&emit(&f)).unwrap(), f);
        }
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "",
            "8/8/8/8/8/8/8/8 w - - 0",     // 5 fields
            "8/8/8/8/8/8/8/8 w - - 0 1 2", // 7
            "9/8/8/8/8/8/8/8 w - - 0 1",   // rank too wide
            "8/8/8/8/8/8/8/7 w - - 0 1",   // rank too narrow
            "8/8/8/8/8/8/8/8/8 w - - 0 1", // 9 ranks
            "8/8/8/8/8/8/8/7z w - - 0 1",  // bad piece
            "8/8/8/8/8/8/8/8 x - - 0 1",   // bad active
            "8/8/8/8/8/8/8/8 w KK - 0 1",  // dup castle
            "8/8/8/8/8/8/8/8 w X - 0 1",   // bad castle
            "8/8/8/8/8/8/8/8 w - z9 0 1",  // bad ep
            "8/8/8/8/8/8/8/8 w - - -1 1",  // bad halfmove
            "8/8/8/8/8/8/8/8 w - - 0 0",   // fullmove 0
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(START), parse(START));
        assert_eq!(emit(&parse(START).unwrap()), emit(&parse(START).unwrap()));
    }
}
