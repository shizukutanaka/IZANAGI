//! `minidlna.conf` (ReadyMedia) 検出モジュール。
//!
//! MiniDLNA/ReadyMedia の設定は平坦な `key=value` 形式で、
//! `media_dir=`(繰り返し可、`A,`/`V,`/`P,` 型指定付きもある)/
//! `friendly_name=`/`db_dir=`/`log_dir=`/`port=`/`presentation_url=`/
//! `uuid=`/`serial=`/`model_name=`/`model_number=`/`notify_interval=`/
//! `inotify=`/`tivo_discovery=`/`dlna_stui`/`ssdp_udn`/
//! `force_sort_criteria`/`merge_media_dirs`/`max_connections`/
//! `minissdpdsocket`/`root_container`/`album_art_names`/
//! `wide_links`/`enable_tivo`/`strict_dlna`/`network_interface`
//! 等のキーで構成される。
//!
//! ```
//! let b = b"media_dir=V,/srv/media/videos\n\
//!           media_dir=A,/srv/media/music\n\
//!           friendly_name=MyServer\n\
//!           db_dir=/var/cache/minidlna\n\
//!           port=8200\n";
//! let c = izanagi_kit::minidlna::parse(b);
//! assert!(izanagi_kit::minidlna::detect(b));
//! assert_eq!(c.keys, 5);
//! ```

const KEYS: &[&str] = &[
    "album_art_names",
    "db_dir",
    "dlna_stui",
    "enable_tivo",
    "force_sort_criteria",
    "friendly_name",
    "http_port",
    "inotify",
    "ipv6",
    "log_dir",
    "log_level",
    "max_connections",
    "media_dir",
    "merge_media_dirs",
    "miniupnpciface",
    "minissdpdsocket",
    "model_description",
    "model_name",
    "model_number",
    "model_url",
    "network_interface",
    "notify_interval",
    "port",
    "presentation_url",
    "root_container",
    "serial",
    "smart_albums",
    "ssdp_udn",
    "strict_dlna",
    "tivo_discovery",
    "user",
    "uuid",
    "wide_links",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が minidlna.conf に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut keys = 0usize;
    let mut media = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if tr.starts_with("media_dir=") || tr.starts_with("media_dir =") {
            media += 1;
            keys += 1;
        } else if tr.contains('=') && key_present(tr) {
            keys += 1;
        }
    }
    (media >= 1 && keys >= 2) || keys >= 3
}

/// minidlna.conf の統計。
#[derive(Debug, Default, Clone)]
pub struct Minidlna {
    /// 既知キー行数。
    pub keys: usize,
    /// `media_dir` 行数。
    pub media_dirs: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を minidlna.conf として統計する。
pub fn parse(b: &[u8]) -> Minidlna {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Minidlna::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if tr.starts_with("media_dir=") || tr.starts_with("media_dir =") {
            c.media_dirs += 1;
            c.keys += 1;
        } else if tr.contains('=') && key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"media_dir=/srv/media\nfriendly_name=X\ndb_dir=/var/lib/m\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
        assert_eq!(c.media_dirs, 1);
    }

    #[test]
    fn detects_keys() {
        let b = b"friendly_name=X\ndb_dir=/y\nlog_dir=/z\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"media_dir=/x\n"));
        assert!(!detect(b"friendly_name=X\ndb_dir=/y\n"));
        assert!(!detect(b"key=value\nfoo=bar\nbaz=quux\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.keys, 0);
    }
}
