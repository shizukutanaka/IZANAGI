//! Meltano `meltano.yml` の検出と構造カウント。
//!
//! プラグイン種別ブロック(`extractors:`/`loaders:`/`orchestrators:`/`utilities:`/
//! `transformers:`/`mappers:`/`files:`/`environments:`)と、`- name:` プラグイン
//! エントリ、`pip_url:`/`namespace:`/`settings:`/`config:`/`select:` 等の
//! 既知リーフキーを識別する。
//!
//! ```
//! let c = izanagi_kit::meltano::parse(
//!     b"version: 1\ndefault_environment: dev\nplugins:\n  extractors:\n  - name: tap-gitlab\n    namespace: tap_gitlab\n    pip_url: git+https://gitlab.com/meltano/tap-gitlab.git\n    settings:\n    - name: projects\n").unwrap();
//! assert_eq!(c.sections, 4);
//! assert!(c.options >= 5);
//! assert!(izanagi_kit::meltano::detect(
//!     b"plugins:\n  extractors:\n  - name: tap-x\n    pip_url: x\n    namespace: tap_x\n"));
//! ```

/// トップレベル既知キー。
const TOP_KEYS: &[&str] = &[
    "auto_install",
    "cli",
    "database_uri",
    "default_environment",
    "elt",
    "env_aliases",
    "environments",
    "hub_url",
    "jobs",
    "plugins",
    "project_id",
    "project_readonly",
    "schedules",
    "send_anonymous_usage_stats",
    "state_backend",
    "venv_backend",
    "version",
];

/// `plugins:` 配下のプラグイン種別。
const PLUGIN_KINDS: &[&str] = &[
    "environments",
    "extractors",
    "files",
    "loaders",
    "mappers",
    "orchestrators",
    "transformers",
    "utilities",
];

/// エントリ/リーフ既知キー。
const LEAF_KEYS: &[&str] = &[
    "capabilities",
    "config",
    "docs",
    "env",
    "executable",
    "hidden",
    "label",
    "logo_url",
    "metadata",
    "name",
    "namespace",
    "pip_url",
    "repo",
    "requires",
    "select",
    "settings",
    "task",
    "update",
    "variant",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップ/種別セクション行数。
    pub sections: usize,
    /// 既知リーフキー行数(`- name:` 等を含む)。
    pub options: usize,
    /// `#` コメント行数。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn yaml_key(t: &str) -> Option<&str> {
    if let Some(rest) = t.strip_prefix("- ") {
        return yaml_key(rest.trim_start());
    }
    let (k, _) = t.split_once(':')?;
    let k = k.trim();
    if k.is_empty() {
        return None;
    }
    if k.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Some(k)
    } else {
        None
    }
}

fn line_kind(t: &str, in_plugins: bool) -> u8 {
    let k = match yaml_key(t) {
        Some(k) => k,
        None => return 3,
    };
    if TOP_KEYS.contains(&k) || (in_plugins && PLUGIN_KINDS.contains(&k)) {
        1
    } else if LEAF_KEYS.contains(&k) {
        2
    } else {
        3
    }
}
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `meltano.yml` らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    let mut in_plugins = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        in_plugins = t == "plugins:" || (in_plugins && line.starts_with(' '));
        match line_kind(t, in_plugins) {
            1 => hits += 2,
            2 => hits += 1,
            _ => {}
        }
        if hits >= 6 {
            return true;
        }
    }
    hits >= 4
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
    let mut c = Counts {
        sections: 0,
        options: 0,
        comments: 0,
        misc: 0,
    };
    let mut in_plugins = false;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        in_plugins = t == "plugins:" || (in_plugins && line.starts_with(' '));
        match line_kind(t, in_plugins) {
            1 => c.sections += 1,
            2 => c.options += 1,
            _ => c.misc += 1,
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# meltano.yml\nversion: 1\ndefault_environment: dev\nsend_anonymous_usage_stats: false\nplugins:\n  extractors:\n  - name: tap-gitlab\n    variant: meltanolabs\n    pip_url: git+https://gitlab.com/meltano/tap-gitlab.git\n    namespace: tap_gitlab\n    config:\n      projects: a/b\n    settings:\n    - name: projects\n      kind: array\n  loaders:\n  - name: target-jsonl\n    variant: andyh1203\nenvironments:\n- name: dev\n- name: prod\nschedules:\n- name: daily\n";

    #[test]
    fn meltano() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 8);
        assert_eq!(c.options, 12);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 2);
    }

    #[test]
    fn not_meltano() {
        assert!(!detect(b"name: x\nversion: 1\n"));
        assert!(parse(b"just: text\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
