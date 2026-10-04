//! Vim syntax file (`syntax/*.vim`) parser.
//!
//! Detects Vim syntax definitions by `syntax keyword`/`syntax match`/
//! `syntax region`/`syntax cluster`/`syntax include`/`syntax sync`/
//! `syntax clear`/`highlight default link`/`hi def link`/`contains=`/
//! `contained`/`containedin=`/`matchgroup=`/`oneline`/`fold`/`skip=`/
//! `display`/`transparent`/`extend`/`keepend`/`nextgroup=`/`b:current_syntax`
//! directives.
//!
//! ```
//! let b = b"syntax keyword demoKw if else for\nsyntax match demoNum \"\\d\\+\"\nsyntax region demoStr start=+'+ end=+'+\nhighlight default link demoKw Keyword\nlet b:current_syntax = \"demo\"\n";
//! assert!(izanagi_kit::vimsyntax::detect(b));
//! let c = izanagi_kit::vimsyntax::Vimsyn::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed vim syntax summary.
#[derive(Debug, Clone)]
pub struct Vimsyn {
    /// Recognized directive occurrences.
    pub keys: usize,
    /// `syntax` commands (`syntax keyword`/`syntax match`/`syntax region`/`syntax cluster`/`syntax include`/`syntax sync`/`syntax clear`/`syntax case`/`syntax spell`/`syntax iskeyword`/`syntax conceal`/`syntax on`).
    pub syntax_keys: usize,
    /// Region options (`contains=`/`contained`/`containedin=`/`matchgroup=`/`oneline`/`fold`/`skip=`/`display`/`transparent`/`extend`/`keepend`/`nextgroup=`/`start=`/`end=`/`me=`/`he=`/`hs=`/`rs=`/`re=`/`lc=`/`excludenl`/`grouphere`/`groupthere`/`includenl`/`concealends`/`unspell`/`minlines=`/`maxlines=`).
    pub opt_keys: usize,
    /// Highlight/set keys (`highlight default link`/`hi def link`/`hi link`/`hi def`/`hi clear`/`hi! link`/`let b:current_syntax`/`setl iskeyword`/`runtime! syntax`/`unlet! b:current_syntax`/`if exists("b:current_syntax")`/`" Vim syntax file`).
    pub hi_keys: usize,
    /// `"` comment lines.
    pub comments: usize,
}

/// Syntax commands.
const SYNTAX_KEYS: &[&str] = &[
    "syntax keyword",
    "syntax match",
    "syntax region",
    "syntax cluster",
    "syntax include",
    "syntax syn\u{63}",
    "syntax clear",
    "syntax case",
    "syntax spell",
    "syntax iskeyword",
    "syn keyword",
    "syn match",
    "syn region",
    "syn cluster",
    "syn include",
    "syn syn\u{63}",
];

/// Region options.
const OPT_KEYS: &[&str] = &[
    "contains=",
    "containedin=",
    "contained",
    "matchgroup=",
    "oneline",
    "fold",
    "skip=",
    "display",
    "transparent",
    "extend",
    "keepend",
    "nextgroup=",
    "start=",
    "end=",
    "me=",
    "he=",
    "hs=",
    "rs=",
    "re=",
    "lc=",
    "excludenl",
    "grouphere",
    "groupthere",
    "includenl",
    "concealends",
    "unspell",
    "minlines=",
    "maxlines=",
];

/// Highlight/set keys.
const HI_KEYS: &[&str] = &[
    "highlight default link",
    "hi def link",
    "hi! link",
    "hi link",
    "hi def",
    "hi clear",
    "let b:current_syntax",
    "unlet! b:current_syntax",
    "setl iskeyword",
    "runtime! syntax",
    "b:current_syntax",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "syntax keyword",
    "syntax match",
    "syntax region",
    "syn keyword",
    "syn match",
    "syn region",
    "highlight default link",
    "hi def link",
    "b:current_syntax",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Vim syntax file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Vimsyn {
    /// Count categories in a syntax file. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            syntax_keys: 0,
            opt_keys: 0,
            hi_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim_start();
            if tr.starts_with('"') {
                c.comments += 1;
            }
        }
        for k in SYNTAX_KEYS {
            c.syntax_keys += t.matches(k).count();
        }
        for k in OPT_KEYS {
            c.opt_keys += t.matches(k).count();
        }
        for k in HI_KEYS {
            c.hi_keys += t.matches(k).count();
        }
        c.keys = c.syntax_keys + c.opt_keys + c.hi_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"\" Vim syntax file\nif exists(\"b:current_syntax\")\n  finish\nendif\nsyntax keyword demoKw if else for contained\nsyntax match demoNum \"\\d\\+\" display\nsyntax region demoStr start='\"' end='\"' contains=demoNum\nsyntax sync minlines=10\nhi def link demoKw Keyword\nhi def link demoNum Number\nlet b:current_syntax = \"demo\"\n";
        assert!(detect(b));
        let c = Vimsyn::parse(b).unwrap();
        assert!(c.comments >= 1);
        assert!(c.syntax_keys >= 3);
        assert!(c.opt_keys >= 3);
        assert!(c.hi_keys >= 3);
        assert!(c.keys >= 9);
    }

    #[test]
    fn rejects_vimrc() {
        assert!(!detect(b"set number\nset nowrap\n"));
        assert!(Vimsyn::parse(b"a = b\n").is_none());
    }
}
