//! Ultimaker Cura コンテナプロファイル (`*.inst.cfg`/`cura.cfg`) の検出・カウント。
//!
//! `[general]`/`[metadata]`/`[values]`/`[machine]`/`[containers]` 等のセクションと
//! `key = value` 行で構成され、`setting_version = N` が必須メタデータ。
//!
//! ```
//! let cfg = b"[general]\nversion = 4\nname = MyProfile\ndefinition = fdmprinter\n\n\
//!             [metadata]\ntype = quality_changes\nsetting_version = 22\n\n\
//!             [values]\nlayer_height = 0.15\ninfill_sparse_density = 20\n";
//! assert!(izanagi_kit::curaconf::detect(cfg));
//! let c = izanagi_kit::curaconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.setting_version_entries, 1);
//! assert_eq!(c.values_entries, 2);
//! assert_eq!(c.metadata_entries, 2);
//! ```

/// Cura コンテナで既知のセクション名。
const KNOWN_SECTIONS: &[&str] = &[
    "general",
    "metadata",
    "values",
    "machine",
    "containers",
    "profile",
    "alterations",
    "mesh",
    "platform_attributes",
];

fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
    {
        return None;
    }
    Some((k, s[i + 1..].trim()))
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション数。
    pub sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// `setting_version` 指定の行数。
    pub setting_version_entries: usize,
    /// `[values]` セクション内の行数。
    pub values_entries: usize,
    /// `[metadata]` セクション内の行数。
    pub metadata_entries: usize,
    /// `[general]` セクション内の行数。
    pub general_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

/// `b` が Cura コンテナプロファイル形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.setting_version_entries >= 1 && c.sections >= 2)
        || (c.values_entries >= 1 && c.metadata_entries >= 1)
}

/// `b` を Cura `*.inst.cfg` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        setting_version_entries: 0,
        values_entries: 0,
        metadata_entries: 0,
        general_entries: 0,
        comments: 0,
    };
    let mut section = String::new();
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') {
            let inner = s
                .strip_prefix('[')
                .unwrap_or(s)
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            section = inner.to_ascii_lowercase();
            if KNOWN_SECTIONS.contains(&section.as_str()) {
                c.sections += 1;
            }
            continue;
        }
        if let Some((k, _v)) = key_of(s) {
            c.entries += 1;
            if k == "setting_version" {
                c.setting_version_entries += 1;
            }
            match section.as_str() {
                "values" => c.values_entries += 1,
                "metadata" => c.metadata_entries += 1,
                "general" => c.general_entries += 1,
                _ => {}
            }
        }
    }
    if c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_inst_cfg() {
        let cfg = b"[general]\nversion = 4\nname = draft\n\n[metadata]\n\
                    setting_version = 20\ntype = quality\nquality_type = draft\n\n\
                    [values]\nlayer_height = 0.2\nspeed_print = 50\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.values_entries, 2);
        assert_eq!(c.metadata_entries, 3);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(!detect(
            b"[database]\nhost = localhost\nport = 5432\n[user]\nname = x\n"
        ));
    }
}
