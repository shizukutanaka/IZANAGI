//! Deterministic integer expression evaluator — a Pratt (TDOP) parser.
//!
//! Grammar: `i64` literals (decimal, `0x`, `0b`, `0o`), parenthesized
//! groups, unary `- + ~ !`, binary `* / % + - << >>` then comparisons
//! `< <= > >= == !=`, bitwise `& ^ |`, and logical `&& ||` (nonzero is
//! true, results are 0/1). Builtin calls: `min(a,b)`, `max(a,b)`,
//! `abs(x)`, `clamp(x,lo,hi)`. Variables resolve through a caller-given
//! slice. Everything is `checked_*` — overflow, divide-by-zero, a bad
//! token, or an unknown variable all return `None`, never panic.
//!
//! ```
//! use izanagi_kit::expr::eval;
//! assert_eq!(eval("1 + 2 * 3", &[]), Some(7));
//! assert_eq!(eval("max(hp - dmg, 0)", &[("hp", 5), ("dmg", 8)]), Some(0));
//! ```

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Num(i64),
    Ident(usize, usize),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
    End,
}

fn lex(s: &str) -> Option<Vec<Tok>> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        match c {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            b')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'0'..=b'9' => {
                let st = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Num(num(&s[st..i])?));
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                let st = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                out.push(Tok::Ident(st, i - st));
            }
            _ => {
                let two = if i + 1 < b.len() { &s[i..i + 2] } else { "" };
                let op2 = ["<<", ">>", "<=", ">=", "==", "!=", "&&", "||"]
                    .iter()
                    .find(|o| two == **o);
                if let Some(o) = op2 {
                    out.push(Tok::Op(o));
                    i += 2;
                } else {
                    let one = &s[i..i + 1];
                    let op1 = ["+", "-", "*", "/", "%", "<", ">", "&", "|", "^", "!", "~"]
                        .iter()
                        .find(|o| one == **o);
                    let o = op1?;
                    out.push(Tok::Op(o));
                    i += 1;
                }
            }
        }
    }
    out.push(Tok::End);
    Some(out)
}

fn num(s: &str) -> Option<i64> {
    let s: String = s.chars().filter(|&c| c != '_').collect();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(h, 16).ok();
    }
    if let Some(h) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i64::from_str_radix(h, 2).ok();
    }
    if let Some(h) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return i64::from_str_radix(h, 8).ok();
    }
    s.parse().ok()
}

/// Infix binding powers `(l, r)`; `None` = not infix.
fn infix(op: &str) -> Option<(u8, u8)> {
    Some(match op {
        "||" => (10, 11),
        "&&" => (20, 21),
        "|" => (30, 31),
        "^" => (35, 36),
        "&" => (40, 41),
        "==" | "!=" => (45, 46),
        "<" | "<=" | ">" | ">=" => (50, 51),
        "<<" | ">>" => (55, 56),
        "+" | "-" => (60, 61),
        "*" | "/" | "%" => (70, 71),
        _ => return None,
    })
}

fn apply_bin(op: &str, a: i64, b: i64) -> Option<i64> {
    Some(match op {
        "+" => a.checked_add(b)?,
        "-" => a.checked_sub(b)?,
        "*" => a.checked_mul(b)?,
        "/" => {
            if b == 0 || (a == i64::MIN && b == -1) {
                return None;
            }
            a / b
        }
        "%" => {
            if b == 0 || (a == i64::MIN && b == -1) {
                return None;
            }
            a % b
        }
        "<<" => a.checked_shl(u32::try_from(b).ok()?)?,
        ">>" => a.checked_shr(u32::try_from(b).ok()?)?,
        "&" => a & b,
        "|" => a | b,
        "^" => a ^ b,
        "<" => i64::from(a < b),
        "<=" => i64::from(a <= b),
        ">" => i64::from(a > b),
        ">=" => i64::from(a >= b),
        "==" => i64::from(a == b),
        "!=" => i64::from(a != b),
        "&&" => i64::from(a != 0 && b != 0),
        "||" => i64::from(a != 0 || b != 0),
        _ => return None,
    })
}

struct P<'a> {
    t: &'a [Tok],
    s: &'a str,
    i: usize,
    vars: &'a [(&'a str, i64)],
}

impl P<'_> {
    fn peek(&self) -> Tok {
        self.t.get(self.i).copied().unwrap_or(Tok::End)
    }
    fn next(&mut self) -> Tok {
        let t = self.peek();
        self.i += 1;
        t
    }
    fn expr(&mut self, min_bp: u8, depth: usize) -> Option<i64> {
        if depth > 256 {
            return None;
        }
        let mut lhs = match self.next() {
            Tok::Num(v) => v,
            Tok::Ident(st, n) => {
                let name = &self.s[st..st + n];
                if self.peek() == Tok::LParen {
                    return self.call(name, depth);
                }
                self.vars
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| *v)?
            }
            Tok::Op(op) => {
                let a = self.expr(80, depth + 1)?;
                match op {
                    "-" => a.checked_neg()?,
                    "+" => a,
                    "~" => !a,
                    "!" => i64::from(a == 0),
                    _ => return None,
                }
            }
            Tok::LParen => {
                let v = self.expr(0, depth + 1)?;
                if self.next() != Tok::RParen {
                    return None;
                }
                v
            }
            _ => return None,
        };
        while let Tok::Op(op) = self.peek() {
            let Some((l, r)) = infix(op) else { break };
            if l < min_bp {
                break;
            }
            self.next();
            let rhs = self.expr(r, depth + 1)?;
            lhs = apply_bin(op, lhs, rhs)?;
        }
        Some(lhs)
    }
    fn call(&mut self, name: &str, depth: usize) -> Option<i64> {
        self.next(); // '('
        let mut args = Vec::new();
        if self.peek() != Tok::RParen {
            loop {
                args.push(self.expr(0, depth + 1)?);
                match self.next() {
                    Tok::Comma => continue,
                    Tok::RParen => break,
                    _ => return None,
                }
            }
        } else {
            self.next();
        }
        match (name, args.as_slice()) {
            ("min", [a, b]) => Some((*a).min(*b)),
            ("max", [a, b]) => Some((*a).max(*b)),
            ("abs", [a]) => a.checked_abs(),
            ("clamp", [x, lo, hi]) => Some((*x).clamp(*lo, *hi)),
            _ => None,
        }
    }
}

/// Evaluate `src` against `vars` (`(name, value)` pairs). Total: any
/// lexical, syntactic, or arithmetic failure yields `None`.
pub fn eval(src: &str, vars: &[(&str, i64)]) -> Option<i64> {
    let t = lex(src)?;
    let mut p = P {
        t: &t,
        s: src,
        i: 0,
        vars,
    };
    let v = p.expr(0, 0)?;
    if p.peek() != Tok::End {
        return None;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence_and_literals() {
        assert_eq!(eval("1 + 2 * 3", &[]), Some(7));
        assert_eq!(eval("(1 + 2) * 3", &[]), Some(9));
        assert_eq!(eval("1 << 4 | 2", &[]), Some(18));
        assert_eq!(eval("0x10 + 0b101 + 0o7", &[]), Some(28));
        assert_eq!(eval("10 / 3", &[]), Some(3));
        assert_eq!(eval("10 % 3", &[]), Some(1));
        assert_eq!(eval("-2 * 3", &[]), Some(-6));
        assert_eq!(eval("~0", &[]), Some(-1));
        assert_eq!(eval("!5", &[]), Some(0));
        assert_eq!(eval("!0", &[]), Some(1));
        assert_eq!(eval("2 * 3 % 4", &[]), Some(2));
        // C order: '<' binds tighter than '&' → (1 < 2) & 3 = 1.
        assert_eq!(eval("1 < 2 & 3", &[]), Some(1));
    }

    #[test]
    fn comparisons_and_logic() {
        assert_eq!(eval("3 > 2", &[]), Some(1));
        assert_eq!(eval("3 <= 2", &[]), Some(0));
        assert_eq!(eval("3 == 3", &[]), Some(1));
        assert_eq!(eval("3 != 3", &[]), Some(0));
        assert_eq!(eval("1 && 0", &[]), Some(0));
        assert_eq!(eval("1 || 0", &[]), Some(1));
        assert_eq!(eval("5 && 7", &[]), Some(1));
    }

    #[test]
    fn vars_and_calls() {
        assert_eq!(eval("hp - dmg", &[("hp", 5), ("dmg", 8)]), Some(-3));
        assert_eq!(eval("max(hp - dmg, 0)", &[("hp", 5), ("dmg", 8)]), Some(0));
        assert_eq!(eval("min(3, 4)", &[]), Some(3));
        assert_eq!(eval("abs(-9)", &[]), Some(9));
        assert_eq!(eval("clamp(5, 0, 3)", &[]), Some(3));
        assert_eq!(eval("clamp(-5, 0, 3)", &[]), Some(0));
        assert_eq!(eval("x", &[("x", 41)]), Some(41));
    }

    #[test]
    fn failures_are_total() {
        for bad in [
            "",
            "1 +",
            "*3",
            "(1",
            "1)",
            "1 2",
            "unknown_var",
            "1 / 0",
            "1 % 0",
            "min(1)",
            "min(1,2,3)",
            "foo(1)",
            "9223372036854775807 + 1",
            "-9223372036854775808 * -1",
            "1 << 64",
            "abs(-9223372036854775808)",
            "1 &&",
            "@",
        ] {
            assert_eq!(eval(bad, &[]), None, "{bad}");
        }
    }
}
