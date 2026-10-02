//! Black `pyproject.toml` `[tool.black]` の検出と構造カウント。
//!
//! `line-length`/`target-version`/`include`/`exclude`/`skip-*`/`preview`/
//! `enable-unstable-feature` 等の既知キーを TOML 風に分類する。
//!
//! ```
//! let c = izanagi_kit::blackconf::parse(
//!     b"[tool.black]\nline-length = 100\ntarget-version = [\"py312\"]\n").unwrap();
//! assert_eq!(c.sections, 1);
//! assert!(izanagi_kit::blackconf::detect(b"[tool.black]\nline-length = 88\n"));
//! ```

/// 既知セクション。
const SECTIONS: &[&str] = &["tool.black"];
/// 既知オプションキー。
const KEYS: &[&str] = &[
    "check",
    "color",
    "diff",
    "enable-unstable-feature",
    "exclude",
    "extend-exclude",
    "fast",
    "force-exclude",
    "include",
    "ipynb",
    "line-length",
    "line_ranges",
    "no-color",
    "preview",
    "pyi",
    "python-cell-magics",
    "quiet",
    "required-version",
    "safe",
    "skip-magic-trailing-comma",
    "skip-source-first-line",
    "skip-string-normalization",
    "target-version",
    "unstable",
    "verbose",
    "workers",
];

/// Black 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[tool.black]` セクション。
    pub sections: usize,
    /// 既知オプション行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// b が `[tool.black]` 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines().any(|l| l.trim() == "[tool.black]")
}

/// 構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_black = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            let name = &t[1..t.len() - 1];
            if SECTIONS.contains(&name) {
                c.sections += 1;
                in_black = true;
            } else {
                in_black = false;
                c.misc += 1;
            }
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if in_black && KEYS.contains(&t[..pos].trim()) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.sections >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# black\n[tool.black]\nline-length = 100\ntarget-version = [\"py311\", \"py312\"]\ninclude = \"\\\\.pyi?$\"\nexclude = \"migrations\"\nextend-exclude = \"generated\"\nskip-string-normalization = true\npreview = true\nrequired-version = \"24.4\"\nworkers = 4\n\n[tool.isort]\nprofile = \"black\"\n";

    #[test]
    fn blackconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.options, 9);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_black() {
        assert!(!detect(b"[tool.ruff]\nline-length = 100\n"));
        assert!(!detect(b"hello\n"));
    }
}
