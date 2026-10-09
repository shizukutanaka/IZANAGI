//! GNU nano `.nanorc` census.
//!
//! `set opt`/`unset opt` (`linenumbers`, `mouse`, `autoindent`,
//! `tabsize`, `tabstospaces`, `historylog`, `positionlog`,
//! `softwrap`, `smarthome`, `backup`, `backupdir`, `guidestripe`,
//! `minibar`, `indicator`, `zap`, `linenumbers`),
//! `bind key func menu`/`unbind`, `include "path"`,
//! `syntax "name" ["regex" ...]` blocks with
//! `color`/`icolor fg,bg "regex"`/`comment`/`header`/`magic`/
//! `linter`/`formatter`/`tabsize`/`extendsyntax`/`speller`,
//! `set multibuffer`/`set rcfile` etc.
//!
//! ```rust
//! let n = "set linenumbers\nset tabsize 4\ninclude /usr/share/nano/*.nanorc\n";
//! let c = izanagi_kit::nanorc::Nanorc::parse(n.as_bytes()).unwrap();
//! assert_eq!(c.sets, 2);
//! ```

use crate::textutil::strip_bom;
/// nanorc census.
#[derive(Debug, Clone)]
pub struct Nanorc {
    /// `set` statements.
    pub sets: usize,
    /// `unset` statements.
    pub unsets: usize,
    /// `bind`/`unbind` statements.
    pub binds: usize,
    /// `include` statements.
    pub includes: usize,
    /// `syntax`/`extendsyntax` statements.
    pub syntaxes: usize,
    /// `color`/`icolor` statements.
    pub colors: usize,
    /// Other recognised statements (`comment`/`header`/`magic`/`linter`/`formatter`/`tabgives`/`speller`/`make`/`execute`/`whereis`/`copytext`/`cut`/`zap`/`treatment`).
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OTHER_HEADS: &[&str] = &[
    "comment",
    "header",
    "magic",
    "linter",
    "formatter",
    "tabgives",
    "speller",
    "execute",
    "treatment",
    "menucolor",
    "titlecolor",
    "statuscolor",
    "selectedcolor",
    "stripecolor",
    "scrollercolor",
    "numbercolor",
    "keycolor",
    "functioncolor",
    "spotlightcolor",
    "miniinfobarcolor",
    "emergencyaccessed",
    "make",
    "whereis",
    "copytext",
    "cut",
    "zap",
    "wordbounds",
    "brackets",
    "whitespace",
    "punct",
    "quotestr",
];

/// Detect nanorc content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        let head = s.split(' ').next().unwrap_or("");
        if head == "set"
            || head == "unset"
            || head == "bind"
            || head == "include"
            || head == "syntax"
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl Nanorc {
    /// Census a nanorc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sets: 0,
            unsets: 0,
            binds: 0,
            includes: 0,
            syntaxes: 0,
            colors: 0,
            named: 0,
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
            let head = s.split(' ').next().unwrap_or("");
            match head {
                "set" => c.sets += 1,
                "unset" => c.unsets += 1,
                "bind" | "unbind" => c.binds += 1,
                "include" => c.includes += 1,
                "syntax" | "extendsyntax" => c.syntaxes += 1,
                "color" | "icolor" => c.colors += 1,
                _ => {
                    if OTHER_HEADS.contains(&head) {
                        c.named += 1;
                    }
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nanorc() {
        let b = b"set linenumbers\nset mouse\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_nanorc() {
        let b = concat!(
            "# nanorc\n",
            "set linenumbers\n",
            "set mouse\n",
            "set autoindent\n",
            "set tabsize 4\n",
            "set tabstospaces\n",
            "set historylog\n",
            "set positionlog\n",
            "set softwrap\n",
            "set smarthome\n",
            "set indicator\n",
            "unset backup\n",
            "bind ^C copy main\n",
            "bind ^V paste all\n",
            "bind M-| speller main\n",
            "unbind ^K main\n",
            "include /usr/share/nano/*.nanorc\n",
            "include ~/.nano/extra.nanorc\n",
            "syntax \"mylang\" \"\\.myl$\"\n",
            "color green \"[a-z]+\"\n",
            "icolor red \"ERROR\"\n",
            "comment \"#\"\n",
            "header \"^#!\"\n",
            "magic \"binary data\"\n",
            "linter mylint -f\n",
            "formatter myfmt --stdin\n",
            "tabgives \"    \"\n",
            "speller \"aspell -x -c\"\n",
            "setguidestripe 80\n",
        );
        let c = Nanorc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 10);
        assert_eq!(c.unsets, 1);
        assert_eq!(c.binds, 4);
        assert_eq!(c.includes, 2);
        assert_eq!(c.syntaxes, 1);
        assert_eq!(c.colors, 2);
        assert!(c.named >= 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
