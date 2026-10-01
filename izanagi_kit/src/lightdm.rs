//! LightDM 設定ファイル (`lightdm.conf`, `lightdm.conf.d/*.conf`) パーサ。
//!
//! `[LightDM]`/`[Seat:*]`/`[XDMCPServer]`/`[VNCServer]` セクションと
//! `greeter-session`/`user-session`/`autologin-*` 等の `key = value` を計数する。
//!
//! ```
//! use izanagi_kit::lightdm;
//! let conf = b"[LightDM]\nlog-directory = /var/log/lightdm\ngreeter-session = lightdm-gtk-greeter\nuser-session = xfce\n";
//! assert!(lightdm::detect(conf));
//! let c = lightdm::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.entries, 3);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数 (`LightDM`/`Seat:*`/`XDMCPServer`/`VNCServer`/`XDMCPClient`)。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "greeter-session",
    "user-session",
    "display-setup-script",
    "display-stopped-script",
    "greeter-setup-script",
    "session-setup-script",
    "session-cleanup-script",
    "autologin-user",
    "autologin-user-timeout",
    "autologin-session",
    "autologin-in-background",
    "autologin-guest",
    "exit-on-failure",
    "minimum-display-number",
    "minimum-vt",
    "lock-memory",
    "user-authority-in-system-dir",
    "guest-account",
    "logind-check-graphical",
    "xserver-command",
    "xserver-layout",
    "xserver-config",
    "xserver-allow-tcp",
    "xserver-share",
    "xserver-hostname",
    "xserver-display-number",
    "xdmcp-manager",
    "xdmcp-port",
    "xdmcp-key",
    "greeter-hide-users",
    "greeter-show-manual-login",
    "greeter-show-remote-login",
    "allow-user-switching",
    "allow-guest",
    "guest-account-script",
    "sessions-directory",
    "remote-sessions-directory",
    "greeters-directory",
    "backup-logs",
    "dbus-service",
    "log-directory",
    "run-directory",
    "cache-directory",
    "seat-type",
    "type",
    "enabled",
    "port",
    "key",
    "listen-address",
    "pam-service",
    "pam-autologin-service",
    "command",
    "idle-timeout",
    "session-wrapper",
    "greeter-user",
];

fn known_section(name: &str) -> bool {
    name == "LightDM"
        || name.starts_with("Seat:")
        || matches!(name, "XDMCPServer" | "VNCServer" | "XDMCPClient")
}

/// 簡易判定 (既知セクション + 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 1 && c.entries >= 3 && c.known_keys >= 2
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    let mut found = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            c.sections += 1;
            if known_section(t[1..t.len() - 1].trim()) {
                c.known_sections += 1;
            }
            found = true;
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim();
        if key.is_empty() || key.bytes().any(|b| b == b' ') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            c.known_keys += 1;
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"[LightDM]\nlog-directory = /var/log/lightdm\nrun-directory = /var/run/lightdm\ngreeters-directory = /etc/lightdm\ngreeter-session = lightdm-gtk-greeter\n\n[Seat:*]\ngreeter-session = lightdm-gtk-greeter\nuser-session = xfce\nautologin-user = alice\nxserver-command = X\n\n[XDMCPServer]\nenabled = true\nport = 177\nkey = secret\n";

    #[test]
    fn detects_lightdm() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.entries, 11);
        assert_eq!(c.known_keys, 11);
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(
            b"[server]\nhost = example.com\nport = 8080\ntimeout = 5\n"
        ));
        assert!(!detect(b"[Seat:*]\nfoo = 1\n"));
    }
}
