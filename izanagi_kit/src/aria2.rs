//! aria2 `aria2.conf` census.
//!
//! `key=value`/`key = value` (no sections): `dir`, `log`,
//! `max-concurrent-downloads`, `check-integrity`, `continue`,
//! `daemon`, `enable-rpc`, `rpc-listen-all`, `rpc-listen-port`,
//! `rpc-secret`, `rpc-max-request-size`, `rpc-secure`,
//! `rpc-certificate`, `rpc-private-key`, `rpc-user`, `rpc-passwd`,
//! `max-download-limit`, `max-overall-download-limit`,
//! `max-overall-upload-limit`, `max-upload-limit`,
//! `max-connection-per-server`, `min-split-size`, `split`,
//! `piece-length`, `max-piece-length`, `file-allocation`,
//! `file-pread`, `no-file-allocation-limit`, `optimize-concurrent-downloads`,
//! `conditional-get`, `disable-ipv6`, `enable-dht`, `enable-dht6`,
//! `dht-listen-port`, `dht-entry-point`, `dht-entry-point6`,
//! `dht-file-path`, `dht-file-path6`, `dht-listen-addr`,
//! `dht-listen-addr6`, `dht-message-timeout`, `dht-secret`,
//! `listen-port`, `enable-peer-exchange`, `peer-agent`,
//! `peer-id-prefix`, `peer-protocol`, `bt-max-peers`,
//! `bt-request-peer-speed-limit`, `bt-tracker`,
//! `bt-tracker-connect-timeout`, `bt-tracker-interval`,
//! `bt-tracker-timeout`, `bt-external-ip`, `bt-lpd`,
//! `bt-enable-lpd`, `bt-detach-seed-only`, `bt-force-encryption`,
//! `bt-hash-check-seed`, `bt-load-saved-metadata`, `bt-max-open-files`,
//! `bt-metadata-only`, `bt-min-crypto-level`, `bt-prioritize-piece`,
//! `bt-remove-unselected-file`, `bt-require-crypto`,
//! `bt-save-metadata`, `bt-seed-unverified`, `bt-stop-timeout`,
//! `bt-tracker-max-announces`, `bt-tracker-allow-aux-announces`,
//! `seed-ratio`, `seed-time`, `follow-torrent`, `save-session`,
//! `input-file`, `save-session-interval`, `force-save`, `retry-wait`,
//! `max-tries`, `timeout`, `connect-timeout`, `http-user`, `http-passwd`,
//! `proxy`, `http-proxy`, `https-proxy`, `ftp-proxy`, `all-proxy`,
//! `no-proxy`, `referer`, `user-agent`, `load-cookies`,
//! `show-files`, `select-file`, `index-out`, `uri-selector`,
//! `on-download-start`, `on-download-complete`, `on-download-error`,
//! `on-download-stop`, `on-bt-download-complete`, `disk-cache`,
//! `enable-mmap`, `summary-interval`, `quiet`, `remote-time`,
//! `reuse-uri`, `truncate-console-readout`, `pause`, `pause-metadata`,
//! `bt-save-metadata`, `bt-hash-check-seed`, `pause-metadata`.
//!
//! ```rust
//! let k = b"dir=/downloads\nmax-concurrent-downloads=5\nenable-rpc=true\nrpc-listen-port=6800\nsplit=16\n";
//! assert!(izanagi_kit::aria2::detect(k));
//! ```

/// aria2.conf census.
#[derive(Debug, Clone)]
pub struct Aria2 {
    /// `key=value` assignments.
    pub settings: usize,
    /// recognised aria2 keys present.
    pub keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "dir",
    "max-concurrent-downloads",
    "check-integrity",
    "continue",
    "enable-rpc",
    "rpc-listen-all",
    "rpc-listen-port",
    "rpc-secret",
    "rpc-max-request-size",
    "rpc-secure",
    "rpc-certificate",
    "rpc-private-key",
    "rpc-user",
    "rpc-passwd",
    "max-download-limit",
    "max-overall-download-limit",
    "max-overall-upload-limit",
    "max-upload-limit",
    "max-connection-per-server",
    "min-split-size",
    "split",
    "piece-length",
    "max-piece-length",
    "file-allocation",
    "no-file-allocation-limit",
    "optimize-concurrent-downloads",
    "conditional-get",
    "disable-ipv6",
    "enable-dht",
    "enable-dht6",
    "dht-listen-port",
    "dht-entry-point",
    "dht-entry-point6",
    "dht-file-path",
    "dht-file-path6",
    "dht-listen-addr",
    "dht-listen-addr6",
    "dht-message-timeout",
    "dht-secret",
    "listen-port",
    "enable-peer-exchange",
    "peer-agent",
    "peer-id-prefix",
    "peer-protocol",
    "bt-max-peers",
    "bt-request-peer-speed-limit",
    "bt-tracker",
    "bt-tracker-connect-timeout",
    "bt-tracker-interval",
    "bt-tracker-timeout",
    "bt-external-ip",
    "bt-lpd",
    "bt-enable-lpd",
    "bt-detach-seed-only",
    "bt-force-encryption",
    "bt-hash-check-seed",
    "bt-load-saved-metadata",
    "bt-max-open-files",
    "bt-metadata-only",
    "bt-min-crypto-level",
    "bt-prioritize-piece",
    "bt-remove-unselected-file",
    "bt-require-crypto",
    "bt-save-metadata",
    "bt-seed-unverified",
    "bt-stop-timeout",
    "bt-tracker-max-announces",
    "seed-ratio",
    "seed-time",
    "follow-torrent",
    "save-session",
    "input-file",
    "save-session-interval",
    "force-save",
    "retry-wait",
    "max-tries",
    "timeout",
    "connect-timeout",
    "http-user",
    "http-passwd",
    "proxy",
    "http-proxy",
    "https-proxy",
    "ftp-proxy",
    "all-proxy",
    "no-proxy",
    "referer",
    "user-agent",
    "load-cookies",
    "show-files",
    "select-file",
    "index-out",
    "uri-selector",
    "on-download-start",
    "on-download-complete",
    "on-download-error",
    "on-download-stop",
    "on-bt-download-complete",
    "disk-cache",
    "enable-mmap",
    "summary-interval",
    "quiet",
    "remote-time",
    "reuse-uri",
    "truncate-console-readout",
    "pause",
    "pause-metadata",
    "bt-tracker-allow-aux-announces",
    "stream-piece-selector",
    "piece-selector",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with(';') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim();
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect an `aria2.conf` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `bt-*`/`dht-*`/`rpc-*`/`seed-*`/`split`/`max-concurrent-downloads`
    // are aria2-exclusive key names.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if KEYS.contains(&k) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Aria2 {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
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
            if let Some(k) = assign_key(line) {
                c.settings += 1;
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
        let b = b"dir=/downloads\nmax-concurrent-downloads=5\nenable-rpc=true\nrpc-listen-port=6800\nsplit=16\n";
        assert!(detect(b));
        let c = Aria2::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"x=1\ny=2\n"));
        assert!(!detect(b"# dir=/x\n# enable-rpc=true\n# split=1\nz=1\n"));
    }
}
