//! Transmission `settings.json` census.
//!
//! Keys: `alt-speed-down`, `alt-speed-up`, `alt-speed-time-begin`,
//! `alt-speed-time-end`, `alt-speed-time-enabled`, `alt-speed-turtle-enabled`,
//! `announce-ip`, `announce-ip-enabled`, `anti-brute-force-enabled`,
//! `anti-brute-force-threshold`, `bind-address-ipv4`, `bind-address-ipv6`,
//! `blocklist-enabled`, `blocklist-url`, `blocklist-updates-enabled`,
//! `cache-size-mb`, `compact-view`, `dht-enabled`, `download-dir`,
//! `download-queue-enabled`, `download-queue-size`, `encryption`,
//! `idle-seeding-limit`, `idle-seeding-limit-enabled`, `incomplete-dir`,
//! `incomplete-dir-enabled`, `inhibit-desktop-hibernation`, `lpd-enabled`,
//! `peer-congestion-algorithm`, `peer-limit-global`,
//! `peer-limit-per-torrent`, `peer-port`, `peer-port-random-high`,
//! `peer-port-random-low`, `peer-port-random-on-start`, `peer-socket-tos`,
//! `pex-enabled`, `port-forwarding-enabled`, `preallocation`,
//! `prefetch-enabled`, `queue-stalled-enabled`, `queue-stalled-minutes`,
//! `ratio-limit`, `ratio-limit-enabled`, `rename-partial-files`,
//! `required`, `rpc-authentication-required`, `rpc-bind-address`,
//! `rpc-enabled`, `rpc-host-whitelist`, `rpc-host-whitelist-enabled`,
//! `rpc-password`, `rpc-port`, `rpc-url`, `rpc-username`,
//! `rpc-whitelist`, `rpc-whitelist-enabled`, `scrape-paused-torrents-enabled`,
//! `script-torrent-added-enabled`, `script-torrent-added-filename`,
//! `script-torrent-done-enabled`, `script-torrent-done-filename`,
//! `script-torrent-done-seeding-enabled`, `script-torrent-done-seeding-filename`,
//! `seed-queue-enabled`, `seed-queue-size`, `speed-limit-down`,
//! `speed-limit-down-enabled`, `speed-limit-up`, `speed-limit-up-enabled`,
//! `start-added-torrents`, `trash-original-torrent-files`, `umask`,
//! `upload-slots-per-torrent`, `utp-enabled`, `watch-dir`,
//! `watch-dir-enabled`, `watch-dir-force-generic`.
//!
//! ```rust
//! let k = b"{\n \"download-dir\": \"/x\",\n \"peer-port\": 51413,\n \"rpc-port\": 9091,\n \"dht-enabled\": true,\n \"speed-limit-up\": 100\n}\n";
//! assert!(izanagi_kit::transmission::detect(k));
//! ```

/// transmission settings.json census.
#[derive(Debug, Clone)]
pub struct Transmission {
    /// `"key": value` pairs.
    pub pairs: usize,
    /// recognised transmission keys present.
    pub keys: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "alt-speed-down",
    "alt-speed-up",
    "alt-speed-time-begin",
    "alt-speed-time-end",
    "alt-speed-time-enabled",
    "alt-speed-turtle-enabled",
    "announce-ip",
    "announce-ip-enabled",
    "anti-brute-force-enabled",
    "anti-brute-force-threshold",
    "bind-address-ipv4",
    "bind-address-ipv6",
    "blocklist-enabled",
    "blocklist-url",
    "blocklist-updates-enabled",
    "cache-size-mb",
    "dht-enabled",
    "download-dir",
    "download-queue-enabled",
    "download-queue-size",
    "encryption",
    "idle-seeding-limit",
    "idle-seeding-limit-enabled",
    "incomplete-dir",
    "incomplete-dir-enabled",
    "inhibit-desktop-hibernation",
    "lpd-enabled",
    "peer-congestion-algorithm",
    "peer-limit-global",
    "peer-limit-per-torrent",
    "peer-port",
    "peer-port-random-high",
    "peer-port-random-low",
    "peer-port-random-on-start",
    "peer-socket-tos",
    "pex-enabled",
    "port-forwarding-enabled",
    "preallocation",
    "prefetch-enabled",
    "queue-stalled-enabled",
    "queue-stalled-minutes",
    "ratio-limit",
    "ratio-limit-enabled",
    "rename-partial-files",
    "rpc-authentication-required",
    "rpc-bind-address",
    "rpc-enabled",
    "rpc-host-whitelist",
    "rpc-host-whitelist-enabled",
    "rpc-password",
    "rpc-port",
    "rpc-url",
    "rpc-username",
    "rpc-whitelist",
    "rpc-whitelist-enabled",
    "scrape-paused-torrents-enabled",
    "script-torrent-added-enabled",
    "script-torrent-added-filename",
    "script-torrent-done-enabled",
    "script-torrent-done-filename",
    "script-torrent-done-seeding-enabled",
    "script-torrent-done-seeding-filename",
    "seed-queue-enabled",
    "seed-queue-size",
    "speed-limit-down",
    "speed-limit-down-enabled",
    "speed-limit-up",
    "speed-limit-up-enabled",
    "start-added-torrents",
    "trash-original-torrent-files",
    "umask",
    "upload-slots-per-torrent",
    "utp-enabled",
    "watch-dir",
    "watch-dir-enabled",
    "watch-dir-force-generic",
    "tcp-enabled",
    "preferred-transport",
    "default-trackers",
];

fn jkey(line: &str) -> Option<&str> {
    let s = line.trim();
    if !s.starts_with('"') {
        return None;
    }
    let end = s[1..].find('"')? + 1;
    let after = s[end + 1..].trim_start();
    if after.starts_with(':') {
        Some(&s[1..end])
    } else {
        None
    }
}

/// Detect a Transmission `settings.json`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `alt-speed-*`/`download-dir`/`peer-port`/`rpc-*`/`*-enabled`
    // key vocabulary is transmission-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(k) = jkey(line) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Transmission {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            pairs: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = jkey(line) {
                c.pairs += 1;
                if KEYS.contains(&k) {
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
        let b = b"{\n \"download-dir\": \"/x\",\n \"peer-port\": 51413,\n \"rpc-port\": 9091,\n \"dht-enabled\": true,\n \"speed-limit-up\": 100\n}\n";
        assert!(detect(b));
        let c = Transmission::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"{\n \"name\": \"x\",\n \"port\": 1\n}\n"));
        assert!(!detect(b"{\n \"peer-port\": 1\n}\n"));
    }
}
