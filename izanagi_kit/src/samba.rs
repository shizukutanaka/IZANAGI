//! Samba `smb.conf` パーサ。
//!
//! `[global]`/`[homes]`/`[printers]` 等既知セクションと `workgroup`/`security`/
//! `map to guest`/`vfs objects`/`valid users` 等の既知キーを計数する。
//!
//! ```
//! use izanagi_kit::samba;
//! let conf = b"[global]\nworkgroup = WORKGROUP\nsecurity = user\nmap to guest = Bad User\n[homes]\nread only = no\n";
//! assert!(samba::detect(conf));
//! let c = samba::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 2);
//! assert_eq!(c.entries, 4);
//! assert_eq!(c.known_keys, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "global", "homes", "printers", "print$", "netlogon", "profiles", "ipc$", "sysvol",
];

const KNOWN_KEYS: &[&str] = &[
    "workgroup",
    "security",
    "netbios name",
    "server string",
    "server role",
    "realm",
    "domain master",
    "local master",
    "preferred master",
    "wins server",
    "wins support",
    "dns proxy",
    "interfaces",
    "bind interfaces only",
    "log file",
    "max log size",
    "log level",
    "map to guest",
    "guest account",
    "guest ok",
    "public",
    "browseable",
    "read only",
    "writable",
    "writeable",
    "create mask",
    "directory mask",
    "force user",
    "force group",
    "valid users",
    "invalid users",
    "read list",
    "write list",
    "admin users",
    "path",
    "comment",
    "available",
    "hosts allow",
    "hosts deny",
    "vfs objects",
    "passdb backend",
    "idmap config",
    "encrypt passwords",
    "smb encrypt",
    "socket options",
    "dead time",
    "follow symlinks",
    "wide links",
    "unix extensions",
    "printcap name",
    "printing",
    "load printers",
    "usershare path",
    "usershare max shares",
    "winbind separator",
    "template shell",
    "template homedir",
    "obey pam restrictions",
    "pam password change",
    "unix password sync",
    "passwd program",
    "passwd chat",
    "machine password timeout",
];

/// `smb.conf` らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => {
            (c.known_sections >= 1 && c.known_keys >= 2) || (c.known_keys >= 3 && c.entries >= 4)
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
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim().to_ascii_lowercase();
            if KNOWN_SECTIONS.contains(&name.as_str()) {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim().to_ascii_lowercase();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'_' | b'.'))
        {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key.as_str()) {
            c.known_keys += 1;
        }
    }
    (c.entries > 0 || c.sections > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"[global]\nworkgroup = WORKGROUP\nsecurity = user\nmap to guest = Bad User\nlog file = /var/log/samba/log.%m\n[homes]\nread only = no\nbrowseable = no\n[data]\npath = /srv/data\nguest ok = yes\n";

    #[test]
    fn detects_smbconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 2);
        assert_eq!(c.entries, 8);
        assert_eq!(c.known_keys, 8);
    }

    #[test]
    fn rejects_generic_ini() {
        let ini = b"[section]\nfoo = bar\nbaz = 42\n[other]\nx = y\n";
        assert!(!detect(ini));
    }
}
