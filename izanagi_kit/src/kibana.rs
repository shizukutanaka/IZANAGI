//! Kibana `kibana.yml` config census.
//!
//! Kibana top-level keys: `server.port`, `server.host`,
//! `server.name`, `server.publicBaseUrl`, `server.basePath`,
//! `server.uuid`, `server.maxPayloadBytes`, `server.xsrf.*`,
//! `elasticsearch.hosts`, `elasticsearch.username`,
//! `elasticsearch.password`, `elasticsearch.serviceAccountToken`,
//! `elasticsearch.ssl.*`, `kibana.index`, `kibana.defaultAppId`,
//! `kibana.defaultLocale`, `savedObjects.*`, `xpack.*`,
//! `monitoring.*`, `i18n.locale`, `logging.*`, `ops.interval`,
//! `telemetry.*`, `migrations.*`, `status.allowAnonymous`,
//! `csp.*`, `pid.file`, `reporting.*`, `map.*`, `console.*`,
//! `vis_type.*`, `enterpriseSearch.*`, `newsfeed.*`, `home.*`,
//! `usageCollection.*`, `screenshotting.*`, `data.autocomplete.*`.
//!
//! ```rust
//! let k = b"server.port: 5601\nserver.host: 0.0.0.0\nserver.name: kibana\nelasticsearch.hosts: [\"http://es:9200\"]\ni18n.locale: en\n";
//! assert!(izanagi_kit::kibana::detect(k));
//! ```

/// Kibana config census.
#[derive(Debug, Clone)]
pub struct Kibana {
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
    "server.port",
    "server.host",
    "server.name",
    "server.publicBaseUrl",
    "server.basePath",
    "server.rewriteBasePath",
    "server.uuid",
    "server.maxPayloadBytes",
    "server.xsrf",
    "server.customResponseHeaders",
    "server.ssl",
    "server.securityResponseHeaders",
    "elasticsearch.hosts",
    "elasticsearch.username",
    "elasticsearch.password",
    "elasticsearch.serviceAccountToken",
    "elasticsearch.ssl",
    "kibana.index",
    "kibana.defaultAppId",
    "kibana.defaultLocale",
    "kibana.autocompleteTerminateAfter",
    "kibana.autocompleteTimeout",
    "savedObjects.maxImportExportSize",
    "savedObjects.maxImportPayloadBytes",
    "savedObjects.permissionCheckingEnabled",
    "status.allowAnonymous",
    "i18n.locale",
    "ops.interval",
    "pid.file",
    "csp.rules",
    "csp.strict",
    "csp.warnLegacyBrowsers",
    "enterpriseSearch.accessToken",
    "enterpriseSearch.host",
    "data.autocomplete.valueSuggestions.timeout",
    "data.autocomplete.valueSuggestions.terminateAfter",
];

const WEAK: &[&str] = &[
    "server",
    "elasticsearch",
    "kibana",
    "logging",
    "xpack",
    "monitoring",
    "savedObjects",
    "i18n",
    "ops",
    "telemetry",
    "csp",
    "plugins",
    "console",
    "reporting",
    "map",
    "newsfeed",
    "home",
    "status",
    "migrations",
    "data",
    "usageCollection",
    "screenshotting",
    "vis_type",
    "vis_builder",
    "timelion",
    "vega",
    "regionmap",
    "tilemap",
    "externalUrl",
    "interpreter",
    "path",
    "statistics",
    "cpu",
    "cpuacct",
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

/// Detect a `kibana.yml`.
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

impl Kibana {
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
        let b = b"server.port: 5601\nserver.host: 0.0.0.0\nserver.name: kibana\nelasticsearch.hosts: [\"http://es:9200\"]\ni18n.locale: en\n";
        assert!(detect(b));
        let c = Kibana::parse(b).unwrap();
        assert!(c.top_keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"server:\n  port: 8080\nlogging:\n  x: y\n"));
        assert!(!detect(
            b"# server.port: 5601\n# elasticsearch.hosts: [x]\nserver:\n  x: 1\n"
        ));
    }
}
