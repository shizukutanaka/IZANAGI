//! qBittorrent `qBittorrent.conf` census.
//!
//! INI sections: `[BitTorrent]` (`Session\\BTProtocol`,
//! `Session\\Port`, `Session\\MaxConnections`,
//! `Session\\MaxUploads`, `Session\\GlobalDLSpeedLimit`,
//! `Session\\GlobalUPSpeedLimit`, `Session\\AlternativeGlobalDLSpeedLimit`,
//! `Session\\AlternativeGlobalUPSpeedLimit`, `Session\\AddExtensionToIncompleteFiles`,
//! `Session\\AddTrackersEnabled`, `Session\\AdditionalTrackers`,
//! `Session\\AnonymousModeEnabled`, `Session\\BandwidthSchedulerEnabled`,
//! `Session\\CategoryOptions`, `Session\\Categories`,
//! `Session\\CreateTorrentSubfolder`, `Session\\DHTEnabled`,
//! `Session\\DefaultSavePath`, `Session\\DisableAutoTMMByDefault`,
//! `Session\\DiskIOType`, `Session\\DiskQueueSize`,
//! `Session\\Encryption`, `Session\\ExcludedFileNames`,
//! `Session\\FinishedTorrentExportDirectory`,
//! `Session\\GlobalMaxSeedingMinutes`, `Session\\GlobalMaxRatio`,
//! `Session\\IgnoreSlowTorrentsForQueueing`, `Session\\IncludeOverheadInLimits`,
//! `Session\\Interface`, `Session\\InterfaceAddress`,
//! `Session\\InterfaceName`, `Session\\LSDEnabled`, `Session\\MaxActiveDownloads`,
//! `Session\\MaxActiveTorrents`, `Session\\MaxActiveUploads`,
//! `Session\\MaxConnectionsPerTorrent`, `Session\\MaxUploadsPerTorrent`,
//! `Session\\MultiConnectionsPerIpEnabled`, `Session\\PeXEnabled`,
//! `Session\\Preallocation`, `Session\\QueueingSystemEnabled`,
//! `Session\\SSL`, `Session\\SSLPort`, `Session\\ScanDirs`,
//! `Session\\SendUploadPieceSuggestions`, `Session\\SlowTorrentsDownloadRate`,
//! `Session\\SlowTorrentsInactivityTimer`, `Session\\SlowTorrentsUploadRate`,
//! `Session\\SubcategoriesEnabled`, `Session\\SuggestMode`,
//! `Session\\Tags`, `Session\\TempPath`, `Session\\TempPathEnabled`,
//! `Session\\TorrentExportDirectory`, `Session\\TorrentStopCondition`,
//! `Session\\UseOSGlobalSpeedLimit`, `Session\\uTPRateLimited`,
//! `Session\\uTPEnabled`),
//! `[Preferences]` (`WebUI\\Address`, `WebUI\\AlternativeUIEnabled`,
//! `WebUI\\AuthSubnetWhitelist`, `WebUI\\AuthSubnetWhitelistEnabled`,
//! `WebUI\\BanDuration`, `WebUI\\ClickjackingProtection`,
//! `WebUI\\CSRFProtection`, `WebUI\\CustomHTTPHeaders`,
//! `WebUI\\HTTPS\\CertificatePath`, `WebUI\\HTTPS\\Enabled`,
//! `WebUI\\HTTPS\\KeyPath`, `WebUI\\HostHeaderValidation`,
//! `WebUI\\Language`, `WebUI\\LocalHostAuth`, `WebUI\\MaxAuthenticationFailCount`,
//! `WebUI\\Password_ha1`, `WebUI\\Password_PBKDF2`, `WebUI\\Port`,
//! `WebUI\\RootFolder`, `WebUI\\SecureCookie`, `WebUI\\ServerDomains`,
//! `WebUI\\SessionTimeout`, `WebUI\\UseUPnP`, `WebUI\\Username`,
//! `General\\Locale`, `General\\PreventFromSuspendWhenDownloading`,
//! `General\\PreventFromSuspendWhenSeeding`,
//! `Downloads\\ScanDirsV2`, `Downloads\\StartInPause`,
//! `MailNotification\\enabled`, `MailNotification\\email`,
//! `MailNotification\\password`, `MailNotification\\req_auth`,
//! `MailNotification\\req_ssl`, `MailNotification\\sender`,
//! `MailNotification\\smtp_server`, `MailNotification\\username`),
//! `[Application]`/`[Core]`/`[LegalNotice]`/`[Network]` (`PortForwardingEnabled`,
//! `Proxy\\Type`, `Proxy\\IP`, `Proxy\\Port`, `Proxy\\Username`, `Proxy\\Password`,
//! `Proxy\\OnlyForTorrents`), `[SpeedWidget]`/`[RSS]`/`[ShutdownConfirm]`/
//! `[TransferList]`/`[MainWindow]`/`[GUI]`/`[Stats]`/`[Shutdown]`.
//!
//! ```rust
//! let k = b"[BitTorrent]\nSession\\DHTEnabled=true\nSession\\Port=6881\n[Preferences]\nWebUI\\Port=8080\nWebUI\\LocalHostAuth=false\n";
//! assert!(izanagi_kit::qbittorrent::detect(k));
//! ```

/// qBittorrent.conf census.
#[derive(Debug, Clone)]
pub struct Qbittorrent {
    /// `[x]` section headers.
    pub sections: usize,
    /// `key=value` assignments.
    pub settings: usize,
    /// `Session\`/`WebUI\`/`MailNotification\` prefixed keys.
    pub keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with(';') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim();
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

const PREFIXES: &[&str] = &[
    "Session\\",
    "WebUI\\",
    "MailNotification\\",
    "Downloads\\",
    "General\\",
    "Proxy\\",
    "RSS\\",
    "Stats\\",
    "OptionsDialog\\",
    "MainWindow\\",
    "TransferList\\",
    "GUI\\",
    "BitTorrent\\",
    "Network\\",
    "SpeedWidget\\",
    "Shutdown\\",
    "LegalNotice\\",
    "AutoRun\\",
    "TorrentCreator\\",
    "Application\\",
    "Core\\",
    "Meta\\",
];

/// Detect a `qBittorrent.conf` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `Session\`/`WebUI\`/`MailNotification\` backslash-namespaced
    // keys inside `[BitTorrent]`/`[Preferences]` are qbittorrent-only.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if PREFIXES.iter().any(|p| k.starts_with(p)) {
                n += 1;
            }
        }
    }
    n >= 2
}

impl Qbittorrent {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                continue;
            }
            if let Some(k) = assign_key(line) {
                c.settings += 1;
                if PREFIXES.iter().any(|p| k.starts_with(p)) {
                    c.keys += 1;
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
    fn detects() {
        let b = b"[BitTorrent]\nSession\\DHTEnabled=true\nSession\\Port=6881\n[Preferences]\nWebUI\\Port=8080\nWebUI\\LocalHostAuth=false\n";
        assert!(detect(b));
        let c = Qbittorrent::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[server]\nport=1\nhost=x\n"));
        assert!(!detect(b"# Session\\Port=1\n# WebUI\\Port=2\n[x]\ny=1\n"));
    }
}
