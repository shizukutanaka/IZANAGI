//! markdownlint 設定ファイル(`.markdownlint.json`/`.markdownlintrc`/
//! `.markdownlint.yaml`)の検出と構造カウント。JSON と YAML の両形式に対応。
//!
//! `MDNNN` ルールコードと名前付きルールエイリアス(`default`/`line-length`/
//! `first-line-heading`/`no-...` 等)を識別する。
//!
//! ```
//! let c = izanagi_kit::markdownlint::parse(
//!     b"{\n  \"default\": true,\n  \"MD013\": { \"line_length\": 120 },\n  \"no-inline-html\": false\n}\n").unwrap();
//! assert_eq!(c.rules, 3);
//! assert_eq!(c.options, 1);
//! assert!(izanagi_kit::markdownlint::detect(b"MD013: false\ndefault: true\n"));
//! ```

/// 既知ルールエイリアス名。
const ALIASES: &[&str] = &[
    "blanks-around-fences",
    "blanks-around-headings",
    "blanks-around-lists",
    "code-block-style",
    "code-fence-style",
    "commands-show-output",
    "default",
    "emphasis-style",
    "fenced-code-language",
    "first-line-heading",
    "first-line-h1",
    "hard-tab",
    "heading-start-left",
    "heading-style",
    "hr-style",
    "line-length",
    "link-fragments",
    "link-image-reference-definitions",
    "link-image-style",
    "list-indent",
    "list-marker-space",
    "no-alt-text",
    "no-bare-urls",
    "no-blanks-blockquote",
    "no-duplicate-heading",
    "no-emphasis-as-heading",
    "no-empty-links",
    "no-hard-tabs",
    "no-inline-html",
    "no-missing-space-atx",
    "no-missing-space-closed-atx",
    "no-multiple-blanks",
    "no-multiple-space-atx",
    "no-multiple-space-closed-atx",
    "no-reversed-links",
    "no-space-in-code",
    "no-space-in-emphasis",
    "no-space-in-links",
    "no-trailing-punctuation",
    "no-trailing-spaces",
    "no-undefined-references",
    "no-unused-definitions",
    "ol-prefix",
    "proper-names",
    "required-headings",
    "single-h1",
    "single-title",
    "strong-style",
    "table-column-style",
    "table-pipe-style",
    "table-row-count",
    "ul-indent",
    "ul-start-left",
    "ul-style",
];

/// markdownlint 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// ルールエントリ(`MDNNN` または既知エイリアス)。
    pub rules: usize,
    /// ルール内オプションキー。
    pub options: usize,
    /// `#`/`//` コメント行(YAML 時)。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// `MD` + 3 桁数字か。
fn is_md_code(k: &str) -> bool {
    let b = k.as_bytes();
    b.len() == 5 && b[0] == b'M' && b[1] == b'D' && b[2..].iter().all(|c| c.is_ascii_digit())
}

/// キーがルール名か。
fn is_rule(k: &str) -> bool {
    is_md_code(k) || ALIASES.contains(&k)
}

/// JSON 行内の `"key":` 出現キーを全て抜き出す。
fn keys_in_line<'a>(t: &'a str, out: &mut Vec<&'a str>) {
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                j += 1;
            }
            if j < bytes.len() && j > start {
                let mut k = j + 1;
                while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b':' {
                    out.push(&t[start..j]);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

/// YAML 行の `key:` キー名。
fn yaml_key(t: &str) -> Option<&str> {
    let p = t.find(':')?;
    let k = t[..p].trim().trim_matches('"').trim_matches('\'');
    if k.is_empty()
        || !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        None
    } else {
        Some(k)
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// b が markdownlint 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('{') || t.starts_with('[') || t.is_empty() {
            continue;
        }
        let mut keys = Vec::new();
        keys_in_line(t, &mut keys);
        if !keys.is_empty() {
            hits += keys.iter().filter(|k| is_rule(k)).count();
        } else if yaml_key(t).is_some_and(is_rule) {
            hits += 1;
        }
    }
    hits >= 2
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        rules: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        let mut keys = Vec::new();
        keys_in_line(t, &mut keys);
        if !keys.is_empty() {
            for k in keys {
                if is_rule(k) {
                    c.rules += 1;
                } else {
                    c.options += 1;
                }
            }
            if t.chars().any(|ch| ch.is_alphanumeric()) && !t.contains('"') {
                c.misc += 1;
            }
            continue;
        }
        if let Some(k) = yaml_key(t) {
            if is_rule(k) {
                c.rules += 1;
            } else {
                c.options += 1;
            }
            continue;
        }
        // `{`/`}`/`[`/`]`/`,`/単純値のみの行は構造行とみなす。
        if t.chars().any(|ch| ch.is_alphanumeric()) {
            c.misc += 1;
        }
    }
    (c.rules >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &[u8] = b"{\n  \"default\": true,\n  \"MD003\": { \"style\": \"atx\" },\n  \"MD013\": { \"line_length\": 150, \"tables\": false },\n  \"MD033\": false,\n  \"no-inline-html\": { \"allowed_elements\": [\"br\"] }\n}\n";
    const SAMPLE_YAML: &[u8] =
        b"# lint\ndefault: true\nMD013:\n  line_length: 120\nno-inline-html: false\nMD041: false\n";

    #[test]
    fn markdownlint_json() {
        let c = parse(SAMPLE_JSON).unwrap();
        assert_eq!(c.rules, 5);
        assert_eq!(c.options, 4);
        assert_eq!(c.misc, 0);
    }

    #[test]
    fn markdownlint_yaml() {
        let c = parse(SAMPLE_YAML).unwrap();
        assert_eq!(c.rules, 4);
        assert_eq!(c.options, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn not_markdownlint() {
        assert!(!detect(b"{\"name\": \"x\", \"version\": \"1\"}\n"));
        assert!(!detect(b"key: value\nother: thing\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
