//! `Doxyfile` 検出モジュール。
//!
//! Doxygen の設定ファイルは `KEY = value` 形式で、`PROJECT_NAME`、
//! `OUTPUT_DIRECTORY`、`INPUT`、`GENERATE_HTML`、`EXTRACT_ALL` 等の
//! 大文字キーが特徴。`@INCLUDE`/`@INCLUDE_PATH` ディレクティブも持つ。
//!
//! ```
//! let b = br#"PROJECT_NAME           = MyProject
//! OUTPUT_DIRECTORY       = docs
//! INPUT                  = src include
//! RECURSIVE              = YES
//! EXTRACT_ALL            = YES
//! GENERATE_HTML          = YES
//! GENERATE_LATEX         = NO
//! "#;
//! let c = izanagi_kit::doxygenconf::parse(b);
//! assert!(izanagi_kit::doxygenconf::detect(b));
//! assert_eq!(c.keys, 7);
//! ```

const KEYS: &[&str] = &[
    "ABBREVIATE_BRIEF",
    "ALWAYS_DETAILED_SEC",
    "BUILTIN_STL_SUPPORT",
    "CALLER_GRAPH",
    "CALL_GRAPH",
    "CLASS_DIAGRAMS",
    "COLS_IN_ALPHA_INDEX",
    "CREATE_SUBDIRS",
    "DISABLE_INDEX",
    "DISTRIBUTE_GROUP_DOC",
    "DOXYFILE_ENCODING",
    "ENABLE_PREPROCESSING",
    "EXCLUDE",
    "EXCLUDE_PATTERNS",
    "EXTRACT_ALL",
    "EXTRACT_PRIVATE",
    "EXTRACT_STATIC",
    "FILE_PATTERNS",
    "GENERATE_HTML",
    "GENERATE_LATEX",
    "GENERATE_MAN",
    "GENERATE_XML",
    "GRAPHICAL_HIERARCHY",
    "HIDE_UNDOC_MEMBERS",
    "HTML_FILE_EXTENSION",
    "HTML_OUTPUT",
    "IMAGE_PATH",
    "INHERIT_DOCS",
    "INPUT",
    "INPUT_ENCODING",
    "INTERNAL_DOCS",
    "JAVADOC_AUTOBRIEF",
    "LAYOUT_FILE",
    "MACRO_EXPANSION",
    "MARKDOWN_SUPPORT",
    "MULTILINE_CPP_IS_BRIEF",
    "OPTIMIZE_OUTPUT_FOR_C",
    "OUTPUT_DIRECTORY",
    "OUTPUT_LANGUAGE",
    "PREDEFINED",
    "PROJECT_BRIEF",
    "PROJECT_LOGO",
    "PROJECT_NAME",
    "PROJECT_NUMBER",
    "QT_AUTOBRIEF",
    "RECURSIVE",
    "REFERENCED_BY_RELATION",
    "REFERENCES_LINK_SOURCE",
    "REFERENCES_RELATION",
    "SEARCHENGINE",
    "SHOW_FILES",
    "SOURCE_BROWSER",
    "STRIP_FROM_PATH",
    "SUBGROUPING",
    "TAB_SIZE",
    "TYPEDEF_HIDES_STRUCT",
    "USE_MATHJAX",
    "WARNINGS",
    "WARN_IF_UNDOCUMENTED",
    "XML_OUTPUT",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn key_present(t: &str, k: &str) -> bool {
    if !t.starts_with(k) {
        return false;
    }
    let rest = t[k.len()..].trim_start();
    rest.starts_with('=')
}

/// `b` が Doxyfile に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            keys += 1;
        }
    }
    keys >= 3
}

/// Doxyfile の統計。
#[derive(Debug, Default, Clone)]
pub struct DoxygenConf {
    /// 既知オプションの代入行数。
    pub keys: usize,
    /// `@INCLUDE`/`@INCLUDE_PATH` ディレクティブ行数。
    pub include_directives: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を Doxyfile として統計する。
pub fn parse(b: &[u8]) -> DoxygenConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = DoxygenConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("@INCLUDE") {
            c.include_directives += 1;
            continue;
        }
        if KEYS.iter().any(|k| key_present(tr, k)) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"PROJECT_NAME = x
OUTPUT_DIRECTORY = docs
INPUT = src
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_include() {
        let b = br#"PROJECT_NAME = x
OUTPUT_DIRECTORY = docs
INPUT = src
@INCLUDE = shared.cfg
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.include_directives, 1);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"PROJECT_NAME = x\n"));
        assert!(!detect(b"key = 1\nother = 2\nthird = 3\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
