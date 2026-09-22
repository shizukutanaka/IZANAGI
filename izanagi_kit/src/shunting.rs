//! Shunting-yard — Dijkstra's 1961 infix→postfix conversion plus a
//! strict `i64` postfix evaluator. Used for rule expressions, damage
//! formulas, and script bindings that must evaluate identically on
//! every peer.
//!
//! Grammar: `expr := term (op term)*`, `term := Num | Neg term |
//! '(' expr ')'`. Binary `+ - * / %`, unary [`Op::Neg`]
//! (right-associative, binds tighter than `*`), parentheses.
//! Division and remainder **truncate toward zero** like Rust;
//! `/ 0` and `% 0` yield `None`, as does any `i64` overflow —
//! evaluation is fail-closed, never panics, and is a pure function
//! of the token stream.
//!
//! ```
//! use izanagi_kit::shunting::{eval, Op, Tok};
//! use Op::{Add, Mul, Neg, Sub};
//! use Tok::{Num, Op as O, LP, RP};
//! // 2 + 3 * -4 - (1 + 1)
//! let t = [Num(2), O(Add), Num(3), O(Mul), O(Neg), Num(4),
//!          O(Sub), LP, Num(1), O(Add), Num(1), RP];
//! assert_eq!(eval(&t), Some(-12));
//! ```

/// An operator token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// `a + b`
    Add,
    /// `a - b`
    Sub,
    /// `a * b`
    Mul,
    /// `a / b` — truncating integer division; `/ 0` fails.
    Div,
    /// `a % b` — truncating remainder; `% 0` fails.
    Rem,
    /// Unary `-a` — right-associative, highest precedence.
    Neg,
}

/// A token of the infix input / postfix output stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    /// Integer literal.
    Num(i64),
    /// Operator.
    Op(Op),
    /// `(` (infix only — never appears in postfix).
    LP,
    /// `)` (infix only).
    RP,
}

enum Stack {
    Op(Op),
    LP,
}

fn prec(op: Op) -> u8 {
    match op {
        Op::Add | Op::Sub => 1,
        Op::Mul | Op::Div | Op::Rem => 2,
        Op::Neg => 3,
    }
}

/// Convert an infix token stream to postfix — `None` on mismatched
/// parentheses or a malformed stream (empty, leading/trailing
/// binary op, adjacent operands, `(` in operand position…). The
/// conversion checks *structure* (operand/operator alternation and
/// paren depth), so a `Some` result is always evaluable.
pub fn shunting_yard(tokens: &[Tok]) -> Option<Vec<Tok>> {
    if tokens.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(tokens.len());
    let mut ops: Vec<Stack> = Vec::new();
    let mut parens = 0usize;
    // The grammar alternates operand/operator; Neg counts as
    // operand-position (it prefixes an operand).
    let mut want_operand = true;
    for &t in tokens {
        match t {
            Tok::Num(_) => {
                if !want_operand {
                    return None; // adjacent operands
                }
                out.push(t);
                want_operand = false;
            }
            Tok::Op(Op::Neg) => {
                if !want_operand {
                    return None; // unary only
                }
                ops.push(Stack::Op(Op::Neg));
            }
            Tok::Op(op) => {
                if want_operand {
                    return None; // binary op where a term belongs
                }
                // Left-associative: pop while top has ≥ precedence.
                while let Some(Stack::Op(top)) = ops.last() {
                    if prec(*top) < prec(op) {
                        break;
                    }
                    out.push(Tok::Op(*top));
                    ops.pop();
                }
                ops.push(Stack::Op(op));
                want_operand = true;
            }
            Tok::LP => {
                if !want_operand {
                    return None; // ")(" or "2("
                }
                parens += 1;
                ops.push(Stack::LP);
            }
            Tok::RP => {
                if want_operand || parens == 0 {
                    return None; // empty parens or unbalanced
                }
                parens -= 1;
                while let Some(s) = ops.pop() {
                    match s {
                        Stack::Op(op) => out.push(Tok::Op(op)),
                        Stack::LP => break,
                    }
                }
                want_operand = false;
            }
        }
    }
    if parens != 0 || want_operand {
        return None;
    }
    while let Some(s) = ops.pop() {
        match s {
            Stack::Op(op) => out.push(Tok::Op(op)),
            Stack::LP => return None, // can't happen: parens == 0
        }
    }
    Some(out)
}

/// Evaluate a postfix stream — `None` on stack underflow, a
/// non-single final stack, division/rem by zero, or `i64` overflow.
/// Streams from [`shunting_yard`] never underflow, but callers may
/// compose postfix directly.
pub fn eval_rpn(postfix: &[Tok]) -> Option<i64> {
    let mut st: Vec<i64> = Vec::with_capacity(16);
    for &t in postfix {
        match t {
            Tok::Num(v) => st.push(v),
            Tok::Op(Op::Neg) => {
                let a = st.pop()?;
                st.push(a.checked_neg()?);
            }
            Tok::Op(op) => {
                let b = st.pop()?;
                let a = st.pop()?;
                let v = match op {
                    Op::Add => a.checked_add(b)?,
                    Op::Sub => a.checked_sub(b)?,
                    Op::Mul => a.checked_mul(b)?,
                    Op::Div => {
                        if b == 0 || (a == i64::MIN && b == -1) {
                            return None;
                        }
                        a / b
                    }
                    Op::Rem => {
                        if b == 0 || (a == i64::MIN && b == -1) {
                            return None;
                        }
                        a % b
                    }
                    Op::Neg => return None, // handled above
                };
                st.push(v);
            }
            Tok::LP | Tok::RP => return None,
        }
    }
    if st.len() == 1 {
        Some(st[0])
    } else {
        None
    }
}

/// Infix → value in one call — `shunting_yard` then `eval_rpn`.
pub fn eval(tokens: &[Tok]) -> Option<i64> {
    eval_rpn(&shunting_yard(tokens)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use Op::*;
    use Tok::{Num, Op as O, LP, RP};

    /// Independent oracle: a recursive-descent evaluator of the same
    /// grammar over a *tree-shaped* random generator — different
    /// algorithm than the operator stack under test.
    struct P<'a> {
        t: &'a [Tok],
        i: usize,
    }
    impl<'a> P<'a> {
        fn expr(&mut self) -> Option<i64> {
            let mut v = self.term()?;
            loop {
                let op = match self.t.get(self.i) {
                    Some(O(Add)) => Add,
                    Some(O(Sub)) => Sub,
                    _ => break,
                };
                self.i += 1;
                let b = self.term()?;
                v = if op == Add {
                    v.checked_add(b)
                } else {
                    v.checked_sub(b)
                }?;
            }
            Some(v)
        }
        fn term(&mut self) -> Option<i64> {
            let mut v = self.unary()?;
            loop {
                let op = match self.t.get(self.i) {
                    Some(O(Mul)) => Mul,
                    Some(O(Div)) => Div,
                    Some(O(Rem)) => Rem,
                    _ => break,
                };
                self.i += 1;
                let b = self.unary()?;
                v = match op {
                    Mul => v.checked_mul(b)?,
                    Div => {
                        if b == 0 || (v == i64::MIN && b == -1) {
                            return None;
                        }
                        v / b
                    }
                    _ => {
                        if b == 0 || (v == i64::MIN && b == -1) {
                            return None;
                        }
                        v % b
                    }
                };
            }
            Some(v)
        }
        fn unary(&mut self) -> Option<i64> {
            match self.t.get(self.i) {
                Some(O(Neg)) => {
                    self.i += 1;
                    self.unary()?.checked_neg()
                }
                Some(Num(v)) => {
                    self.i += 1;
                    Some(*v)
                }
                Some(LP) => {
                    self.i += 1;
                    let v = self.expr()?;
                    if self.t.get(self.i) != Some(&RP) {
                        return None;
                    }
                    self.i += 1;
                    Some(v)
                }
                _ => None,
            }
        }
    }
    fn oracle(t: &[Tok]) -> Option<i64> {
        let mut p = P { t, i: 0 };
        let v = p.expr()?;
        (p.i == t.len()).then_some(v)
    }

    /// Random well-formed token stream generator (grammar-directed).
    fn gen(rng: &mut SplitMix64, t: &mut Vec<Tok>, depth: usize) {
        // operand position — compound forms stop at depth 3
        let r = rng.below(4);
        match (r, depth < 3) {
            (0, true) => {
                t.push(O(Neg));
                gen(rng, t, depth + 1);
            }
            (1, true) => {
                t.push(LP);
                expr(rng, t, depth + 1);
                t.push(RP);
            }
            _ => {
                t.push(Num(rng.below(20) as i64 - 10));
            }
        }
        expr_tail(rng, t, depth);
    }
    fn expr(rng: &mut SplitMix64, t: &mut Vec<Tok>, depth: usize) {
        gen(rng, t, depth);
    }
    fn expr_tail(rng: &mut SplitMix64, t: &mut Vec<Tok>, depth: usize) {
        while rng.below(3) > 0 && depth < 3 {
            let op = [Add, Sub, Mul, Div, Rem][rng.below(5) as usize];
            t.push(O(op));
            gen(rng, t, depth + 1);
        }
    }

    #[test]
    fn eval_matches_descent_oracle() {
        let mut rng = SplitMix64::new(0x5B7C);
        for _ in 0..2000 {
            let mut t = Vec::new();
            gen(&mut rng, &mut t, 0);
            assert_eq!(eval(&t), oracle(&t), "tokens {t:?}");
        }
    }

    #[test]
    fn precedence_associativity_and_parens() {
        use Tok::{Num as N, Op as P};
        // 2 + 3*4 = 14, not 20.
        assert_eq!(eval(&[N(2), P(Add), N(3), P(Mul), N(4)]), Some(14));
        // left assoc: 10 - 3 - 2 = 5.
        assert_eq!(eval(&[N(10), P(Sub), N(3), P(Sub), N(2)]), Some(5));
        // (10 - 3) - 2 vs 10 - (3 - 2)
        assert_eq!(eval(&[LP, N(10), P(Sub), N(3), RP, P(Sub), N(2)]), Some(5));
        assert_eq!(eval(&[N(10), P(Sub), LP, N(3), P(Sub), N(2), RP]), Some(9));
        // unary binds tighter: -3 * -3 = 9, --5 = 5.
        assert_eq!(eval(&[P(Neg), N(3), P(Mul), P(Neg), N(3)]), Some(9));
        assert_eq!(eval(&[P(Neg), P(Neg), N(5)]), Some(5));
        // truncating div/rem: -7 / 2 = -3, -7 % 2 = -1.
        assert_eq!(eval(&[P(Neg), N(7), P(Div), N(2)]), Some(-3));
        assert_eq!(eval(&[P(Neg), N(7), P(Rem), N(2)]), Some(-1));
    }

    #[test]
    fn malformed_and_divzero_fail_closed() {
        use Tok::{Num as N, Op as P};
        assert_eq!(eval(&[]), None);
        assert_eq!(eval(&[P(Add), N(1)]), None); // leading binary op
        assert_eq!(eval(&[N(1), P(Add)]), None); // trailing op
        assert_eq!(eval(&[N(1), N(2)]), None); // adjacent operands
        assert_eq!(eval(&[N(1), P(Add), LP, N(2)]), None); // unclosed
        assert_eq!(eval(&[N(1), RP]), None); // extra close
        assert_eq!(eval(&[LP, RP]), None); // empty parens
        assert_eq!(eval(&[N(1), LP, N(2), RP]), None); // "1(2)"
        assert_eq!(eval(&[N(1), P(Neg), N(2)]), None); // binary-position Neg
                                                       // div / rem by zero
        assert_eq!(eval(&[N(1), P(Div), N(0)]), None);
        assert_eq!(eval(&[N(1), P(Rem), N(0)]), None);
        // overflow
        assert_eq!(eval(&[N(i64::MAX), P(Add), N(1)]), None);
        assert_eq!(eval(&[P(Neg), N(i64::MIN)]), None);
        assert_eq!(eval(&[N(i64::MIN), P(Div), P(Neg), N(1)]), None);
    }

    #[test]
    fn rpn_output_shape_and_direct_eval() {
        use Tok::{Num as N, Op as P};
        // 2 + 3 * 4 → [2 3 4 * +]
        let rpn = shunting_yard(&[N(2), P(Add), N(3), P(Mul), N(4)]).unwrap();
        assert_eq!(rpn, vec![N(2), N(3), N(4), P(Mul), P(Add)]);
        assert_eq!(eval_rpn(&rpn), Some(14));
        // direct malformed postfix
        assert_eq!(eval_rpn(&[N(1), P(Add)]), None); // underflow
        assert_eq!(eval_rpn(&[N(1), N(2)]), None); // leftover
        assert_eq!(eval_rpn(&[LP]), None);
        // parens alone are not an expression
        assert_eq!(eval_rpn(&[]), None);
    }
}
