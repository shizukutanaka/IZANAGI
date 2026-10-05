//! ブラウザ拡張機能 `manifest.json`(MV2/MV3)の検出と構造カウント。
//!
//! `manifest_version`/`permissions`/`host_permissions`/`background`/
//! `content_scripts`/`action`/`browser_action`/`options_ui`/
//! `web_accessible_resources`/`declarative_net_request`/`side_panel`/
//! `browser_specific_settings` 等の拡張マニフェストキーを識別する。
//!
//! ```
//! let c = izanagi_kit::extmanifest::parse(
//!     b"{\n  \"manifest_version\": 3,\n  \"name\": \"ext\",\n  \"version\": \"1.0\",\n  \"permissions\": [\"tabs\", \"storage\"],\n  \"background\": {\"service_worker\": \"bg.js\"},\n  \"action\": {\"default_popup\": \"pop.html\"},\n  \"content_scripts\": [{\"matches\": [\"<all_urls>\"], \"js\": [\"cs.js\"]}]\n}\n").unwrap();
//! assert!(c.options >= 6);
//! assert!(izanagi_kit::extmanifest::detect(
//!     b"{\"manifest_version\": 3, \"permissions\": [\"tabs\"], \"background\": {}}\n"));
//! ```

/// 拡張マニフェスト既知キー。
const KEYS: &[&str] = &[
    "action",
    "all_frames",
    "author",
    "automation",
    "background",
    "browser_action",
    "browser_specific_settings",
    "chrome_url_overrides",
    "commands",
    "content_scripts",
    "content_security_policy",
    "css",
    "declarative_net_request",
    "default_icon",
    "default_locale",
    "default_popup",
    "default_title",
    "description",
    "devtools_page",
    "exclude_globs",
    "exclude_matches",
    "externally_connectable",
    "gecko",
    "homepage_url",
    "host_permissions",
    "icons",
    "incognito",
    "include_globs",
    "input_components",
    "js",
    "key",
    "keyword",
    "manifest_version",
    "matches",
    "minimum_chrome_version",
    "minimum_edge_version",
    "name",
    "omnibox",
    "optional_host_permissions",
    "optional_permissions",
    "options_page",
    "options_ui",
    "page",
    "page_action",
    "persistent",
    "permissions",
    "run_at",
    "sandbox",
    "scripts",
    "service_worker",
    "side_panel",
    "sidebar_action",
    "split",
    "storage",
    "theme",
    "tts_engine",
    "update_url",
    "version",
    "version_name",
    "web_accessible_resources",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー出現行数。
    pub options: usize,
    /// コメント行数(`//`)。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

fn key_hits(t: &str) -> usize {
    let t = t.strip_prefix("- ").map_or(t, |s| s.trim_start());
    let mut n = 0usize;
    for k in KEYS {
        if t.contains(&format!("\"{}\":", k)) || t.starts_with(&format!("{k}:")) {
            n += 1;
        }
    }
    n
}

/// 拡張マニフェストらしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let mut hits = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        hits += key_hits(t);
        if hits >= 3 {
            return true;
        }
    }
    false
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("//") || t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if key_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n  \"manifest_version\": 3,\n  \"name\": \"ext\",\n  \"version\": \"1.0\",\n  \"description\": \"d\",\n  \"permissions\": [\"tabs\", \"storage\"],\n  \"host_permissions\": [\"*://*/*\"],\n  \"background\": {\n    \"service_worker\": \"bg.js\"\n  },\n  \"action\": {\n    \"default_popup\": \"pop.html\"\n  },\n  \"content_scripts\": [\n    {\"matches\": [\"<all_urls>\"], \"js\": [\"cs.js\"], \"run_at\": \"document_end\"}\n  ],\n  \"options_ui\": {\"page\": \"opt.html\", \"open_in_tab\": true},\n  \"web_accessible_resources\": [{\"resources\": [\"i.png\"], \"matches\": []}]\n}\n";

    #[test]
    fn extmanifest() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 14);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn not_extmanifest() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"hello\n").is_none());
    }
}
