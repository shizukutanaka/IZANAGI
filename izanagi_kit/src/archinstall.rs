//! archinstall `config.json`/`user_configuration.json` の認識と計数。
//!
//! archinstall 設定は JSON で、既知トップレベルキー:
//! `archinstall-language`/`additional-repositories`/`audio_config`/
//! `bootloader`/`config_version`/`custom-commands`/`debug`/`disk_config`/
//! `hostname`/`kernels`/`locale_config`/`mirror_config`/`network_config`/
//! `no_pkg_lookups`/`ntp`/`offline`/`packages`/`parallel_downloads`/
//! `profile_config`/`save_config`/`script`/`services`/`silent`/`swap`/
//! `timezone`/`version`/`users`/`superusers`/`!root-password`/`!separator`。
//! `disk_config`/`users` 等はネストしたオブジェクト・配列を持つ。
//!
//! ```
//! let b = br#"{
//!   "archinstall-language": "English",
//!   "audio_config": {"audio": "pipewire"},
//!   "bootloader": "Systemd-boot",
//!   "disk_config": {"device": "/dev/sda"},
//!   "hostname": "archlinux",
//!   "kernels": ["linux"],
//!   "ntp": true,
//!   "packages": ["vim", "git"],
//!   "parallel_downloads": 0,
//!   "services": ["sshd"],
//!   "swap": true,
//!   "timezone": "Asia/Tokyo",
//!   "users": [{"username": "dev", "password": "x"}],
//!   "version": "3.0.0"
//! }
//! "#;
//! assert!(izanagi_kit::archinstall::detect(b));
//! let c = izanagi_kit::archinstall::parse(b).unwrap();
//! assert_eq!(c.known_keys, 14);
//! assert_eq!(c.bool_keys, 2); // ntp/swap
//! assert_eq!(c.array_keys, 4); // kernels/packages/services/users
//! assert_eq!(c.object_keys, 2); // audio_config/disk_config
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知 archinstall キーの個数(出現したもののみ、重複なし)。
    pub known_keys: usize,
    /// 既知キーのうち値が真偽値のものの個数。
    pub bool_keys: usize,
    /// 既知キーのうち値が配列のものの個数。
    pub array_keys: usize,
    /// 既知キーのうち値が `{` オブジェクトのものの個数。
    pub object_keys: usize,
    /// 既知キーのうち値が数値のものの個数。
    pub numeric_keys: usize,
    /// JSON トップレベル(depth 1)のキー総数(既知・未知問わず)。
    pub top_keys: usize,
}

const KNOWN: &[&str] = &[
    "archinstall-language",
    "additional-repositories",
    "app_config",
    "audio_config",
    "auth_config",
    "bootloader",
    "config_version",
    "custom-commands",
    "debug",
    "disk_config",
    "dry-run",
    "hostname",
    "kernels",
    "locale_config",
    "mirror_config",
    "network_config",
    "no_pkg_lookups",
    "ntp",
    "offline",
    "packages",
    "parallel_downloads",
    "profile_config",
    "save_config",
    "script",
    "services",
    "silent",
    "swap",
    "timezone",
    "users",
    "version",
    "!root-password",
    "__separator__",
    "separator",
];

fn str_len(b: &[u8], i: usize) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            b'"' => return j,
            _ => {}
        }
        j += 1;
    }
    b.len()
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
        i += 1;
    }
    i
}

fn skip_val(b: &[u8], i: usize) -> usize {
    let i = skip_ws(b, i);
    if i >= b.len() {
        return i;
    }
    match b[i] {
        b'"' => str_len(b, i) + 1,
        b'{' | b'[' => {
            let mut d = 1usize;
            let mut j = i + 1;
            while j < b.len() && d > 0 {
                match b[j] {
                    b'{' | b'[' => d += 1,
                    b'}' | b']' => d -= 1,
                    b'"' => j = str_len(b, j),
                    _ => {}
                }
                j += 1;
            }
            j
        }
        _ => {
            let mut j = i;
            while j < b.len() && b[j] != b',' && b[j] != b'}' && b[j] != b']' {
                j += 1;
            }
            j
        }
    }
}

/// JSON トップレベル(`{`直下)の `("key", value_kind)` を列挙する。
/// kind: 0=その他, 1=bool, 2=配列, 3=オブジェクト, 4=数値。
fn top_entries<'a>(t: &'a str, out: &mut std::vec::Vec<(&'a str, u8)>) {
    let b = t.as_bytes();
    let Some(open) = t.find('{') else {
        return;
    };
    let mut i = skip_ws(b, open + 1);
    if i < b.len() && b[i] == b'}' {
        return;
    }
    loop {
        i = skip_ws(b, i);
        if i >= b.len() || b[i] != b'"' {
            break;
        }
        let e = str_len(b, i);
        if e >= b.len() {
            break;
        }
        let key = &t[i + 1..e];
        i = skip_ws(b, e + 1);
        if i >= b.len() || b[i] != b':' {
            break;
        }
        i = skip_ws(b, i + 1);
        let kind = match b.get(i) {
            Some(b't') | Some(b'f') => 1u8,
            Some(b'[') => 2,
            Some(b'{') => 3,
            Some(c) if c.is_ascii_digit() || *c == b'-' => 4,
            _ => 0,
        };
        out.push((key, kind));
        let v_end = skip_val(b, i);
        i = skip_ws(b, v_end);
        if i < b.len() && b[i] == b',' {
            i += 1;
            continue;
        }
        break;
    }
}

/// archinstall 設定らしさを返す。既知キーが複数あること。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut entries = std::vec::Vec::new();
    top_entries(t, &mut entries);
    let mut hits = 0usize;
    for (k, _) in &entries {
        if KNOWN.contains(k) {
            hits += 1;
        }
    }
    hits >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let t = std::str::from_utf8(b).ok()?;
    let mut entries = std::vec::Vec::new();
    top_entries(t, &mut entries);
    let mut c = Counts {
        known_keys: 0,
        bool_keys: 0,
        array_keys: 0,
        object_keys: 0,
        numeric_keys: 0,
        top_keys: 0,
    };
    c.top_keys = entries.len();
    let mut seen: std::vec::Vec<&str> = std::vec::Vec::new();
    for (k, kind) in entries {
        if KNOWN.contains(&k) && !seen.contains(&k) {
            seen.push(k);
            c.known_keys += 1;
            match kind {
                1 => c.bool_keys += 1,
                2 => c.array_keys += 1,
                3 => c.object_keys += 1,
                4 => c.numeric_keys += 1,
                _ => {}
            }
        }
    }
    if !t.trim().is_empty()
        && c.known_keys + c.bool_keys + c.array_keys + c.object_keys + c.numeric_keys + c.top_keys
            == 0
    {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        assert!(detect(br#"{"hostname": "x", "ntp": true}"#));
        assert!(detect(br#"{"bootloader": "grub", "timezone": "UTC"}"#));
    }

    #[test]
    fn rejects() {
        assert!(!detect(br#"{"foo": 1, "bar": 2}"#));
        assert!(!detect(&[0xff]));
    }

    #[test]
    fn counts() {
        let b = br#"{"hostname": "x", "ntp": true, "packages": ["a"], "disk_config": {}, "parallel_downloads": 2, "zzz": 1}"#;
        let c = parse(b).unwrap();
        assert_eq!(c.top_keys, 6);
        assert_eq!(c.known_keys, 5);
        assert_eq!(c.bool_keys, 1);
        assert_eq!(c.array_keys, 1);
        assert_eq!(c.object_keys, 1);
        assert_eq!(c.numeric_keys, 1);
    }

    #[test]
    fn copy_eq() {
        let c = parse(br#"{"hostname": "x", "ntp": true}"#).unwrap();
        let d = c;
        assert_eq!(c, d);
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
