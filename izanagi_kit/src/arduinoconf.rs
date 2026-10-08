//! Arduino `arduino-cli.yaml` / `sketch.json` / `library.properties` /
//! `arduino.ino` 周辺設定の認識と計数。
//!
//! `arduino-cli.yaml` は `board_manager:`(`additional_urls:` リスト)、
//! `daemon:`、`directories:`(`data:`/`downloads:`/`user:`)、`library:`/
//! `logging:`/`metrics:`/`output:`/`sketch:`/`updater:` セクションを持つ YAML。
//! `sketch.json` スケッチプロファイルは `"cpu": {"fqbn":…, "config":…}`、
//! `"secrets": [{"name":…,"value":…}]`、`"included_libs":[]`、
//! `"profiles": {"<name>": {"platform":…, "platform_index":…, "fqbn":…, "libraries":[]}}`。
//! `library.properties` は properties 形式(`name=`/`version=`/`author=`/
//! `maintainer=`/`sentence=`/`paragraph=`/`category=`/`url=`/`architectures=`/
//! `includes=`/`depends=`/`dot_a_linkage=`/`precompiled=`/`ldflags=`)。
//!
//! ```
//! let b = b"board_manager:\n    additional_urls:\n        - https://dl.espressif.com/dl/package_esp32_index.json\n    directories:\n        data: ~/Arduino15\n        downloads: ~/Arduino15/staging\n        user: ~/Arduino\n    library:\n        enable_unsafe_install: false\n    logging:\n        level: info\n        format: text\n    metrics:\n        enabled: true\n    updater:\n        enable_notification: true\n    sketch:\n        always_export_binaries: false\n";
//! assert!(izanagi_kit::arduinoconf::detect(b));
//! let c = izanagi_kit::arduinoconf::parse(b).unwrap();
//! assert_eq!(c.keys, 17);
//! assert_eq!(c.sections, 7); // board_manager/directories/library/logging/metrics/updater/sketch
//! assert_eq!(c.list_items, 1);
//! assert_eq!(c.board_keys, 4); // additional_urls + data + downloads + user
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key:`/`key =`/`key:` のキー個数。
    pub keys: usize,
    /// `board_manager`/`daemon`/`directories`/`library`/`logging`/`metrics`/
    /// `output`/`sketch`/`updater`/`cpu`/`secrets`/`profiles`/`included_libs`
    /// セクション・構造キーの個数。
    pub sections: usize,
    /// `additional_urls`/`data`/`downloads`/`user`/`fqbn`/`platform`/
    /// `additional_properties`/`config`/`preferences`/`upload`/`programmer`/
    /// `network` 系ボード関連キーの個数。
    pub board_keys: usize,
    /// `name`/`version`/`author`/`maintainer`/`sentence`/`paragraph`/
    /// `category`/`url`/`architectures`/`includes`/`depends`/`dot_a_linkage`/
    /// `precompiled`/`ldflags`/`helper`/`types` ライブラリ属性キーの個数。
    pub library_keys: usize,
    /// `-` リスト項目行の個数。
    pub list_items: usize,
    /// `#`/`;` コメント行の個数。
    pub comments: usize,
}

const SECTION_KEYS: &[&str] = &[
    "board_manager",
    "daemon",
    "directories",
    "library",
    "logging",
    "metrics",
    "output",
    "sketch",
    "updater",
    "cpu",
    "secrets",
    "profiles",
    "included_libs",
    "properties",
];

const BOARD_KEYS: &[&str] = &[
    "additional_urls",
    "data",
    "downloads",
    "user",
    "fqbn",
    "platform",
    "platform_index",
    "additional_properties",
    "config",
    "preferences",
    "upload",
    "programmer",
    "network",
    "port",
    "serial",
];

const LIBRARY_KEYS: &[&str] = &[
    "name",
    "version",
    "author",
    "maintainer",
    "sentence",
    "paragraph",
    "category",
    "url",
    "architectures",
    "includes",
    "depends",
    "dot_a_linkage",
    "precompiled",
    "ldflags",
    "helper",
    "types",
    "crc32",
    "size",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// `arduino-cli.yaml`/`sketch.json`/`library.properties` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        for key in [
            "board_manager",
            "additional_urls",
            "fqbn",
            "arduino",
            "architectures=",
            "sentence=",
            "paragraph=",
            "category=",
            "dot_a_linkage",
            "platform_index",
            "directories:",
            "\"cpu\"",
            "\"secrets\"",
            "\"profiles\"",
            "included_libs",
        ] {
            if s.starts_with(key) || s.contains(key) {
                hits += 1;
            }
        }
    }
    hits >= 2
}

fn line_key(s: &str) -> Option<&str> {
    let t = s.strip_prefix('-').map_or(s, |r| r.trim_start());
    if let Some(rest) = t.strip_prefix('"') {
        let end = rest.find('"')?;
        if rest[end + 1..].trim_start().starts_with(':') {
            return Some(&rest[..end]);
        }
        return None;
    }
    for sep in [':', '='] {
        if let Some(pos) = t.find(sep) {
            // `https://` 等スキーム区切りはキーと見なさない。
            if sep == ':' && t[pos + 1..].starts_with("//") {
                continue;
            }
            let k = t[..pos]
                .trim_end()
                .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
                .next()
                .unwrap_or("");
            if !k.is_empty() {
                return Some(k);
            }
        }
    }
    None
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let t = strip_bom(t);
    let mut c = Counts {
        keys: 0,
        sections: 0,
        board_keys: 0,
        library_keys: 0,
        list_items: 0,
        comments: 0,
    };
    for l in t.lines() {
        if l.trim().is_empty() {
            continue;
        }
        let s = l.trim();
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('-') {
            c.list_items += 1;
        }
        let Some(key) = line_key(s) else {
            continue;
        };
        c.keys += 1;
        if SECTION_KEYS.contains(&key) {
            c.sections += 1;
        }
        if BOARD_KEYS.contains(&key) {
            c.board_keys += 1;
        }
        if LIBRARY_KEYS.contains(&key) {
            c.library_keys += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(
            b"board_manager:\n    additional_urls:\n        - https://x\n    directories:\n        data: d\n"
        ));
        assert!(detect(
            b"name=Foo\nversion=1.0\nsentence=S\nparagraph=P\narchitectures=avr\n"
        ));
    }

    #[test]
    fn rejects() {
        assert!(!detect(b"foo: bar\nkind: y\n"));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = b"board_manager:\n    additional_urls:\n        - https://a\n        - https://b\n    directories:\n        data: d\n        user: u\n    sketch:\n        always_export_binaries: false\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 3); // board_manager/directories/sketch
        assert_eq!(c.board_keys, 3); // additional_urls/data/user
        assert_eq!(c.list_items, 2);
    }

    #[test]
    fn library_props() {
        let b = b"name=Foo\nversion=1.0.0\nauthor=me\nmaintainer=me\nsentence=S\nparagraph=P\ncategory=Other\nurl=https://x\narchitectures=avr,esp32\nincludes=Foo.h\ndepends=Bar(>=1.0)\n";
        let c = parse(b).unwrap();
        assert_eq!(c.keys, 11);
        assert_eq!(c.library_keys, 11);
    }

    #[test]
    fn copy_eq() {
        let c = parse(b"board_manager:\n    additional_urls:\n        - u\n").unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
