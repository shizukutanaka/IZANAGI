//! KeePassXC `keepassxc.ini` census.
//!
//! KeePassXC stores settings in Qt INI: `[General]`/`[GUI]`/
//! `[Browser]`/`[Security]`/`[PasswordGenerator]`/`[Proxy]`/`[Cli]`/
//! `[SSHA]`/`[AutoType]`/`[FdoSecrets]`/`[SSHAgent]` sections plus
//! `key=value` keys — `SingleInstance`, `MinimizeToTray`,
//! `AutoTypeEntryTitleMatch`, `AutoTypeEntryURLMatch`, `AutoTypeDelay`,
//! `BrowserIntegration_Enabled`, `AutoSaveAfterEveryChange`,
//! `RememberDatabases`, `ClearClipboardTimeout`, `HideUsernames`,
//! `HidePasswords`, `DataFile`, `LastOpenedFiles`, `LastDatabases`.
//!
//! ```rust
//! let c = izanagi_kit::keepassxc::Keepassxc::parse(b"[General]\nSingleInstance=true\nMinimizeToTray=false\n").unwrap();
//! assert_eq!(c.sections, 1);
//! ```

/// KeePassXC `keepassxc.ini` census.
#[derive(Debug, Clone)]
pub struct Keepassxc {
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` settings.
    pub settings: usize,
    /// `true`/`false` settings.
    pub booleans: usize,
    /// `;`/`#` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "SingleInstance",
    "MinimizeToTray",
    "MinimizeOnStartup",
    "MinimizeOnClose",
    "AutoTypeEntryTitleMatch",
    "AutoTypeEntryURLMatch",
    "AutoTypeDelay",
    "AutoTypeStartDelay",
    "BrowserIntegration_Enabled",
    "BrowserIntegration_ShowNotification",
    "AutoSaveAfterEveryChange",
    "AutoSaveOnExit",
    "RememberDatabases",
    "ClearClipboardTimeout",
    "HideUsernames",
    "HidePasswords",
    "LastOpenedFiles",
    "LastDatabases",
    "OpenPreviousDatabasesOnStartup",
    "BackupBeforeSave",
    "ConfirmDelete",
    "UseAtomicSaves",
    "lockDatabasesAfterInactivity",
    "inactivityTimeoutMinutes",
    "dropToBackgroundOnCopy",
    "iconBadgeType",
    "defaultAutoTypeSequence",
    "autoTypeDelay",
    "trayIconAppearance",
    "updateChecksEnabled",
    "searchAfterEveryChange",
];

/// Whether the buffer looks like keepassxc.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    KEYS.iter().filter(|k| t.contains(**k)).count() >= 2
        || (t.contains("[General]") && (t.contains("KeePassXC") || t.contains("keepassxc")))
}

impl Keepassxc {
    /// Parse keepassxc.ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            booleans: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') || s.starts_with('#') {
                c.comments += 1;
            } else if s.starts_with('[') && s.contains(']') {
                c.sections += 1;
            } else if s.contains('=') {
                c.settings += 1;
                let v = s.split('=').nth(1).unwrap_or("").trim();
                if v == "true" || v == "false" {
                    c.booleans += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ini() {
        let b = concat!(
            "[General]\n",
            "SingleInstance=true\n",
            "MinimizeToTray=false\n",
            "AutoTypeDelay=25\n",
            "[GUI]\n",
            "CompactMode=false\n",
            "HideUsernames=false\n",
            "[Browser]\n",
            "BrowserIntegration_Enabled=true\n",
            "AutoSaveAfterEveryChange=true\n",
            "; comment\n",
        );
        let c = Keepassxc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.settings, 7);
        assert_eq!(c.booleans, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Keepassxc::parse(b"[foo]\nx=1\n").is_none());
    }
}
