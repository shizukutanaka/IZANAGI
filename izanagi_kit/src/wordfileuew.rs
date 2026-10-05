//! UltraEdit syntax wordfile (`.uew`) parser.
//!
//! Detects UE wordfiles by their `/L1"Name"` language header, `/Nocase`/
//! `/Case`/`/Disable Line Comment`/`Line Comment`/`Block Comment`/
//! `/Escape Char`/`/String Chars`/`/Delimiter Characters`/`/Function String`/
//! `/C1"Color"` color-group directives and `**` keyword-group separators.
//!
//! ```
//! let b = b"/L1\"Demo\" Nocase Line Comment = // Escape Char = ~ String Chars = \"\n/C1\"Keywords\"\nif else for while\n**\n/C2\"Numbers\"\n123\n";
//! assert!(izanagi_kit::wordfileuew::detect(b));
//! let c = izanagi_kit::wordfileuew::Uew::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed .uew summary.
#[derive(Debug, Clone)]
pub struct Uew {
    /// Recognized directive/group occurrences.
    pub keys: usize,
    /// `/Ln` language headers plus lang directives (`/Nocase`/`/Case`/`/Line Comment`/`/Block Comment`/`/Disable Line Comment`/`/Disable Block Comment`/`/Escape Char`/`/String Chars`/`/File Extensions`/`/File Names`/`/Auto Mark Corresponding Strings`/`/Function String`/`/Indent Strings`/`/Unindent Strings`/`/Open Fold Strings`/`/Close Fold Strings`/`/Ignore Fold Strings`/`/Open Brace Strings`/`/Close Brace Strings`/`/EnableCFaaS`/`/EnableFunctions`).
    pub header_keys: usize,
    /// `/Cn` color groups (`/C` + digit prefix lines) plus `/Colors` family directives.
    pub group_keys: usize,
    /// Delimiter/other directives (`/Delimiter Characters`/`/Quotation Mark`/`/Member Select String`/`/Continue String`/`/Sub Language`/`/On Backspace`/`/Ignore Keyword`/`/Word Wrap`/`/Ignore String`/`/Marker Characters`/`/Command String`/`/Export Function`).
    pub other_keys: usize,
    /// `**` group separator lines.
    pub separators: usize,
    /// `#`/empty-line comments.
    pub comments: usize,
}

/// Header directives.
const HEADER_KEYS: &[&str] = &[
    "/L1",
    "/L2",
    "/L3",
    "Nocase",
    "Line Comment",
    "Block Comment",
    "Disable Line Comment",
    "Disable Block Comment",
    "Escape Char",
    "String Chars",
    "File Extensions",
    "File Names",
    "Function String",
    "Indent Strings",
    "Unindent Strings",
    "Open Fold Strings",
    "Close Fold Strings",
    "Ignore Fold Strings",
    "Open Brace Strings",
    "Close Brace Strings",
    "EnableCFaaS",
    "EnableFunctions",
    "Auto Mark Corresponding Strings",
];

/// `/Colors` family directives (`/C` + digit group headers are counted per line).
const GROUP_KEYS: &[&str] = &["/Colors"];

/// Other directives.
const OTHER_KEYS: &[&str] = &[
    "Delimiter Characters",
    "Delimiters",
    "Quotation Mark",
    "Member Select String",
    "Continue String",
    "Sub Language",
    "On Backspace",
    "Ignore Keyword",
    "Word Wrap",
    "Ignore String",
    "Marker Characters",
    "Command String",
    "Export Function",
];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "/L1",
    "Nocase",
    "Line Comment",
    "Block Comment",
    "String Chars",
    "Delimiter",
    "/C1",
    "/C2",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

fn is_group_line(tr: &str) -> bool {
    tr.len() >= 3 && tr.starts_with("/C") && tr.as_bytes()[2].is_ascii_digit()
}

/// Detect an UltraEdit wordfile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let groups = t.lines().filter(|l| is_group_line(l.trim())).count();
    hits + usize::from(groups > 0) >= 2
}

impl Uew {
    /// Count categories in a .uew file. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            header_keys: 0,
            group_keys: 0,
            other_keys: 0,
            separators: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr == "**" {
                c.separators += 1;
            }
            if is_group_line(tr) {
                c.group_keys += 1;
            }
            if tr.starts_with('#') || tr.is_empty() {
                c.comments += 1;
            }
        }
        for k in HEADER_KEYS {
            c.header_keys += t.matches(k).count();
        }
        for k in GROUP_KEYS {
            c.group_keys += t.matches(k).count();
        }
        for k in OTHER_KEYS {
            c.other_keys += t.matches(k).count();
        }
        c.keys = c.header_keys + c.group_keys + c.other_keys + c.separators;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"/L1\"Demo\" Nocase Line Comment = // Block Comment On = /* Block Comment Off = */ Escape Char = ~ String Chars = \"' Delimiters = ~!@$%^&*()=+|\\{}[]:;\",.<>/?\n/C1\"Keywords\"\nif else for while\n**\n/C2\"Numbers\"\n123\n";
        assert!(detect(b));
        let c = Uew::parse(b).unwrap();
        assert!(c.header_keys >= 3);
        assert!(c.group_keys >= 2);
        assert_eq!(c.separators, 1);
        assert!(c.keys >= 6);
    }

    #[test]
    fn counts_high_color_groups() {
        let b = b"/L1\"Demo\" Nocase Line Comment = //\n/C9\nkw1\n**\n/C12\nkw2\n/Colors = 0\n";
        assert!(detect(b));
        let c = Uew::parse(b).unwrap();
        assert_eq!(c.group_keys, 3);
        assert_eq!(c.separators, 1);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[section]\nkey=val\n"));
        assert!(Uew::parse(b"a = b\n").is_none());
    }
}
