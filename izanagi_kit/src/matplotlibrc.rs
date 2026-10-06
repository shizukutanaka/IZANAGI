//! `matplotlibrc` 検出モジュール。
//!
//! matplotlib の設定ファイルは `key: value` 形式（`#` コメント）で、
//! `backend`、`lines.linewidth`、`axes.*`、`figure.*`、`savefig.*`、
//! `font.*` 等のドット区切りキーが特徴。
//!
//! ```
//! let b = br#"backend: Agg
//! lines.linewidth: 1.5
//! axes.grid: True
//! figure.figsize: 8, 6
//! savefig.dpi: 300
//! font.size: 10
//! "#;
//! let c = izanagi_kit::matplotlibrc::parse(b);
//! assert!(izanagi_kit::matplotlibrc::detect(b));
//! assert_eq!(c.keys, 6);
//! ```

const KEY_PREFIXES: &[&str] = &[
    "agg.path.chunksize",
    "animation.",
    "axes.",
    "backend",
    "boxplot.",
    "contour.",
    "datapath",
    "date.",
    "docstring.",
    "errorbar.",
    "figure.",
    "font.",
    "grid.",
    "image.",
    "interactive",
    "keymap.",
    "legend.",
    "lines.",
    "markers.",
    "mathtext.",
    "patch.",
    "path.",
    "pcolor.",
    "pgf.",
    "polaraxes.",
    "ps.",
    "savefig.",
    "scatter.",
    "svg.",
    "text.",
    "timezone",
    "tk.",
    "toolbar",
    "verbose.",
    "webagg.",
    "xtick.",
    "ytick.",
];

fn is_comment(t: &str) -> bool {
    t.starts_with('#')
}

fn key_hit(t: &str) -> bool {
    let Some(colon) = t.find(':') else {
        return false;
    };
    let key = t[..colon].trim();
    if key.is_empty() {
        return false;
    }
    KEY_PREFIXES.iter().any(|p| {
        if let Some(head) = p.strip_suffix('.') {
            key.starts_with(p) || key == head
        } else {
            key == *p || key.starts_with(&format!("{p}."))
        }
    })
}

/// `b` が matplotlibrc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || is_comment(tr) {
            continue;
        }
        if key_hit(tr) {
            keys += 1;
        }
    }
    keys >= 3
}

/// matplotlibrc の統計。
#[derive(Debug, Default, Clone)]
pub struct Matplotlibrc {
    /// 既知キー行数。
    pub keys: usize,
    /// `figure.*`/`savefig.*` キー行数。
    pub figure_keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を matplotlibrc として統計する。
pub fn parse(b: &[u8]) -> Matplotlibrc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Matplotlibrc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if is_comment(tr) {
            c.comments += 1;
            continue;
        }
        if key_hit(tr) {
            c.keys += 1;
            if tr.starts_with("figure.") || tr.starts_with("savefig.") {
                c.figure_keys += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"backend: Agg
lines.linewidth: 1.5
axes.grid: True
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_figure() {
        let b = br#"figure.figsize: 8, 6
figure.dpi: 100
savefig.dpi: 300
savefig.format: png
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.figure_keys, 4);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"backend: x\n"));
        assert!(!detect(b"key: 1\nother: 2\nthird: 3\n"));
        assert!(!detect(b"[section]\nkey=1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
