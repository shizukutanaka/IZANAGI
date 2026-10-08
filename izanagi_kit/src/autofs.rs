//! autofs `auto.master`/`auto.*` マップファイル パーサ。
//!
//! auto.master 行: `<mount-point> <map> [options]` または `<mount> <maptype>:<map>`。
//! マップ行: `<key> [-options] <location>`。`+mapname`/`/-`/`/net`/`program:`/
//! `file:`/`yp:`/`nisplus:`/`ldap:`/`ldaps:`/`sss:` 等を計数する。
//!
//! ```
//! use izanagi_kit::autofs;
//! let m = b"/net\t-hosts\n/auto\t/etc/auto.misc\n/home\tauto.home --timeout=60\n";
//! assert!(autofs::detect(m));
//! let c = autofs::parse(m).unwrap();
//! assert_eq!(c.master_entries, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// auto.master 系エントリ数。
    pub master_entries: usize,
    /// maptype:map 指定 (`file:`/`program:`/`yp:`/`nisplus:`/`ldap:`/`sss:`) 数。
    pub typed_maps: usize,
    /// `-options` 付きエントリ数。
    pub optioned: usize,
    /// `+name` 包含行数。
    pub includes: usize,
}

const MAP_TYPES: &[&str] = &[
    "file", "program", "exec", "yp", "nisplus", "nis", "hesiod", "ldap", "ldaps", "sss", "dir",
    "multi", "cache",
];

/// autofs マップらしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => {
            (c.master_entries >= 2 && (c.typed_maps >= 1 || c.optioned >= 1))
                || c.typed_maps >= 2
                || (c.master_entries >= 1 && c.optioned >= 1 && c.typed_maps >= 1)
        }
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        master_entries: 0,
        typed_maps: 0,
        optioned: 0,
        includes: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('+') {
            c.includes += 1;
            continue;
        }
        let mut parts = t.split_whitespace();
        let Some(mount) = parts.next() else {
            continue;
        };
        // mount key must be an absolute path, direct-map `/-`,
        // wildcard `*` or netgroup `+name` — a bare word is not a mount
        if !(mount.starts_with('/')
            || mount == "*"
            || mount.starts_with('+')
            || mount.starts_with('-'))
        {
            continue;
        }
        let Some(map) = parts.next() else {
            continue;
        };
        // map reference: `-map`, `type:name`, `auto.*`, an `/etc`-style
        // path, or a plain word carrying `:` — a second bare word
        // (`cmd arg` in any list) does not qualify
        if !(map.starts_with('-')
            || map.contains(':')
            || map.contains('/')
            || map.starts_with("auto."))
        {
            continue;
        }
        c.master_entries += 1;
        if let Some(ty) = map.split(':').next() {
            if map.contains(':') && MAP_TYPES.contains(&ty) {
                c.typed_maps += 1;
            }
        }
        if map.starts_with('-') || parts.any(|p| p.starts_with('-')) {
            c.optioned += 1;
        }
    }
    (c.master_entries > 0 || c.includes > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &[u8] = b"/net\t-hosts\n/auto\t/etc/auto.misc\n/home\tauto.home --timeout=60\n/misc\tfile:/etc/auto.misc\n/mnt\tprogram:/etc/auto.mnt\n";

    #[test]
    fn detects_auto_master() {
        assert!(detect(MASTER));
        let c = parse(MASTER).unwrap();
        assert_eq!(c.master_entries, 5);
        assert_eq!(c.typed_maps, 2);
        assert_eq!(c.optioned, 2);
    }

    #[test]
    fn rejects_fstab() {
        let fstab = b"/dev/sda1 / ext4 defaults 0 1\n/dev/sda2 /home ext4 defaults 0 2\n";
        assert!(!detect(fstab));
    }
}
