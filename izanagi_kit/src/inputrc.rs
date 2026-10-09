//! GNU Readline `.inputrc` census.
//!
//! `set var val` (`editing-mode`, `keymap`, `bell-style`,
//! `show-all-if-ambiguous`, `completion-ignore-case`,
//! `colored-stats`, `colored-completion-prefix`,
//! `menu-complete-display-prefix`, `mark-symlinked-directories`,
//! `visible-stats`, `page-completions`, `completion-query-items`,
//! `history-size`, `echo-control-characters`,
//! `enable-keypad`, `expand-tilde`, `convert-meta`,
//! `input-meta`, `output-meta`, `horizontal-scroll-mode`,
//! `mark-modified-lines`, `prefer-visible-bell`,
//! `print-completions-horizontally`, `show-all-if-unmodified`,
//! `skip-completed-text`, `completion-map-case`,
//! `enable-active-region`, `blink-matching-paren`),
//! `$if`/`$else`/`$endif`/`$include` conditionals
//! (`$if mode=vi`, `$if Bash`, `$if term=xterm`), and
//! `"keyseq": function` bindings (`"\e[A":
//! history-search-backward`, `"\C-l": clear-screen`,
//! `TAB: menu-complete`, `Control-x: "…"` macros).
//!
//! ```rust
//! let i = "set editing-mode vi\nset completion-ignore-case on\n\"\\e[A\": history-search-backward\n";
//! let c = izanagi_kit::inputrc::Inputrc::parse(i.as_bytes()).unwrap();
//! assert_eq!(c.sets, 2);
//! ```

use crate::textutil::strip_bom;
/// inputrc census.
#[derive(Debug, Clone)]
pub struct Inputrc {
    /// `set` statements.
    pub sets: usize,
    /// `$if`/`$else`/`$endif` conditionals.
    pub ifs: usize,
    /// `"seq": func` / `key: func` bindings.
    pub bindings: usize,
    /// `$include` lines.
    pub includes: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Detect inputrc content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut strong = 0usize;
    let mut soft = 0usize;
    let mut escaped = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        // `$if`/`$endif`/`$else`/`$include` conditionals
        if s.starts_with('$') {
            strong += 1;
            continue;
        }
        // `set var value` — readline variable names carry `-` or are
        // one of the well-known single words
        if let Some(rest) = s.strip_prefix("set ") {
            let var = rest.split_whitespace().next().unwrap_or("");
            // readline variable names are `dashed-words` (or `keymap`);
            // a leading-`-` word is a shell option (`set -e`), not a var
            if !var.starts_with('-') && (var.contains('-') || var == "keymap") {
                strong += 1;
            } else if !var.is_empty()
                && !var.starts_with('-')
                && var.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            {
                soft += 1;
            }
            continue;
        }
        // `"key": fn` / `"key": "macro"` bindings — `\\`-escaped keys
        // (`\\e[`, `\\C-`) are readline-exclusive
        if s.starts_with('"')
            && s.contains("\": ")
            && s.chars().all(|c| !matches!(c, '{' | '}' | '[' | ']'))
        {
            let key = &s[1..s.find("\": ").unwrap_or(1)];
            let rest = s[s.find("\": ").unwrap_or(s.len()) + 3..].trim();
            let fnish = !rest.is_empty()
                && rest
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || matches!(c, '-' | '_' | '"'));
            if key.contains('\\') {
                escaped += 1;
                strong += 1;
            } else if fnish {
                soft += 1;
            }
        }
    }
    strong >= 1 || (soft >= 2 && escaped >= 1) || soft >= 3
}

impl Inputrc {
    /// Census an inputrc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sets: 0,
            ifs: 0,
            bindings: 0,
            includes: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("set ") {
                c.sets += 1;
                continue;
            }
            if s.starts_with("$if") || s.starts_with("$else") || s.starts_with("$endif") {
                c.ifs += 1;
                continue;
            }
            if s.starts_with("$include") {
                c.includes += 1;
                continue;
            }
            if s.contains(": ") && (s.starts_with('"') || s.contains(':')) {
                c.bindings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_inputrc() {
        let b = b"set editing-mode vi\nset completion-ignore-case on\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_inputrc() {
        let b = concat!(
            "# inputrc\n",
            "set editing-mode vi\n",
            "set keymap vi\n",
            "set bell-style none\n",
            "set completion-ignore-case on\n",
            "set show-all-if-ambiguous on\n",
            "set colored-stats on\n",
            "set colored-completion-prefix on\n",
            "set mark-symlinked-directories on\n",
            "set page-completions off\n",
            "set completion-query-items 350\n",
            "set enable-active-region on\n",
            "set blink-matching-paren on\n",
            "$if mode=vi\n",
            "  \"\\e[A\": history-search-backward\n",
            "  \"\\e[B\": history-search-forward\n",
            "  \"\\C-l\": clear-screen\n",
            "$else\n",
            "  TAB: menu-complete\n",
            "  \"\\e[Z\": menu-complete-backward\n",
            "  Shift-Tab: \"\\e[Z~\"\n",
            "  Control-x: \"bash -c 'x'\\e\\C-e\\C-m\\C-m\"\n",
            "$endif\n",
            "$if Bash\n",
            "  Space: magic-space\n",
            "$endif\n",
            "$include ~/.inputrc.local\n",
        );
        let c = Inputrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 12);
        assert_eq!(c.ifs, 5);
        assert_eq!(c.bindings, 8);
        assert_eq!(c.includes, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
