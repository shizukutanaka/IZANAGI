//! Nix expression language (`.nix`) — attrset `{ … }`, `let … in`, `with pkgs;`,
//! `builtins.…`, function `arg:`/`{args}:` lambdas, and `rec` attrsets.
//!
//! ```
//! let d = b"{ pkgs ? import <nixpkgs> {} }:\nlet version = \"1\x2e0\"; in\npkgs.stdenv.mkDerivation rec {\n  pname = \"demo\";\n  inherit version;\n}\n";
//! let n = izanagi_kit::nix::parse(d).unwrap();
//! assert!(n.lambda);
//! assert_eq!(n.lets, 1);
//! assert_eq!(n.bindings, 2); // `pname =` + `inherit version;`
//! assert!(izanagi_kit::nix::detect(d));
//! ```

/// A parsed Nix expression summary.
#[derive(Debug, Clone)]
pub struct Nix {
    /// `let … in` blocks.
    pub lets: usize,
    /// `rec` attrsets.
    pub recs: usize,
    /// `with expr;` statements.
    pub withs: usize,
    /// `inherit (a) b c;` / `inherit b;` statements.
    pub inherits: usize,
    /// `builtins.xxx` references.
    pub builtins: usize,
    /// `import`, `map`, `filter`, `derivation`, `mkDerivation`, `fetchurl` calls.
    pub known_calls: usize,
    /// lambda parameters (`x:`/`{a,b}:` patterns) — `true` when one is present.
    pub lambda: bool,
    /// Total `key = value;` bindings inside attrsets/lets.
    pub bindings: usize,
    /// `if … then … else` expressions.
    pub ifs: usize,
    /// Comments (`#` lines and `#` inline comments).
    pub comments: usize,
}

fn strip_comment(line: &str) -> &str {
    // `#` starts a comment (Nix also has /* */ but a line scan is fine here)
    line.split('#').next().unwrap_or(line)
}

fn count_kw(t: &str, kw: &str) -> usize {
    let mut n = 0;
    let mut off = 0;
    while let Some(i) = t[off..].find(kw) {
        let j = off + i + kw.len();
        let before_ok = off + i == 0
            || !t.as_bytes()[off + i - 1].is_ascii_alphanumeric()
                && t.as_bytes()[off + i - 1] != b'_';
        let after_ok = match t.as_bytes().get(j) {
            None => true,
            Some(&c) => !c.is_ascii_alphanumeric() && c != b'_',
        };
        if before_ok && after_ok {
            n += 1;
        }
        off = j;
    }
    n
}

/// Detects Nix: `let … in`, `with …;`, `mkDerivation`/`derivation`, or a `{…}:` lambda head.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    count_kw(t, "let") > 0 && count_kw(t, "in") > 0
        || count_kw(t, "with") > 0
        || t.contains("mkDerivation")
        || t.contains("stdenv")
        || t.contains("<nixpkgs>")
        || (t.trim_start().starts_with('{') && t.contains("}:"))
}

/// Parses a Nix expression; `None` on non-UTF-8 or no Nix markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Nix> {
    if !detect(b) {
        return None;
    }
    let t = std::str::from_utf8(b).ok()?;
    let mut s = Nix {
        lets: count_kw(t, "let"),
        recs: count_kw(t, "rec"),
        withs: count_kw(t, "with"),
        inherits: count_kw(t, "inherit"),
        builtins: t.match_indices("builtins.").count(),
        known_calls: 0,
        lambda: false,
        bindings: 0,
        ifs: 0,
        comments: 0,
    };
    for k in [
        "import",
        "map",
        "filter",
        "derivation",
        "mkDerivation",
        "fetchurl",
        "mkShell",
        "callPackage",
        "builtins",
    ] {
        s.known_calls += count_kw(t, k);
    }
    s.ifs = count_kw(t, "if") + count_kw(t, "then") + count_kw(t, "else");
    s.ifs /= 3; // crude: triple keyword hits ≈ one expression
    for line in t.lines() {
        let l = strip_comment(line);
        if raw_needs_comment(&line) {
            s.comments += 1;
        }
        let l = l.trim();
        // `ident = expr;` bindings
        if l.ends_with(';') && l.contains('=') {
            let before = l.split('=').next().unwrap_or("").trim();
            if !before.is_empty()
                && before
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
                && !l.contains("==")
            {
                s.bindings += 1;
            }
        }
        if l.ends_with(':') || (l.contains(':') && l.trim_end().ends_with(':')) {
            s.lambda = true;
        }
    }
    // `inherit` lines also count as bindings of the attrset
    s.bindings += s.inherits;
    if t.trim_start().starts_with('{') && t.contains("}:") {
        s.lambda = true;
    }
    Some(s)
}

fn raw_needs_comment(line: &&str) -> bool {
    line.contains('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"# a package\n{ pkgs ? import <nixpkgs> {} }:\nlet\n  version = \"1\x2e0\";\n  src = fetchurl { url = \"u\"; };\nin\npkgs.stdenv.mkDerivation rec {\n  pname = \"demo\";\n  inherit version;\n  buildInputs = with pkgs; [ zlib ];\n}\n";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert!(s.lambda);
        assert_eq!(s.lets, 1);
        assert_eq!(s.recs, 1);
        assert_eq!(s.withs, 1);
        assert_eq!(s.inherits, 1);
        assert!(s.known_calls >= 2);
        assert!(s.bindings >= 3);
        assert_eq!(s.comments, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"{ lib }: lib.id 1"));
        assert!(detect(b"pkgs.stdenv.mkDerivation { }"));
        assert!(!detect(b"{ \"a\": 1 }"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"select * from t").is_none());
    }
}
