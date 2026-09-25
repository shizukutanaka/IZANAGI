//! Portable Game Notation — chess records: `[Tag "value"]` pairs then
//! movetext of SAN moves, move-number glyphs (`1.`/`...`), NAGs `$n`,
//! `{comments}`, `()` recursive variations, and a terminator
//! (`1-0`, `0-1`, `1/2-1/2`, `*`). [`sgf`](crate::sgf)'s chess sibling.
//!
//! ```
//! use izanagi_kit::pgn::{parse, result};
//! let g = parse("[White \"a\"]\n[Black \"b\"]\n\n1. e4 e5 2. Nf3 Nc6 1-0\n").unwrap();
//! assert_eq!(g[0].moves.len(), 4);
//! assert_eq!(result(&g[0]), "1-0");
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// One movetext move.
#[derive(Clone, Debug, PartialEq)]
pub struct Move {
    /// SAN verbatim (`"e4"`, `"Nxf7+"`, `"O-O"`, …).
    pub san: String,
    /// NAG annotations (`$1` → 1).
    pub nags: Vec<u32>,
    /// `{…}` comment (braces stripped).
    pub comment: Option<String>,
    /// RAV variations branching from this move.
    pub variations: Vec<Vec<Move>>,
}

/// A game record: tags, main-line moves, terminator.
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    /// `[Name "value"]` pairs.
    pub tags: BTreeMap<String, String>,
    /// Main-line moves (colour agnostic).
    pub moves: Vec<Move>,
    /// The terminator verbatim (`"1-0"`, `"1/2-1/2"`, `"*"`, …).
    pub result: String,
    /// Whether the first recorded move is White's (`1... e5` starts black).
    pub first_is_white: bool,
}

/// The game's terminator.
pub fn result(g: &Game) -> &str {
    &g.result
}

const MAX_DEPTH: usize = 64;

struct P<'a> {
    s: &'a [u8],
    i: usize,
    /// Terminator most recently seen (shared across recursion).
    term: Option<String>,
    /// Saw `N...` / `...` before the first SAN.
    first_black: bool,
}

fn mv(san: String) -> Move {
    Move {
        san,
        nags: Vec::new(),
        comment: None,
        variations: Vec::new(),
    }
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        loop {
            match self.s.get(self.i).copied() {
                Some(b) if (b as char).is_whitespace() => self.i += 1,
                Some(b'%') => {
                    // %-directive lines run to EOL
                    while self.i < self.s.len() && self.s[self.i] != b'\n' {
                        self.i += 1;
                    }
                }
                _ => return,
            }
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn word(&mut self) -> &'a [u8] {
        let st = self.i;
        while self.i < self.s.len()
            && !(self.s[self.i] as char).is_whitespace()
            && !matches!(
                self.s[self.i],
                b'(' | b')' | b'{' | b'}' | b'$' | b'[' | b']'
            )
        {
            self.i += 1;
        }
        &self.s[st..self.i]
    }
    /// `[Name "value"]` → `(name, value)`.
    fn tag(&mut self) -> Option<(String, String)> {
        if self.peek() != Some(b'[') {
            return None;
        }
        self.i += 1;
        let st = self.i;
        while self.i < self.s.len() && !matches!(self.s[self.i], b' ' | b'\t' | b'"' | b']') {
            self.i += 1;
        }
        let name = String::from_utf8(self.s[st..self.i].to_vec()).ok()?;
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.i += 1;
        }
        if self.peek() != Some(b'"') {
            return None;
        }
        self.i += 1;
        let mut v = Vec::new();
        loop {
            match *self.s.get(self.i)? {
                b'\\' => {
                    v.push(*self.s.get(self.i + 1)?);
                    self.i += 2;
                }
                b'"' => {
                    self.i += 1;
                    break;
                }
                c => {
                    v.push(c);
                    self.i += 1;
                }
            }
        }
        if self.peek() != Some(b']') {
            return None;
        }
        self.i += 1;
        Some((name, String::from_utf8(v).ok()?))
    }
    /// One movetext sequence until `)` / terminator / EOF.
    fn sequence(&mut self, depth: usize) -> Option<Vec<Move>> {
        if depth > MAX_DEPTH {
            return None;
        }
        let mut out: Vec<Move> = Vec::new();
        loop {
            self.ws();
            match self.peek()? {
                b'(' => {
                    self.i += 1;
                    let v = self.sequence(depth + 1)?;
                    out.last_mut()?.variations.push(v);
                }
                b')' => {
                    self.i += 1;
                    return Some(out);
                }
                b'{' => {
                    self.i += 1;
                    let st = self.i;
                    while self.i < self.s.len() && self.s[self.i] != b'}' {
                        self.i += 1;
                    }
                    let c = String::from_utf8(self.s[st..self.i].to_vec()).ok()?;
                    self.i += 1; // '}'
                    if let Some(m) = out.last_mut() {
                        m.comment = Some(c);
                    }
                }
                b'$' => {
                    self.i += 1;
                    let st = self.i;
                    while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                        self.i += 1;
                    }
                    let n: u32 = std::str::from_utf8(&self.s[st..self.i])
                        .ok()?
                        .parse()
                        .ok()?;
                    if let Some(m) = out.last_mut() {
                        m.nags.push(n);
                    }
                }
                b';' => {
                    while self.i < self.s.len() && self.s[self.i] != b'\n' {
                        self.i += 1;
                    }
                }
                _ => {
                    let tok = self.word();
                    let tok = std::str::from_utf8(tok).ok()?;
                    if tok.is_empty() {
                        return None;
                    }
                    match tok {
                        "1-0" | "0-1" | "1/2-1/2" | "*" => {
                            self.term = Some(tok.to_string());
                            return Some(out);
                        }
                        _ => {}
                    }
                    if tok.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                        // move number; `...`/`N...` flags black-to-move
                        if out.is_empty() && tok.contains("...") {
                            self.first_black = true;
                        }
                    } else {
                        out.push(mv(tok.to_string()));
                    }
                }
            }
        }
    }
}

/// Parse zero or more games; `None` on malformed input.
pub fn parse(src: &str) -> Option<Vec<Game>> {
    let b = src.replace("\r\n", "\n");
    let mut p = P {
        s: b.as_bytes(),
        i: 0,
        term: None,
        first_black: false,
    };
    let mut games = Vec::new();
    loop {
        p.ws();
        if p.i >= b.len() {
            return Some(games);
        }
        let mut tags = BTreeMap::new();
        while let Some((k, v)) = p.tag() {
            tags.insert(k, v);
            p.ws();
        }
        p.first_black = false;
        p.term = None;
        let moves = p.sequence(0)?;
        let result = p.term.clone().unwrap_or_else(|| "*".to_string());
        games.push(Game {
            tags,
            moves,
            result,
            first_is_white: !p.first_black,
        });
    }
}

fn emit_seq(moves: &[Move], black_first: bool, start_num: u32, s: &mut String) {
    for (i, m) in moves.iter().enumerate() {
        let is_white = (i % 2 == 0) != black_first;
        let num = start_num + (i as u32 + black_first as u32) / 2;
        if is_white {
            s.push_str(&std::format!("{num}. "));
        }
        s.push_str(&m.san);
        for n in &m.nags {
            s.push_str(&std::format!(" ${n}"));
        }
        if let Some(c) = &m.comment {
            s.push_str(&std::format!(" {{{c}}}"));
        }
        for v in &m.variations {
            s.push_str(" (");
            // a variation replaces this move: same number, same colour
            emit_seq(v, !is_white, num, s);
            s.push(')');
        }
        s.push(' ');
    }
}

/// Canonical emission: tags, blank line, numbered movetext, result.
pub fn emit(g: &Game) -> String {
    let mut s = String::new();
    for (k, v) in &g.tags {
        s.push_str(&std::format!("[{k} \"{v}\"]\n"));
    }
    s.push('\n');
    emit_seq(&g.moves, !g.first_is_white, 1, &mut s);
    s.push_str(&g.result);
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const G1: &str =
        "[Event \"?\"]\n[White \"a\"]\n[Black \"b\"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bc4 (3. Bb5 a6) Nf6 $1 {two knights} 1-0\n";

    #[test]
    fn basic_game() {
        let g = parse(G1).unwrap();
        assert_eq!(g.len(), 1);
        let g = &g[0];
        assert_eq!(g.tags["White"], "a");
        assert_eq!(result(g), "1-0");
        assert_eq!(g.moves.len(), 6);
        assert_eq!(g.moves[4].san, "Bc4");
        assert_eq!(g.moves[4].variations.len(), 1);
        assert_eq!(g.moves[4].variations[0][0].san, "Bb5");
        assert_eq!(g.moves[5].nags, vec![1]);
        assert_eq!(g.moves[5].comment.as_deref(), Some("two knights"));
    }

    #[test]
    fn black_first_and_terminators() {
        let g = parse("[FEN \"x\"]\n\n1... e5 2. Nf3 *\n").unwrap();
        assert!(!g[0].first_is_white);
        assert_eq!(result(&g[0]), "*");
        for t in ["1-0", "0-1", "1/2-1/2", "*"] {
            let g = parse(&std::format!("1. e4 {t}\n")).unwrap();
            assert_eq!(result(&g[0]), t);
        }
    }

    #[test]
    fn multiple_games_and_empty() {
        let g = parse("1. e4 *\n\n[White \"b\"]\n\n1. d4 d5 0-1\n").unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(parse(""), Some(vec![]));
        assert_eq!(parse("   \n"), Some(vec![]));
    }

    #[test]
    fn escapes_in_tags() {
        let g = parse("[White \"a\\\"b\"]\n\n1. e4 *\n").unwrap();
        assert_eq!(g[0].tags["White"], "a\"b");
    }

    #[test]
    fn emit_roundtrips() {
        let g = parse(G1).unwrap();
        assert_eq!(parse(&emit(&g[0])).unwrap()[0], g[0]);
    }

    #[test]
    fn malformed_rejected() {
        for bad in [
            "(",                     // unterminated variation
            "(1. e4",                // unterminated
            "(e5) 1. e4 *",          // variation off nothing
            "[White a]\n\n1. e4 *",  // unquoted tag
            "[White \"a]\n\n1. e4*", // unterminated string
            "$x 1. e4 *",            // bad NAG
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(G1), parse(G1));
        assert_eq!(emit(&parse(G1).unwrap()[0]), emit(&parse(G1).unwrap()[0]));
    }
}
