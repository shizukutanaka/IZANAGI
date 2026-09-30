//! Census of a Syncthing `config.xml` file.
//!
//! `<configuration>` root holding `<folder id="…" path="…" …>`,
//! `<device id="…" name="…">`, `<gui>`(`<address>`/`<tls>`), `<ldap>`,
//! `<options>` (`<listenAddress>`/`<globalAnnounceEnabled>`/…),
//! `<defaults>` (`<folder>`/`<device>` templates), `<ignores>`,
//! `<minDiskFree>`/`<maxConflicts>`/`<fsWatcherEnabled>` leaves.
//! Counts folders, devices, options leaves, xml comments.
//!
//! ```rust
//! let c = izanagi_kit::syncthingconf::SyncthingConf::parse(
//!     b"<configuration>\n<folder id=\"d\" path=\"/d\"></folder>\n\
//!       <device id=\"abc\" name=\"n1\"></device>\n</configuration>\n",
//! ).unwrap();
//! assert_eq!(c.folders, 1);
//! assert_eq!(c.devices, 1);
//! ```
#![forbid(unsafe_code)]

/// syncthing config.xml census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncthingConf {
    /// `<folder id=` elements (non-`<defaults>` folder templates also counted).
    pub folders: usize,
    /// `<device id=` elements.
    pub devices: usize,
    /// `<options>`/`<gui>`/`<ldap>`/`<api>` leaf settings.
    pub options: usize,
    /// `<defaults>`/`ignoredFolders`/`ignoredDevices` misc blocks.
    pub misc: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

/// Leaf keys counted as options.
const LEAVES: &[&str] = &[
    "address",
    "tls",
    "listenAddress",
    "globalAnnounceEnabled",
    "globalAnnounceServer",
    "localAnnounceEnabled",
    "maxSendKbps",
    "maxRecvKbps",
    "reconnectionIntervalS",
    "relaysEnabled",
    "relayReconnectIntervalM",
    "startBrowser",
    "natEnabled",
    "natLeaseMinutes",
    "natRenewalMinutes",
    "natTimeoutSeconds",
    "urAccepted",
    "urSeen",
    "urUniqueId",
    "urURL",
    "urPostInsecurely",
    "urInitialDelayS",
    "autoUpgradeIntervalH",
    "upgradeToPreReleases",
    "keepTemporariesH",
    "cacheIgnoredFiles",
    "progressUpdateIntervalS",
    "limitBandwidthInLan",
    "minHomeDiskFree",
    "maxConflicts",
    "fsWatcherEnabled",
    "fsWatcherDelayS",
    "connectionLimitEnough",
    "connectionLimitMax",
    "insecureAllowOldTLSVersions",
];

/// True if `b` looks like a syncthing config.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<configuration") && (t.contains("<folder id") || t.contains("<device id"))
}

impl SyncthingConf {
    /// Parse config.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            folders: 0,
            devices: 0,
            options: 0,
            misc: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            let mut rest = l;
            while let Some(pos) = rest.find('<') {
                let after = &rest[pos + 1..];
                if after.starts_with("folder id") {
                    c.folders += 1;
                } else if after.starts_with("device id") {
                    c.devices += 1;
                } else if after.starts_with("defaults")
                    || after.starts_with("ignoredFolder")
                    || after.starts_with("ignoredDevice")
                    || after.starts_with("untrusted")
                {
                    c.misc += 1;
                } else {
                    let name: String = after
                        .chars()
                        .take_while(|ch| ch.is_alphanumeric() || *ch == '-')
                        .collect();
                    if LEAVES.contains(&name.as_str()) {
                        c.options += 1;
                    }
                }
                let Some(next) = after.find('<') else {
                    break;
                };
                rest = &after[next..];
            }
        }
        if c.folders + c.devices + c.options == 0 {
            return None;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "<configuration>\n",
            "<folder id=\"docs\" path=\"/docs\" type=\"sendreceive\"></folder>\n",
            "<folder id=\"pics\" path=\"/pics\" type=\"receiveonly\"></folder>\n",
            "<device id=\"AAAA\" name=\"n1\"></device>\n",
            "<device id=\"BBBB\" name=\"n2\"></device>\n",
            "<gui enabled=\"true\"><address>127.0.0.1:8384</address></gui>\n",
            "<options><listenAddress>default</listenAddress><globalAnnounceEnabled>true</globalAnnounceEnabled></options>\n",
            "<defaults><folder id=\"d\"></folder></defaults>\n",
            "</configuration>\n",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = SyncthingConf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.folders, 3);
        assert_eq!(c.devices, 2);
        assert_eq!(c.options, 3);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[server]\nport = 1\n"));
        assert!(SyncthingConf::parse(b"<x/>").is_none());
    }
}
