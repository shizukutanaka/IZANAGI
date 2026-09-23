//! Regular expressions over bytes — Thompson NFA construction
//! (Thompson 1968, Cox's "Regular Expression Matching Can Be
//! Simple And Fast") with an explicit `O(#states)`-per-byte
//! simulation: no backtracking, no exponential blow-up, and the
//! compiled program is a pure function of the pattern.
//!
//! Supported syntax:
//! - literals and `.` (any byte)
//! - `*` `+` `?` postfix repetition, `|` alternation, `()` grouping
//! - `[abc]` `[a-z]` `[^a-z]` byte classes, escapes `\.` `\d` `\w`
//!   `\s` (and their capitals) `\\` `\-` etc.
//!
//! `is_match` is a substring search (like `grep`); `full_match`
//! anchors both ends. All semantics are byte-oriented and `&str`
//! inputs are matched on their UTF-8 bytes — character classes are
//! byte classes, so multibyte codepoints are matched byte-wise
//! (deterministic, and documented).
//!
//! ```
//! use izanagi_kit::regex::Regex;
//!
//! let re = Regex::new("ca(t|r)+s?").unwrap();
//! assert!(re.is_match("the cats sat"));
//! assert!(re.full_match("cat"));
//! assert!(!re.full_match("cats!"));
//! ```
//!
//! References: Thompson, "Regular expression search algorithm"
//! (1968); Cox, *regexp2.txt*.

/// One NFA instruction.
enum Inst {
    /// Consume this byte → `next`.
    Byte(u8, usize),
    /// Consume any byte → `next`.
    Any(usize),
    /// Consume a byte in the 256-bit set → `next`.
    Class([u64; 4], usize),
    /// Epsilon-split: try `a`, fall back to `b`.
    Split(usize, usize),
    /// Epsilon jump.
    Jmp(usize),
    /// Accept.
    Match,
}

/// One unparsed parse position.
struct Parser<'a> {
    pat: &'a [u8],
    pos: usize,
}

/// A compiled fragment: entry state + dangling target slots.
struct Frag {
    start: usize,
    outs: Vec<(usize, usize)>, // (inst index, slot)
}

/// Compiled regex: deterministic NFA program.
pub struct Regex {
    prog: Vec<Inst>,
    /// Entry instruction index (the compiled fragment's start).
    start: usize,
}

const HOLE: usize = !0; // placeholder target, always patched before use

impl Regex {
    /// Compile `pat`; `None` on a syntax error (unbalanced paren or
    /// class, dangling escape, misplaced `|`/`*`).
    pub fn new(pat: &str) -> Option<Regex> {
        let mut prog = Vec::new();
        let mut p = Parser {
            pat: pat.as_bytes(),
            pos: 0,
        };
        let frag = p.expr(&mut prog)?;
        if p.pos != p.pat.len() {
            return None; // stray ')' etc.
        }
        prog.push(Inst::Match);
        let m = prog.len() - 1;
        for &(i, slot) in &frag.outs {
            patch(&mut prog, i, slot, m);
        }
        Some(Regex {
            prog,
            start: frag.start,
        })
    }

    /// Substring search: `true` iff some contiguous slice of
    /// `text`'s bytes is matched.
    pub fn is_match(&self, text: &str) -> bool {
        self.is_match_bytes(text.as_bytes())
    }

    /// Byte-wise substring search.
    pub fn is_match_bytes(&self, text: &[u8]) -> bool {
        // Unanchored pike loop: reseed the start state at every
        // position so a match can begin anywhere.
        let mut clist = vec![false; self.prog.len()];
        let mut seen = vec![false; self.prog.len()];
        self.add_state(self.start, &mut clist, &mut seen);
        if clist_ends(&self.prog, &clist) {
            return true;
        }
        for &b in text {
            let mut nlist = vec![false; self.prog.len()];
            seen.iter_mut().for_each(|v| *v = false);
            self.step(&clist, &mut nlist, &mut seen, b);
            // reseed
            self.add_state(self.start, &mut nlist, &mut seen);
            if clist_ends(&self.prog, &nlist) {
                return true;
            }
            clist = nlist;
        }
        false
    }

    /// Anchored match over the whole input.
    pub fn full_match(&self, text: &str) -> bool {
        self.full_match_bytes(text.as_bytes())
    }

    /// Byte-wise anchored match.
    pub fn full_match_bytes(&self, text: &[u8]) -> bool {
        let mut clist = vec![false; self.prog.len()];
        let mut seen = vec![false; self.prog.len()];
        self.add_state(self.start, &mut clist, &mut seen);
        for &b in text {
            let mut nlist = vec![false; self.prog.len()];
            seen.iter_mut().for_each(|v| *v = false);
            self.step(&clist, &mut nlist, &mut seen, b);
            clist = nlist;
        }
        clist_ends(&self.prog, &clist)
    }

    /// Epsilon-closure: add `i` and everything reachable via
    /// Split/Jmp into `set`.
    fn add_state(&self, i: usize, set: &mut [bool], seen: &mut [bool]) {
        if seen[i] {
            return;
        }
        seen[i] = true;
        match &self.prog[i] {
            Inst::Split(a, b) => {
                self.add_state(*a, set, seen);
                self.add_state(*b, set, seen);
            }
            Inst::Jmp(t) => self.add_state(*t, set, seen),
            _ => set[i] = true,
        }
    }

    /// One byte step: from `cur` states, fire transitions on `b`
    /// and epsilon-close targets into `next`.
    fn step(&self, cur: &[bool], next: &mut [bool], seen: &mut [bool], b: u8) {
        for (i, on) in cur.iter().enumerate() {
            if !on {
                continue;
            }
            let t = match &self.prog[i] {
                Inst::Byte(x, t) if *x == b => Some(*t),
                Inst::Any(t) => Some(*t),
                Inst::Class(bits, t) if class_has(bits, b) => Some(*t),
                _ => None,
            };
            if let Some(t) = t {
                self.add_state(t, next, seen);
            }
        }
    }
}

fn clist_ends(prog: &[Inst], set: &[bool]) -> bool {
    set.iter()
        .enumerate()
        .any(|(i, &on)| on && matches!(prog[i], Inst::Match))
}

fn class_has(bits: &[u64; 4], b: u8) -> bool {
    bits[(b >> 6) as usize] & (1u64 << (b & 63)) != 0
}

fn class_set(bits: &mut [u64; 4], b: u8) {
    bits[(b >> 6) as usize] |= 1u64 << (b & 63);
}

fn patch(prog: &mut [Inst], i: usize, slot: usize, target: usize) {
    match &mut prog[i] {
        Inst::Byte(_, t) | Inst::Any(t) | Inst::Class(_, t) | Inst::Jmp(t) => *t = target,
        Inst::Split(a, b) => {
            if slot == 0 {
                *a = target;
            } else {
                *b = target;
            }
        }
        Inst::Match => {}
    }
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.pat.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// expr := concat ('|' concat)*
    fn expr(&mut self, prog: &mut Vec<Inst>) -> Option<Frag> {
        let mut acc = self.concat(prog)?;
        while self.eat(b'|') {
            let rhs = self.concat(prog)?;
            let s = prog.len();
            prog.push(Inst::Split(acc.start, rhs.start));
            let mut outs = acc.outs;
            outs.extend(rhs.outs);
            acc = Frag { start: s, outs };
        }
        Some(acc)
    }

    /// concat := rep*  (ends at '|', ')' or EOF)
    fn concat(&mut self, prog: &mut Vec<Inst>) -> Option<Frag> {
        let mut acc: Option<Frag> = None;
        loop {
            match self.peek() {
                None | Some(b'|') | Some(b')') => break,
                _ => {}
            }
            let f = self.rep(prog)?;
            acc = Some(match acc {
                None => f,
                Some(a) => {
                    for &(i, s) in &a.outs {
                        patch(prog, i, s, f.start);
                    }
                    Frag {
                        start: a.start,
                        outs: f.outs,
                    }
                }
            });
        }
        Some(match acc {
            Some(f) => f,
            None => {
                // Empty alternative — epsilon.
                let i = prog.len();
                prog.push(Inst::Jmp(HOLE));
                Frag {
                    start: i,
                    outs: vec![(i, 0)],
                }
            }
        })
    }

    /// rep := atom ('*'|'+'|'?')*
    fn rep(&mut self, prog: &mut Vec<Inst>) -> Option<Frag> {
        let mut f = self.atom(prog)?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    let s = prog.len();
                    prog.push(Inst::Split(f.start, HOLE));
                    for &(i, sl) in &f.outs {
                        patch(prog, i, sl, s);
                    }
                    f = Frag {
                        start: s,
                        outs: vec![(s, 1)],
                    };
                }
                Some(b'+') => {
                    self.pos += 1;
                    let s = prog.len();
                    prog.push(Inst::Split(f.start, HOLE));
                    for &(i, sl) in &f.outs {
                        patch(prog, i, sl, s);
                    }
                    f = Frag {
                        start: f.start,
                        outs: vec![(s, 1)],
                    };
                }
                Some(b'?') => {
                    self.pos += 1;
                    let s = prog.len();
                    prog.push(Inst::Split(f.start, HOLE));
                    let mut outs = f.outs;
                    outs.push((s, 1));
                    f = Frag { start: s, outs };
                }
                _ => break,
            }
        }
        Some(f)
    }

    /// atom := '(' expr ')' | '[' class ']' | '.' | escape | literal
    fn atom(&mut self, prog: &mut Vec<Inst>) -> Option<Frag> {
        match self.bump()? {
            b'(' => {
                let f = self.expr(prog)?;
                if !self.eat(b')') {
                    return None;
                }
                Some(f)
            }
            b'[' => {
                let bits = self.class()?;
                let i = prog.len();
                prog.push(Inst::Class(bits, HOLE));
                Some(Frag {
                    start: i,
                    outs: vec![(i, 0)],
                })
            }
            b'.' => {
                let i = prog.len();
                prog.push(Inst::Any(HOLE));
                Some(Frag {
                    start: i,
                    outs: vec![(i, 0)],
                })
            }
            b'\\' => {
                let b = self.bump()?;
                Some(match b {
                    b'd' => self.named_class(prog, b"0-9", false),
                    b'w' => self.named_class(prog, b"a-zA-Z0-9_", false),
                    b's' => self.named_class(prog, b" \t\n\r", false),
                    b'D' => self.named_class(prog, b"0-9", true),
                    b'W' => self.named_class(prog, b"a-zA-Z0-9_", true),
                    b'S' => self.named_class(prog, b" \t\n\r", true),
                    _ => self.literal(prog, b),
                })
            }
            b'*' | b'+' | b'?' | b'|' | b')' => None, // stray operator
            b => Some(self.literal(prog, b)),
        }
    }

    fn literal(&mut self, prog: &mut Vec<Inst>, b: u8) -> Frag {
        let i = prog.len();
        prog.push(Inst::Byte(b, HOLE));
        Frag {
            start: i,
            outs: vec![(i, 0)],
        }
    }

    /// Emit a `Class` from a compact spec like `b"a-zA-Z0-9_"`.
    fn named_class(&mut self, prog: &mut Vec<Inst>, spec: &[u8], negate: bool) -> Frag {
        let mut bits = [0u64; 4];
        let mut i = 0;
        while i < spec.len() {
            if i + 2 < spec.len() && spec[i + 1] == b'-' {
                let (lo, hi) = (spec[i], spec[i + 2]);
                for b in lo..=hi {
                    class_set(&mut bits, b);
                }
                i += 3;
            } else {
                class_set(&mut bits, spec[i]);
                i += 1;
            }
        }
        if negate {
            for w in &mut bits {
                *w = !*w;
            }
        }
        let idx = prog.len();
        prog.push(Inst::Class(bits, HOLE));
        Frag {
            start: idx,
            outs: vec![(idx, 0)],
        }
    }

    /// `'[' ... ']'` → 256-bit set.
    fn class(&mut self) -> Option<[u64; 4]> {
        let mut bits = [0u64; 4];
        let negate = self.eat(b'^');
        let mut first = true;
        loop {
            let c = self.bump()?;
            if c == b']' && !first {
                break;
            }
            first = false;
            let lo = match c {
                b'\\' => {
                    let e = self.bump()?;
                    match e {
                        b'd' => {
                            for b in b'0'..=b'9' {
                                class_set(&mut bits, b);
                            }
                            continue;
                        }
                        b'n' => b'\n',
                        b't' => b'\t',
                        b'r' => b'\r',
                        other => other,
                    }
                }
                other => other,
            };
            // Range?
            if self.peek() == Some(b'-') && self.pat.get(self.pos + 1) != Some(&b']') {
                self.pos += 1; // '-'
                let hi = self.bump()?;
                if hi < lo {
                    return None;
                }
                for b in lo..=hi {
                    class_set(&mut bits, b);
                }
            } else {
                class_set(&mut bits, lo);
            }
        }
        if negate {
            for w in &mut bits {
                *w = !*w;
            }
        }
        Some(bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_and_concat() {
        let re = Regex::new("cat").unwrap();
        assert!(re.full_match("cat"));
        assert!(!re.full_match("ca"));
        assert!(!re.full_match("cats"));
        assert!(re.is_match("concatenate"));
        assert!(!re.is_match("dog"));
    }

    #[test]
    fn operators() {
        let re = Regex::new("ab*c").unwrap();
        assert!(re.full_match("ac"));
        assert!(re.full_match("abbbbc"));
        let re = Regex::new("a(b|c)*d").unwrap();
        assert!(re.full_match("ad"));
        assert!(re.full_match("abcbd"));
        assert!(!re.full_match("ax"));
        let re = Regex::new("colou?r").unwrap();
        assert!(re.full_match("color"));
        assert!(re.full_match("colour"));
        let re = Regex::new("a|bc*").unwrap();
        assert!(re.full_match("a"));
        assert!(re.full_match("bccc"));
        assert!(!re.full_match("ab"));
    }

    #[test]
    fn classes_and_dot() {
        let re = Regex::new("[a-c]+").unwrap();
        assert!(re.full_match("abca"));
        assert!(!re.full_match("abcx"));
        let re = Regex::new("[^0-9]").unwrap();
        assert!(re.full_match("z"));
        assert!(!re.full_match("5"));
        let re = Regex::new("a.c").unwrap();
        assert!(re.full_match("axc"));
        assert!(!re.full_match("ac"));
        let re = Regex::new("\\d+\\.\\d+").unwrap();
        assert!(re.full_match("12.34"));
        assert!(!re.full_match("12"));
    }

    #[test]
    fn empty_and_anchor_semantics() {
        let re = Regex::new("").unwrap();
        assert!(re.is_match("anything"));
        assert!(re.full_match(""));
        assert!(!re.full_match("x"));
        assert!(re.full_match_bytes(b""));
        assert!(re.is_match_bytes(b"anything"));
        assert!(re.is_match_bytes(b""));
        let re = Regex::new("a|").unwrap();
        assert!(re.full_match(""));
        assert!(re.full_match("a"));
    }

    #[test]
    fn parse_errors_rejected() {
        assert!(Regex::new("(").is_none());
        assert!(Regex::new("a)").is_none());
        assert!(Regex::new("[a-").is_none());
        assert!(Regex::new("a\\").is_none());
        assert!(Regex::new("*a").is_none());
    }

    // ---- oracle: random AST → pattern; semantic matcher on AST ----

    #[derive(Clone)]
    enum Ast {
        Lit(u8),
        Any,
        Class(Vec<u8>),
        Cat(Vec<Ast>),
        Alt(Vec<Ast>),
        Star(Box<Ast>),
        Plus(Box<Ast>),
        Opt(Box<Ast>),
    }

    fn render(a: &Ast, out: &mut Vec<u8>) {
        match a {
            Ast::Lit(b) => out.push(*b),
            Ast::Any => out.push(b'.'),
            Ast::Class(cs) => {
                out.push(b'[');
                for &c in cs {
                    out.push(c);
                }
                out.push(b']');
            }
            Ast::Cat(v) => {
                if v.is_empty() {
                    out.extend_from_slice(b"()"); // empty group = ε
                } else {
                    for a in v {
                        render(a, out);
                    }
                }
            }
            Ast::Alt(v) => {
                out.push(b'(');
                for (i, a) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(b'|');
                    }
                    render(a, out);
                }
                out.push(b')');
            }
            Ast::Star(a) => {
                out.push(b'(');
                render(a, out);
                out.extend_from_slice(b")*");
            }
            Ast::Plus(a) => {
                out.push(b'(');
                render(a, out);
                out.extend_from_slice(b")+");
            }
            Ast::Opt(a) => {
                out.push(b'(');
                render(a, out);
                out.extend_from_slice(b")?");
            }
        }
    }

    /// All end positions reachable by matching `a` on `text[pos..]`.
    /// A second, independent implementation of the semantics.
    fn ast_match(a: &Ast, text: &[u8], pos: usize, outs: &mut Vec<usize>, depth: usize) {
        if depth > 64 {
            return;
        }
        match a {
            Ast::Lit(b) => {
                if text.get(pos) == Some(b) {
                    outs.push(pos + 1);
                }
            }
            Ast::Any => {
                if pos < text.len() {
                    outs.push(pos + 1);
                }
            }
            Ast::Class(cs) => {
                if text.get(pos).is_some_and(|b| cs.contains(b)) {
                    outs.push(pos + 1);
                }
            }
            Ast::Cat(v) => {
                let mut frontier = vec![pos];
                for a in v {
                    let mut next = Vec::new();
                    for &p in &frontier {
                        ast_match(a, text, p, &mut next, depth + 1);
                    }
                    next.sort_unstable();
                    next.dedup();
                    frontier = next;
                    if frontier.is_empty() {
                        break;
                    }
                }
                outs.extend(frontier);
            }
            Ast::Alt(v) => {
                for a in v {
                    ast_match(a, text, pos, outs, depth + 1);
                }
            }
            Ast::Star(a) => {
                // Positions only (the match is position-deterministic),
                // deduped against this Star's own seen set — shared
                // `outs` would wrongly drop positions other branches
                // legitimately reported.
                let mut seen = vec![pos];
                let mut frontier = vec![pos];
                outs.push(pos);
                for _ in 0..text.len() + 1 {
                    let mut next = Vec::new();
                    for &p in &frontier {
                        ast_match(a, text, p, &mut next, depth + 1);
                    }
                    next.retain(|p| !seen.contains(p));
                    next.sort_unstable();
                    next.dedup();
                    if next.is_empty() {
                        break;
                    }
                    seen.extend(next.iter().copied());
                    outs.extend(next.iter().copied());
                    frontier = next;
                }
            }
            Ast::Plus(a) => {
                // a·a*
                let mut mid = Vec::new();
                ast_match(a, text, pos, &mut mid, depth + 1);
                mid.sort_unstable();
                mid.dedup();
                for &p in &mid {
                    outs.push(p);
                    ast_match(&Ast::Star(a.clone()), text, p, outs, depth + 1);
                }
            }
            Ast::Opt(a) => {
                outs.push(pos);
                ast_match(a, text, pos, outs, depth + 1);
            }
        }
    }

    fn ast_full(a: &Ast, text: &[u8]) -> bool {
        let mut outs = Vec::new();
        ast_match(a, text, 0, &mut outs, 0);
        outs.contains(&text.len())
    }

    fn ast_sub(a: &Ast, text: &[u8]) -> bool {
        (0..=text.len()).any(|p| {
            let mut outs = Vec::new();
            ast_match(a, text, p, &mut outs, 0);
            !outs.is_empty()
        })
    }

    #[test]
    fn matches_ast_oracle() {
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xE616);

        fn gen(rng: &mut SplitMix64, depth: usize) -> Ast {
            let lit = |rng: &mut SplitMix64| Ast::Lit(b'a' + (rng.next_u64() % 3) as u8);
            if depth >= 3 {
                return lit(rng);
            }
            match rng.next_u64() % 8 {
                0 | 1 => lit(rng),
                2 => Ast::Any,
                3 => Ast::Class(vec![b'a', b'b']),
                4 => Ast::Cat(vec![
                    gen(rng, depth + 1),
                    gen(rng, depth + 1),
                    gen(rng, depth + 1),
                ]),
                5 => Ast::Alt(vec![gen(rng, depth + 1), gen(rng, depth + 1)]),
                6 => Ast::Star(Box::new(gen(rng, depth + 1))),
                7 => {
                    if rng.next_u64() % 2 == 0 {
                        Ast::Plus(Box::new(gen(rng, depth + 1)))
                    } else {
                        Ast::Opt(Box::new(gen(rng, depth + 1)))
                    }
                }
                _ => lit(rng),
            }
        }

        for _ in 0..400 {
            let ast = gen(&mut rng, 0);
            let mut pat = Vec::new();
            render(&ast, &mut pat);
            let pat = String::from_utf8(pat).unwrap_or_else(|_| panic!("pattern"));
            let re = Regex::new(&pat).unwrap_or_else(|| panic!("bad pattern {pat:?}"));
            for _ in 0..20 {
                let len = rng.next_u64() % 9;
                let text: Vec<u8> = (0..len)
                    .map(|_| b'a' + (rng.next_u64() % 4) as u8)
                    .collect();
                let text = String::from_utf8(text).unwrap_or_else(|_| panic!("text"));
                assert_eq!(
                    re.full_match(&text),
                    ast_full(&ast, text.as_bytes()),
                    "full_match {pat:?} vs {text:?}"
                );
                assert_eq!(
                    re.is_match(&text),
                    ast_sub(&ast, text.as_bytes()),
                    "is_match {pat:?} vs {text:?}"
                );
            }
        }
    }
}
