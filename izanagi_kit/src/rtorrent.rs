//! rTorrent `.rtorrent.rc` census.
//!
//! `key = value`/`method.*`/`schedule*`/`system.*`/`network.*`/
//! `dht.*`/`protocol.*`/`trackers.*`/`throttle.*`/`pieces.*`/
//! `directory*`/`session*`/`port_range`/`port_random`/`encoding*`/
//! `scgi_port`/`scgi_local`/`execute*`/`view.*`/`ui.*`/`print`/
//! `log.*`/`import`/`branch`/`method.insert`/`method.set_key`/
//! `method.insert_catch`/`d.*`/`dht.mode`/`network.port_range`/
//! `network.listen.backlog`/`network.xmlrpc.size_limit`/
//! `throttle.global_up.*`/`throttle.global_down.*`/
//! `pieces.memory.max`/`system.file.*`/`system.file.allocate`/
//! `schedule2`/`method.rparse`/`d.add_bootstrap`/`load.*`/
//! `log.open_file`/`log.execute`/`dht.statistics.*`/`ratio.*`/
//! `system.umask`/`system.cwd`/`system.pidfile`/`tracking.*`/
//! `send_buffer_size`/`receive_buffer_size`.
//!
//! ```rust
//! let k = b"directory = ~/downloads\nsession = ~/session\nport_range = 50000-51000\nscgi_port = localhost:5000\nnetwork.port_range.set = 50001\n";
//! assert!(izanagi_kit::rtorrent::detect(k));
//! ```

/// .rtorrent.rc census.
#[derive(Debug, Clone)]
pub struct Rtorrent {
    /// `key = value` assignments.
    pub settings: usize,
    /// recognised rtorrent key prefixes present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const PREFIXES: &[&str] = &[
    "directory",
    "session",
    "port_range",
    "port_random",
    "scgi_port",
    "scgi_local",
    "network.",
    "dht",
    "protocol.",
    "trackers.",
    "throttle.",
    "pieces.",
    "encoding",
    "method.",
    "schedule",
    "system.",
    "view.",
    "ui.",
    "print",
    "log.",
    "import",
    "branch",
    "d.",
    "load.",
    "send_buffer_size",
    "receive_buffer_size",
    "max_peers",
    "min_peers",
    "max_uploads",
    "download_rate",
    "upload_rate",
    "hash_read_ahead",
    "hash_interval",
    "hash_max_tries",
    "check_hash",
    "use_udp_trackers",
    "bind",
    "ip",
    "proxy_address",
    "http_proxy_address",
    "http_capath",
    "http_cacert",
    "xmlrpc_dialect",
    "max_downloads_global",
    "max_uploads_global",
    "safe_sync",
    "max_open_files",
    "max_open_sockets",
    "max_memory_usage",
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

/// Detect a `.rtorrent.rc` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `directory`/`session`/`port_range`/`scgi_port`/`schedule`/
    // `method.`/`dht`/`throttle.`/`pieces.`/`d.*` namespaces are
    // rtorrent-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if PREFIXES.iter().any(|p| k.starts_with(p)) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Rtorrent {
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
        let b = b"directory = ~/downloads\nsession = ~/session\nport_range = 50000-51000\nscgi_port = localhost:5000\nnetwork.port_range.set = 50001\n";
        assert!(detect(b));
        let c = Rtorrent::parse(b).unwrap();
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"x = 1\ny = 2\n"));
        assert!(!detect(
            b"# directory = /x\n# session = /y\n# port_range = 1\nz = 1\n"
        ));
    }
}
