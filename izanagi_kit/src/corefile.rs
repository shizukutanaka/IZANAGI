//! CoreDNS Corefile configuration format.
//!
//! Corefile declares DNS server blocks as `zones:ports { … }` containing
//! plugin directives (`forward`, `proxy`, `etcd`, `kubernetes`, `hosts`,
//! `cache`, `log`, `errors`, `prometheus`, `health`, `ready`, `reload`,
//! `loadbalance`, `loop`, `template`, `transfer`, `whoami`, `file`,
//! `auto`, `secondary`, `metrics`, `bind`, `debug`, `root`, `acme`,
//! `cancel`, `tls`, `chaos`, `erratic`, `federation`, `geoip`,
//! `minimal`, `multiproxy`, `nsid`, `pprof`, `reverse`, `rewrite`,
//! `route53`, `sign`, `trace`, `tsig`, `view`, `acl`, `additional`,
//! `alternate`, `any`, `bufsize`, `dns64`, `dnstap`, `fanout`,
//! `import`, `local`, `metadata`, `nopttl`, `nsid`, `on`, `reload`,
//! `sequential`, `startup`, `shutdown`, `unredact`, `import`).
//!
//! ```
//! let b = concat!(
//!     "example.com {\n",
//!     "    forward . 8.8.8.8\n",
//!     "    cache 30\n",
//!     "    log\n",
//!     "}\n",
//!     ". {\n",
//!     "    forward . tls://1 1 1 1\n",
//!     "    health :8080\n",
//!     "}\n"
//! ).as_bytes();
//! assert!(izanagi_kit::corefile::detect(b));
//! let c = izanagi_kit::corefile::Corefile::parse(b).unwrap();
//! assert_eq!(c.server_blocks, 2);
//! assert_eq!(c.plugins, 5);
//! ```

/// Parsed Corefile summary.
#[derive(Debug, Clone)]
pub struct Corefile {
    /// `zones {` server-block declarations.
    pub server_blocks: usize,
    /// Plugin directive lines inside blocks.
    pub plugins: usize,
    /// Nested plugin blocks (`{` inside a directive).
    pub sub_blocks: usize,
    /// `import` directives.
    pub imports: usize,
    /// `#`/`;`/`//` comment lines.
    pub comments: usize,
    /// `forward`/`proxy`/`etcd`/`kubernetes`/`file`/`secondary`/`auto` backend plugin lines.
    pub backend: usize,
    /// `cache`/`log`/`errors`/`prometheus`/`health`/`ready`/`reload`/`loadbalance`/`loop`/`template`/`transfer`/`whoami`/`metrics`/`bind`/`debug`/`root`/`acme`/`cancel`/`tls`/`chaos`/`erratic`/`federation`/`geoip`/`minimal`/`multiproxy`/`nsid`/`pprof`/`reverse`/`rewrite`/`route53`/`sign`/`trace`/`tsig`/`view`/`acl`/`additional`/`alternate`/`any`/`bufsize`/`dns64`/`dnstap`/`fanout`/`local`/`metadata`/`nopttl`/`on`/`sequential`/`startup`/`shutdown`/`unredact`/`import` plugin lines.
    pub features: usize,
}

const PLUGINS: &[&str] = &[
    "forward",
    "proxy",
    "etcd",
    "kubernetes",
    "hosts",
    "cache",
    "log",
    "errors",
    "prometheus",
    "health",
    "ready",
    "reload",
    "loadbalance",
    "loop",
    "template",
    "transfer",
    "whoami",
    "file",
    "auto",
    "secondary",
    "metrics",
    "bind",
    "debug",
    "root",
    "acme",
    "cancel",
    "tls",
    "chaos",
    "erratic",
    "federation",
    "geoip",
    "minimal",
    "multiproxy",
    "nsid",
    "pprof",
    "reverse",
    "rewrite",
    "route53",
    "sign",
    "trace",
    "tsig",
    "view",
    "acl",
    "additional",
    "alternate",
    "any",
    "bufsize",
    "dns64",
    "dnstap",
    "fanout",
    "import",
    "local",
    "metadata",
    "nopttl",
    "on",
    "sequential",
    "startup",
    "shutdown",
    "unredact",
    "grpc",
    "header",
    "manifold",
    "monitor",
    "network",
    "parse",
    "prefer_udp",
    "ttl",
    "zone",
];
const BACKEND_PLUGINS: &[&str] = &[
    "forward",
    "proxy",
    "etcd",
    "kubernetes",
    "file",
    "secondary",
    "auto",
    "grpc",
    "hosts",
];

/// Whether the buffer looks like a Corefile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut score = 0usize;
    let mut zone_blocks = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') || tr.starts_with("//") {
            continue;
        }
        let head = tr.split_whitespace().next().unwrap_or("");
        let head = head.trim_end_matches('{');
        if PLUGINS.contains(&head) {
            score += 1;
        }
        if tr.ends_with('{') && !head.is_empty() {
            score += 1;
            // ゾーン先頭行はドメイン形(`example.org`/`example.org:53`/`localhost`)か
            // `.` / `.:port` — `server {`/`location {`/`x {` だけでは断定しない。
            if head == "."
                || head.starts_with('.')
                || head.contains(':')
                || head.contains('.')
                || head == "localhost"
            {
                zone_blocks += 1;
            }
        }
    }
    score >= 3 && zone_blocks >= 1
}

impl Corefile {
    /// Parses a Corefile summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            server_blocks: 0,
            plugins: 0,
            sub_blocks: 0,
            imports: 0,
            comments: 0,
            backend: 0,
            features: 0,
        };
        let mut depth = 0usize;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            let opens = tr.matches('{').count();
            let closes = tr.matches('}').count();
            if tr.ends_with('{') && depth == 0 {
                c.server_blocks += 1;
            } else if depth == 0 && tr.starts_with("import") {
                c.imports += 1;
            }
            let head = tr.split_whitespace().next().unwrap_or("");
            let head = head.trim_end_matches('{');
            if PLUGINS.contains(&head) && !tr.ends_with('{') && depth > 0 {
                c.plugins += 1;
                if BACKEND_PLUGINS.contains(&head) {
                    c.backend += 1;
                } else {
                    c.features += 1;
                }
            }
            if tr.ends_with('{') && depth > 0 {
                c.sub_blocks += 1;
            }
            depth += opens;
            depth = depth.saturating_sub(closes);
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_corefile() {
        let b = concat!(
            "example.com {\n",
            "    forward . 8.8.8.8\n",
            "    cache 30\n",
            "    log\n",
            "}\n",
            ". {\n",
            "    forward . tls://1 1 1 1\n",
            "    health :8080\n",
            "    errors\n",
            "}\n",
            "# tail\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Corefile::parse(b).unwrap();
        assert_eq!(c.server_blocks, 2);
        assert_eq!(c.plugins, 6);
        assert_eq!(c.backend, 2);
        assert_eq!(c.features, 4);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn parses_import() {
        let b = concat!(
            "import other.conf\n",
            ". {\n",
            "    forward . 1 1 1 1\n",
            "}\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Corefile::parse(b).unwrap();
        assert_eq!(c.imports, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo { bar }\n"));
        assert!(Corefile::parse(b"x").is_none());
    }
}
