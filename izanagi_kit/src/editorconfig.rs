//! Parser for EditorConfig files (`.editorconfig`).
//!
//! Counts `root =` declaration, `[glob]` sections, `key = value` properties
//! (indent_style/indent_size/end_of_line/charset/trim_trailing_whitespace/
//! insert_final_newline/max_line_length/tab_width), `unset` values, and
//! comments.
//!
//! ```
//! let b = b"root = true\n[*]\nindent_style = space\nindent_size = 2\n";
//! assert!(izanagi_kit::editorconfig::detect(b));
//! let c = izanagi_kit::editorconfig::Editorconfig::parse(b).unwrap();
//! assert_eq!(c.root, 1);
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.properties, 2);
//! ```

/// Parsed .editorconfig summary.
#[derive(Debug, Clone)]
pub struct Editorconfig {
    /// `root = true` line present.
    pub root: usize,
    /// `[glob-pattern]` sections.
    pub sections: usize,
    /// `key = value` property lines.
    pub properties: usize,
    /// `indent_style` values.
    pub indent_styles: usize,
    /// `indent_size`/`tab_width` values.
    pub indent_widths: usize,
    /// `end_of_line` values (lf/crlf/cr).
    pub end_of_lines: usize,
    /// `charset` values.
    pub charsets: usize,
    /// `trim_trailing_whitespace` values.
    pub trim_ws: usize,
    /// `insert_final_newline` values.
    pub final_newlines: usize,
    /// `max_line_length` values.
    pub max_line_lengths: usize,
    /// `unset` property values.
    pub unsets: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

const PROPS: &[&str] = &[
    "indent_style",
    "indent_size",
    "end_of_line",
    "charset",
    "trim_trailing_whitespace",
    "insert_final_newline",
    "max_line_length",
    "tab_width",
];

/// Returns `true` when the bytes look like an .editorconfig file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let props = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            tr.split_once('=')
                .is_some_and(|(k, _)| PROPS.contains(&k.trim()))
        })
        .count();
    props >= 1 && (t.contains('[') || t.contains("root"))
}

impl Editorconfig {
    /// Parses an .editorconfig file, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            root: 0,
            sections: 0,
            properties: 0,
            indent_styles: 0,
            indent_widths: 0,
            end_of_lines: 0,
            charsets: 0,
            trim_ws: 0,
            final_newlines: 0,
            max_line_lengths: 0,
            unsets: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') && tr.ends_with(']') {
                c.sections += 1;
                continue;
            }
            if let Some((k, v)) = tr.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                if key == "root" {
                    c.root += 1;
                    continue;
                }
                if PROPS.contains(&key) {
                    c.properties += 1;
                    match key {
                        "indent_style" => c.indent_styles += 1,
                        "indent_size" | "tab_width" => c.indent_widths += 1,
                        "end_of_line" => c.end_of_lines += 1,
                        "charset" => c.charsets += 1,
                        "trim_trailing_whitespace" => c.trim_ws += 1,
                        "insert_final_newline" => c.final_newlines += 1,
                        _ => c.max_line_lengths += 1,
                    }
                    if val == "unset" {
                        c.unsets += 1;
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

    const CONF: &[u8] = b"# root\nroot = true\n\n[*]\nindent_style = space\nindent_size = 2\nend_of_line = lf\ncharset = utf-8\ntrim_trailing_whitespace = true\ninsert_final_newline = true\n\n[Makefile]\nindent_style = tab\n\n[*.{md,txt}]\nmax_line_length = unset\n";

    #[test]
    fn parses_editorconfig() {
        let c = Editorconfig::parse(CONF).unwrap();
        assert_eq!(c.root, 1);
        assert_eq!(c.sections, 3);
        assert_eq!(c.properties, 8);
        assert_eq!(c.indent_styles, 2);
        assert_eq!(c.indent_widths, 1);
        assert_eq!(c.unsets, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_editorconfig() {
        assert!(!detect(b"[package]\nname = \"x\"\n"));
        assert!(Editorconfig::parse(b"x").is_none());
    }
}
