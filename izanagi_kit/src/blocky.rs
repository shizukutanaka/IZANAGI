//! blocky DNS proxy `config.yml` config census.
//!
//! blocky config top-level keys: `upstreams`, `blocking`,
//! `customDNS`, `conditional`, `caching`, `clientLookup`,
//! `queryLog`, `prometheus`, `redis`, `ports`, `fqdnOnly`,
//! `bootstrapDns`, `hostsFile`, `filtering`, `logLevel`.
//!
//! ```rust
//! let k = b"upstreams:\n  groups:\n    default: [8.8.8.8]\nblocking:\n  blackLists:\n    ads: [x]\nqueryLog:\n  type: csv\n";
//! assert!(izanagi_kit::blocky::detect(k));
//! ```

/// Blocky config census.
#[derive(Debug, Clone)]
pub struct Blocky {
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `- ` list items.
    pub items: usize,
    /// recognised top-level keys present.
    pub top_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "upstreams",
    "blocking",
    "customDNS",
    "bootstrapDns",
    "queryLog",
    "clientLookup",
    "conditional",
    "fqdnOnly",
    "prometheus",
    "redis",
    "ede",
    "filterUnmappedTypes",
    "ecsUseAsClient",
    "minTlsServeVersion",
    "disableIPv6",
    "dohUserAgent",
    "connectIPVersion",
    "influxdb",
    "postgres",
    "edns",
    "hostsFile",
    "rewrite",
    "special",
];

const WEAK: &[&str] = &[
    "ports",
    "certFile",
    "keyFile",
    "logLevel",
    "logFormat",
    "logTimestamp",
    "caching",
    "filtering",
    "download",
    "refreshPeriod",
    "prefetching",
    "prefetchExpires",
    "prefetchMaxItemsCount",
    "prefetchThreshold",
    "startVerifyUpstream",
    "timeout",
    "maxConcurrentRequests",
    "httpTimeout",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"').trim_matches('\'');
            if k.is_empty() {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

/// Detect a blocky `config.yml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // vendor-exclusive keys carry the weight; shared keys only count
    // once a strong anchor is present.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Blocky {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            items: 0,
            top_keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") {
                c.items += 1;
                continue;
            }
            if s.contains(':') {
                c.settings += 1;
                if let Some(k) = top_key(line) {
                    if STRONG.contains(&k) || WEAK.contains(&k) {
                        c.top_keys += 1;
                    }
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
        let b = b"upstreams:\n  groups:\n    default: [8.8.8.8]\nblocking:\n  blackLists:\n    ads: [x]\nqueryLog:\n  type: csv\n";
        assert!(detect(b));
        let c = Blocky::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"ports:\n  dns: 53\nlogLevel: info\n"));
        assert!(!detect(b"# blocking:\n# upstreams:\nports:\n  x: 1\n"));
    }
}
