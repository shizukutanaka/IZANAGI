//! `tilestache.cfg` (TileStache 設定JSON) 検出モジュール。
//!
//! TileStache (タイルサーバ) の設定は JSON で、トップレベルに
//! `"cache"`/`"layers"`/`"logging"`/`"memcache"`/`"index"`/
//! `"preview"`/`"publicUrl"`/`"urlPrefix"`/`"allowed origin"`/
//! `"deployment options"` キーを持ち、`"layers"` の値内に
//! `"provider"`/`"projection"`/`"metatile"`/`"stale lock timeout"`
//! 等のキーがある。
//!
//! ```
//! let b = br#"{
//!     "cache": {"name": "Test"},
//!     "layers": {
//!         "example": {
//!             "provider": {"name": "mapnik", "mapfile": "style.xml"},
//!             "projection": "spherical mercator",
//!             "metatile": {"rows": 4, "columns": 4}
//!         }
//!     }
//! }
//! "#;
//! let c = izanagi_kit::tilestacheconf::parse(b);
//! assert!(izanagi_kit::tilestacheconf::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "allowed origin",
    "bounds",
    "cache",
    "coord",
    "deployment options",
    "dim",
    "height",
    "index",
    "key names",
    "layers",
    "logging",
    "maskfolder",
    "memcache",
    "metatile",
    "name",
    "pixel effect",
    "preview",
    "projection",
    "provider",
    "publicUrl",
    "redirects",
    "stale lock timeout",
    "template cache",
    "tile height",
    "urlPrefix",
    "width",
    "write cache",
];

const HINTS: &[&str] = &[
    "layers",
    "cache",
    "metatile",
    "stale lock timeout",
    "urlPrefix",
    "tile height",
];

/// 行内の最初の `"key":` を返す。
fn jkey(line: &str) -> Option<&str> {
    let s = line.trim_start();
    let s = s.strip_prefix('"')?;
    let e = s.find('"')?;
    let key = &s[..e];
    if s[e + 1..].trim_start().starts_with(':') {
        Some(key)
    } else {
        None
    }
}

/// `b` が TileStache 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut hints = 0usize;
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
            if HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    (hints >= 1 && keys >= 3) || keys >= 5
}

/// TileStache 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct TilestacheConf {
    /// 既知キー行数。
    pub keys: usize,
    /// TileStache 特有キー行数。
    pub hint_keys: usize,
}

/// `b` を TileStache 設定として統計する。
pub fn parse(b: &[u8]) -> TilestacheConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = TilestacheConf::default();
    for l in t.lines() {
        if let Some(k) = jkey(l) {
            if KEYS.contains(&k) {
                c.keys += 1;
            }
            if HINTS.contains(&k) {
                c.hint_keys += 1;
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
        let b = br#"{
    "cache": {"name": "Test"},
    "layers": {},
    "logging": "info"
}
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn detects_many() {
        let b = br#"{
    "cache": {},
    "index": {},
    "preview": {},
    "memcache": {},
    "publicUrl": "x"
}
"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"{\"cache\": {}, \"layers\": {}}"));
        assert!(!detect(b"{\"name\": \"x\", \"width\": 1, \"height\": 2}"));
        assert!(!detect(b"cache=x\nlayers=y\nlogging=z\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
