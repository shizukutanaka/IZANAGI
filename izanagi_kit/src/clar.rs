//! Clarity (Stacks) smart-contract source (`.clar`) parser.
//!
//! Detects Clarity by `(define-*` top-level forms, and counts public/
//! read-only/private functions, data maps/vars/constants, traits,
//! token definitions, `contract-call?`, `ok`/`err` and `;;` comments.
//!
//! ```
//! use izanagi_kit::clar::Clar;
//! let src = b"(define-data-var counter uint u0)\n(define-public (inc)\n    (ok u1))\n";
//! assert!(izanagi_kit::clar::detect(src));
//! let c = Clar::parse(src).unwrap();
//! assert_eq!(c.data_vars, 1);
//! assert_eq!(c.public, 1);
//! ```

/// Parsed census of a Clarity source file.
#[derive(Debug, Clone)]
pub struct Clar {
    /// `(define-public` functions.
    pub public: usize,
    /// `(define-read-only` functions.
    pub read_only: usize,
    /// `(define-private` functions.
    pub private: usize,
    /// `(define-map` entries.
    pub maps: usize,
    /// `(define-data-var` variables.
    pub data_vars: usize,
    /// `(define-constant` values.
    pub constants: usize,
    /// `(define-trait` declarations.
    pub traits: usize,
    /// `(impl-trait` uses.
    pub impl_traits: usize,
    /// `(use-trait` uses.
    pub use_traits: usize,
    /// `(define-fungible-token` / `(define-non-fungible-token`.
    pub tokens: usize,
    /// `(contract-call?` cross-contract calls.
    pub calls: usize,
    /// `(ok ` / `(err ` returns.
    pub results: usize,
    /// Other `(define-` forms.
    pub other_defines: usize,
    /// `;;` comment lines.
    pub comments: usize,
    /// Total `(` forms (depth-agnostic).
    pub forms: usize,
}

/// Returns `true` when `b` looks like Clarity source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("(define-") && t.contains('(') && t.contains(')')
}

impl Clar {
    /// Counts Clarity constructs; `None` when [`detect`] fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = core::str::from_utf8(b).ok()?;
        let mut c = Self {
            public: t.matches("(define-public").count(),
            read_only: t.matches("(define-read-only").count(),
            private: t.matches("(define-private").count(),
            maps: t.matches("(define-map").count(),
            data_vars: t.matches("(define-data-var").count(),
            constants: t.matches("(define-constant").count(),
            traits: t.matches("(define-trait").count(),
            impl_traits: t.matches("(impl-trait").count(),
            use_traits: t.matches("(use-trait").count(),
            tokens: t.matches("(define-fungible-token").count()
                + t.matches("(define-non-fungible-token").count(),
            calls: t.matches("(contract-call?").count(),
            results: t.matches("(ok ").count() + t.matches("(err ").count(),
            other_defines: 0,
            comments: t.matches(";;").count(),
            forms: t.matches('(').count(),
        };
        let defined = c.public
            + c.read_only
            + c.private
            + c.maps
            + c.data_vars
            + c.constants
            + c.traits
            + c.tokens;
        let all_defines = t.matches("(define-").count();
        c.other_defines = all_defines.saturating_sub(defined);
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &[u8] = b"(define-data-var counter uint u0)\n(define-public (inc)\n    (ok u1))\n";

    #[test]
    fn detect_works() {
        assert!(detect(SRC));
        assert!(!detect(b"(+ 1 2)"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses() {
        let c = Clar::parse(SRC).unwrap();
        assert_eq!(c.data_vars, 1);
        assert_eq!(c.public, 1);
        assert_eq!(c.results, 1);
    }

    #[test]
    fn counts_kinds() {
        let s = b";; head\n(define-constant k u1)\n(define-map m principal uint)\n(define-trait t ((f () (response uint uint))))\n(impl-trait .t)\n(use-trait u .u)\n(define-fungible-token tok)\n(define-read-only (r) (ok u1))\n(define-private (p) (err u0))\n(define-public (x) (contract-call? .c f))\n";
        let c = Clar::parse(s).unwrap();
        assert_eq!(c.constants, 1);
        assert_eq!(c.maps, 1);
        assert_eq!(c.traits, 1);
        assert_eq!(c.impl_traits, 1);
        assert_eq!(c.use_traits, 1);
        assert_eq!(c.tokens, 1);
        assert_eq!(c.read_only, 1);
        assert_eq!(c.private, 1);
        assert_eq!(c.calls, 1);
        assert!(c.results >= 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects() {
        assert!(Clar::parse(b"plain text").is_none());
        assert!(Clar::parse(b"").is_none());
    }
}
