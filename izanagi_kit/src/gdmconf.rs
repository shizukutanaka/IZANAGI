//! GDM 設定ファイル (`custom.conf`, `daemon.conf`) パーサ。
//!
//! `[daemon]`/`[security]`/`[xdmcp]`/`[chooser]`/`[debug]` セクションと
//! `AutomaticLogin*`/`TimedLogin*`/`WaylandEnable`/`DisallowTCP` 等キーを計数する。
//!
//! ```
//! use izanagi_kit::gdmconf;
//! let conf = b"[daemon]\nAutomaticLoginEnable = true\nAutomaticLogin = alice\n";
//! assert!(gdmconf::detect(conf));
//! let c = gdmconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.known_keys, 2);
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

const KNOWN_SECTIONS: &[&str] = &["daemon", "security", "xdmcp", "chooser", "debug"];

const KNOWN_KEYS: &[&str] = &[
    "AutomaticLoginEnable",
    "AutomaticLogin",
    "TimedLoginEnable",
    "TimedLogin",
    "TimedLoginDelay",
    "WaylandEnable",
    "InitialSetupEnable",
    "AddGtkModules",
    "GtkModulesList",
    "AllowRemoteAutoLogin",
    "KillInitClients",
    "DisallowTCP",
    "PasswordRequired",
    "Enable",
    "Port",
    "ListenAddress",
    "DisplaysPerHost",
    "MaxPendingIndirects",
    "MaxIndirectWaitTime",
    "MaxWaitTime",
    "MaxSessions",
    "MaxPexIndirects",
    "IncludeAll",
    "Include",
    "Exclude",
    "Greeter",
    "RemoteGreeter",
    "Xnest",
    "XdmcpTimeout",
    "RebootCommand",
    "PoweroffCommand",
    "Chooser",
    "IndirectServer",
    "Multicast",
    "MulticastAddr",
    "Debug",
    "BaseXsession",
    "DefaultSession",
    "RunXsessionsDir",
];

/// 簡易判定 (既知セクション + 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 1 && c.known_keys >= 2
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
            if KNOWN_SECTIONS.contains(&t[1..t.len() - 1].trim()) {
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

    const SAMPLE: &[u8] = b"[daemon]\nAutomaticLoginEnable = true\nAutomaticLogin = alice\nTimedLoginEnable = false\nWaylandEnable = true\n\n[security]\nDisallowTCP = true\nPasswordRequired = false\n\n[xdmcp]\nEnable = true\nPort = 177\nMaxSessions = 16\n\n[debug]\nEnable = false\n";

    #[test]
    fn detects_gdmconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.known_sections, 4);
        assert_eq!(c.entries, 10);
        assert_eq!(c.known_keys, 10);
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(b"[daemon]\nworkers = 4\nqueue = jobs\n"));
        assert!(!detect(b"[main]\nfoo = 1\nbar = 2\n"));
    }
}
