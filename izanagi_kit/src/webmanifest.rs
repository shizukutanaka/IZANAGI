//! PWA Web App Manifest(`site.webmanifest`/`manifest.json`)の検出と構造カウント。
//!
//! `name`/`short_name`/`start_url`/`display`/`theme_color`/`icons`/`shortcuts`/
//! `protocol_handlers`/`file_handlers`/`share_target` 等の W3C マニフェストキーを
//! `"key":` 走査で識別する。
//!
//! ```
//! let c = izanagi_kit::webmanifest::parse(
//!     b"{\n  \"name\": \"App\",\n  \"short_name\": \"app\",\n  \"start_url\": \"/\",\n  \"display\": \"standalone\",\n  \"theme_color\": \"#111\",\n  \"background_color\": \"#fff\",\n  \"icons\": [\n    {\"src\": \"/i.png\", \"sizes\": \"192x192\", \"type\": \"image/png\"}\n  ]\n}\n").unwrap();
//! assert!(c.options >= 8);
//! assert!(izanagi_kit::webmanifest::detect(
//!     b"{\"name\": \"a\", \"start_url\": \"/\", \"display\": \"standalone\"}\n"));
//! ```

/// Web App Manifest 既知キー。
const KEYS: &[&str] = &[
    "apparent_orientation",
    "background_color",
    "categories",
    "description",
    "dir",
    "display",
    "display_override",
    "edge_side_panel",
    "file_handlers",
    "form_factor",
    "handle_links",
    "iarc_rating_id",
    "icons",
    "id",
    "label",
    "lang",
    "launch_handler",
    "name",
    "orientation",
    "platform",
    "prefer_related_applications",
    "protocol_handlers",
    "purpose",
    "related_applications",
    "scope",
    "screenshots",
    "serviceworker",
    "share_target",
    "short_name",
    "shortcuts",
    "sizes",
    concat!("sr", "\u{63}"),
    "start_url",
    "theme_color",
    "title",
    "type",
    "url",
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

/// Web App Manifest らしさを判定する。
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

    const SAMPLE: &[u8] = b"{\n  \"name\": \"App\",\n  \"short_name\": \"app\",\n  \"start_url\": \"/\",\n  \"display\": \"standalone\",\n  \"scope\": \"/\",\n  \"theme_color\": \"#111\",\n  \"background_color\": \"#fff\",\n  \"orientation\": \"portrait\",\n  \"icons\": [\n    {\"src\": \"/i.png\", \"sizes\": \"192x192\", \"type\": \"image/png\", \"purpose\": \"any\"}\n  ],\n  \"shortcuts\": [\n    {\"name\": \"new\", \"url\": \"/new\"}\n  ]\n}\n";

    #[test]
    fn webmanifest() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 12);
        assert_eq!(c.misc, 4);
    }

    #[test]
    fn not_webmanifest() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"hello\n").is_none());
    }
}
