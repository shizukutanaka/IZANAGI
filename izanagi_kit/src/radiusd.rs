//! FreeRADIUS `radiusd.conf` / `clients.conf` census.
//!
//! `key = value` assignments plus `name {` … `}` blocks (`client`,
//! `listen`, `home_server`, `home_server_pool`, `realm`, `proxy server`,
//! `modules`, `instantiate`, `security`, `thread pool`, `log`).
//!
//! ```rust
//! let r = b"client localhost {\n\tipaddr = 127.0.0.1\n\tsecret = testing123\n\tnas_type = other\n}\nclient lan {\n\tipaddr = 192.0.2.0/24\n\tsecret = pw\n}\n";
//! assert!(izanagi_kit::radiusd::detect(r));
//! let c = izanagi_kit::radiusd::Radiusd::parse(r).unwrap();
//! assert_eq!(c.blocks, 2);
//! ```

/// radiusd.conf census.
#[derive(Debug, Clone)]
pub struct Radiusd {
    /// `key = value` lines matching a known FreeRADIUS config key.
    pub settings: usize,
    /// `name … {` block-open lines matching a known block type.
    pub blocks: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Assignment keys used in radiusd.conf / clients.conf / proxy.conf.
const KEYS: &[&str] = &[
    "allow_core_dumps",
    "cleanup_delay",
    "coa_server",
    "default_server",
    "host",
    "hostname_lookups",
    "ipaddr",
    "key",
    "libdir",
    "logdir",
    "max_request_time",
    "max_requests",
    "name",
    "nas_type",
    "netmask",
    "password",
    "pidfile",
    "port",
    "prefix",
    "proto",
    "protocol",
    "raddbdir",
    "require_message_authenticator",
    "response_window",
    "revive_interval",
    "sbindir",
    "secret",
    "shortname",
    "src_ipaddr",
    "status_check",
    "type",
    "user",
    "virtual_server",
    "zombie_period",
];

/// Block names (`name … {`; first word before any value).
const BLOCKS: &[&str] = &[
    "client",
    "clients",
    "home_server",
    "home_server_pool",
    "instantiate",
    "listen",
    "log",
    "modules",
    "proxy",
    "realm",
    "security",
    "server",
    "thread",
];

fn assign_key(l: &str) -> Option<&str> {
    let t = l.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    if k.is_empty() || k.contains(' ') {
        return None;
    }
    Some(k)
}

fn is_block(t: &str) -> bool {
    let Some(open) = t.find('{') else {
        return false;
    };
    let head = t[..open].trim();
    let name = head.split(char::is_whitespace).next().unwrap_or("");
    BLOCKS.contains(&name)
}

/// Detect a FreeRADIUS config file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut keys = 0usize;
    let mut blocks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_block(tr) {
            blocks += 1;
            continue;
        }
        if let Some(k) = assign_key(l) {
            if KEYS.contains(&k) {
                keys += 1;
            }
        }
    }
    (blocks >= 1 && keys >= 2) || keys >= 4
}

impl Radiusd {
    /// Count settings and blocks. Returns `None` when the input does not
    /// look like a FreeRADIUS config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            blocks: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if is_block(tr) {
                c.blocks += 1;
                continue;
            }
            if let Some(k) = assign_key(l) {
                if KEYS.contains(&k) {
                    c.settings += 1;
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
    fn detects_clients() {
        let b = b"# clients.conf\nclient localhost {\n\tipaddr = 127.0.0.1\n\tsecret = testing123\n\tshortname = localhost\n\tnas_type = other\n}\nclient lan {\n\tipaddr = 192.0.2.0/24\n\tsecret = pw\n\tproto = udp\n}\n";
        assert!(detect(b));
        let c = Radiusd::parse(b).unwrap();
        assert_eq!(c.blocks, 2);
        assert_eq!(c.settings, 7);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn detects_radiusd() {
        let b = b"prefix = /usr\nlibdir = /usr/lib/freeradius\nlogdir = /var/log/radius\nraddbdir = /etc/raddb\nname = freeradius\nmax_request_time = 30\ncleanup_delay = 5\nmax_requests = 16384\nhostname_lookups = no\nallow_core_dumps = no\nlog {\n\tdestination = files\n}\nsecurity {\n}\nmodules {\n}\n";
        assert!(detect(b));
        let c = Radiusd::parse(b).unwrap();
        assert_eq!(c.settings, 10);
        assert_eq!(c.blocks, 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(
            b"rocommunity public\nagentaddress udp:161\nsyslocation x\n"
        ));
        assert!(!detect(b"a=1\nb=2\n"));
        assert!(Radiusd::parse(b"").is_none());
    }
}
