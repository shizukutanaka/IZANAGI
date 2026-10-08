//! Python パッケージ公開設定(`.pypirc`)の検出と構造カウント。
//!
//! `[distutils]`(`index-servers`)・`[pypi]`/`[testpypi]`/`[server-login]`
//! 既知セクションと `repository`/`username`/`password`/`ca_cert`/
//! `client_cert`/`cert`/`verify`/`realm` 既知キーを識別する。
//!
//! ```
//! let c = izanagi_kit::pypirc::parse(
//!     b"[distutils]\nindex-servers = pypi\n\n[pypi]\nrepository = https://upload.pypi.org/legacy/\nusername = me\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.options, 3);
//! assert!(izanagi_kit::pypirc::detect(b"[pypi]\nusername = me\npassword = x\n"));
//! ```

use crate::textutil::strip_bom;
/// 既知セクション名。
const SECTIONS: &[&str] = &["distutils", "pypi", "testpypi", "server-login"];

/// 既知キー。
const KEYS: &[&str] = &[
    "ca_cert",
    "cert",
    "client_cert",
    "index-servers",
    "password",
    "realm",
    "repository",
    "repository_url",
    "username",
    "verify",
];

/// pypirc 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[section]` 行(既知名)。
    pub sections: usize,
    /// 未知セクション名(追加 index-server)。
    pub custom_sections: usize,
    /// `key = value` 既知代入。
    pub options: usize,
    /// `#`/`;` コメント行。
    pub comments: usize,
    /// 未知キー/分類不能行。
    pub misc: usize,
}

/// `[name]` セクション行か。
fn section_name(t: &str) -> Option<&str> {
    t.strip_prefix('[')?.strip_suffix(']')
}

/// `key = value` または `key: value` のキー部分。
fn kv_key(t: &str) -> Option<&str> {
    let p = t.find(['=', ':'])?;
    let k = t[..p].trim();
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

/// b が .pypirc かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    let text = strip_bom(text);
    let mut hits = 0;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if (section_name(t).is_some_and(|s| SECTIONS.contains(&s)))
            || kv_key(t).is_some_and(|k| KEYS.contains(&k))
        {
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
        sections: 0,
        custom_sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') || t.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if let Some(s) = section_name(t) {
            if SECTIONS.contains(&s) {
                c.sections += 1;
            } else {
                c.custom_sections += 1;
            }
            continue;
        }
        if let Some(k) = kv_key(t) {
            if KEYS.contains(&k) {
                c.options += 1;
            } else {
                c.misc += 1;
            }
            continue;
        }
        c.misc += 1;
    }
    (c.sections + c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# pypirc\n[distutils]\nindex-servers =\n    pypi\n    testpypi\n\n[pypi]\nrepository = https://upload.pypi.org/legacy/\nusername = me\npassword = secret\n\n[testpypi]\nrepository = https://test.pypi.org/legacy/\nusername = me\nca_cert = /etc/ssl/ca.pem\n";

    #[test]
    fn pypirc() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.options, 7);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_pypirc() {
        assert!(!detect(b"[package]\nname = x\n"));
        assert!(!detect(b"key = value\n"));
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
