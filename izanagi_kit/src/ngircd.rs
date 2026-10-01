//! ngIRCd `ngircd.conf` — `[Global]`/`[Limits]`/`[Options]`/`[SSL]`/
//! `[Operator]`/`[Server]`/`[Channel]` セクションの INI 形式。
//!
//! ```
//! let cfg = b"[Global]\nName = irc.example.net\nInfo = ExampleNet\nPorts = 6667\n[Limits]\nMaxConnections = 500\n[Options]\nChrootDir = /var/empty\n[Operator]\nName = ume\nPassword = secret\n";
//! assert!(izanagi_kit::ngircd::detect(cfg));
//! let c = izanagi_kit::ngircd::parse(cfg).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.known_sections, 4);
//! ```

/// ngIRCd の既知セクション名。
const KNOWN_SECTIONS: &[&str] = &[
    "Global", "Limits", "Options", "SSL", "Operator", "Server", "Channel", "Features",
];

/// ngIRCd の既知キー (代表)。
const KNOWN_KEYS: &[&str] = &[
    "Name",
    "Info",
    "AdminInfo1",
    "AdminInfo2",
    "AdminEMail",
    "Listen",
    "MotdFile",
    "MotdPhrase",
    "Network",
    "Password",
    "PidFile",
    "Ports",
    "ServerGID",
    "ServerUID",
    "ChrootDir",
    "CloakHost",
    "CloakHostSalt",
    "CloakUserToNick",
    "DefaultUserModes",
    "DNS",
    "Ident",
    "IncludeDir",
    "MorePrivacy",
    "NoticeBeforeRegistration",
    "OperCanUseMode",
    "OperChanPMode",
    "OperServerMode",
    "PAM",
    "PAMIsOptional",
    "RequireAuthPing",
    "ScrubCTCP",
    "SyslogFacility",
    "WebircIP",
    "WebircPassword",
    "MaxConnections",
    "MaxConnectionsIP",
    "MaxJoins",
    "MaxNickLength",
    "MaxPenaltyTime",
    "PingTimeout",
    "PongTimeout",
    "ConnectRetry",
    "IdleTimeout",
    "MaxListSize",
    "CertFile",
    "KeyFile",
    "DHFile",
    "CipherList",
    "Host",
    "Port",
    "MyPassword",
    "PeerPassword",
    "ServiceMask",
    "Topic",
    "Modes",
    "KeyFile",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `[Name]` セクション数。
    pub sections: usize,
    /// 既知名のセクション数。
    pub known_sections: usize,
    /// `Key = Value` 行数。
    pub entries: usize,
    /// 既知キーの行数。
    pub known_keys: usize,
    /// `;`/`#` コメント行数。
    pub comments: usize,
}

/// ngircd.conf らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 1 && c.entries >= 3 && c.known_keys >= 2
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
        comments: 0,
    };
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if t.starts_with(';') || t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            let name = &t[1..t.len() - 1];
            if name.bytes().all(|ch| ch.is_ascii_alphabetic()) {
                c.sections += 1;
                if KNOWN_SECTIONS.contains(&name) {
                    c.known_sections += 1;
                }
                continue;
            }
        }
        if let Some((key, _)) = t.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_')
            {
                c.entries += 1;
                if KNOWN_KEYS.contains(&key) {
                    c.known_keys += 1;
                }
            }
        }
    }
    if c.sections == 0 && c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"; ngircd.conf\n[Global]\nName = irc.example.net\nAdminInfo1 = Ume\nAdminEMail = ume@example.net\nPorts = 6667\nMotdFile = /etc/ngircd/motd\n[Limits]\nMaxConnections = 500\nPingTimeout = 120\n[Options]\nChrootDir = /var/empty\nIdent = yes\n[SSL]\nCertFile = /etc/ssl/c.pem\nKeyFile = /etc/ssl/k.pem\n[Channel]\nName = #iz\nTopic = room\n";

    #[test]
    fn detects_ngircd() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.known_sections, 5);
        assert_eq!(c.entries, 13);
        assert_eq!(c.known_keys, 13);
    }

    #[test]
    fn rejects_ini() {
        assert!(!detect(b"[ui]\ncolor = red\nwidth = 3\nheight = 4\n"));
        assert!(!detect(b"[Global]\nx = 1\n"));
    }
}
