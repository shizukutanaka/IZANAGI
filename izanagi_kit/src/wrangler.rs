//! Wrangler `wrangler.toml` config census.
//!
//! Cloudflare Workers config: `name`, `main`, `compatibility_date`,
//! `compatibility_flags`, `workers_dev`, `account_id`, `route`/`routes`,
//! `[vars]`, `[env.*]`, `[dev]`, `[build]`, `[observability]`,
//! `[[kv_namespaces]]`, `[[d1_databases]]`, `[[r2_buckets]]`,
//! `[[durable_objects.bindings]]`, `[[dispatch_namespaces]]`,
//! `[[vectorize]]`, `[[hyperdrive]]`, `[[queues.*]]`,
//! `[[analytics_engine_datasets]]`, `[[mtls_certificates]]`.
//!
//! ```rust
//! let k = b"name = \"worker\"\nmain = \"src/index.ts\"\ncompatibility_date = \"2024-01-01\"\n[[kv_namespaces]]\nbinding = \"KV\"\n";
//! assert!(izanagi_kit::wrangler::detect(k));
//! ```

/// Wrangler config census.
#[derive(Debug, Clone)]
pub struct Wrangler {
    /// `key = value` assignments.
    pub settings: usize,
    /// `[x]`/`[[x]]` section headers.
    pub sections: usize,
    /// recognised keys/sections present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "compatibility_date",
    "compatibility_flags",
    "workers_dev",
    "kv_namespaces",
    "d1_databases",
    "r2_buckets",
    "durable_objects",
    "dispatch_namespaces",
    "vectorize",
    "hyperdrive",
    "analytics_engine_datasets",
    "mtls_certificates",
    "workflows",
    "pipelines",
    "tail_consumers",
    "miniflare",
    "unsafe",
];

const WEAK: &[&str] = &[
    "name",
    "main",
    "route",
    "routes",
    "dev",
    "vars",
    "env",
    "build",
    "observability",
    "placement",
    "triggers",
    "account_id",
    "zone_id",
    "logpush",
    "usage_model",
    "assets",
    "containers",
    "ai",
    "queues",
    "site",
    "browser",
    "images",
    "send_metrics",
];

fn toml_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    let s = s.split('#').next()?.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(inner) = s.strip_prefix("[[") {
        let inner = inner.strip_suffix("]]").unwrap_or(inner);
        let inner = inner.trim().trim_matches('"');
        return inner.rsplit('.').next().filter(|k| !k.is_empty());
    }
    if let Some(inner) = s.strip_prefix('[') {
        let inner = inner.strip_suffix(']').unwrap_or(inner);
        let inner = inner.trim().trim_matches('"');
        return inner.rsplit('.').next().filter(|k| !k.is_empty());
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                k.rsplit('.').next()
            }
        }
        None => None,
    }
}

/// Detect a Wrangler `wrangler.toml`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `compatibility_date`/`workers_dev`/binding tables are
    // Cloudflare-exclusive; shared keys (`name`/`main`/…) only count
    // once a strong anchor is present.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = toml_key(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 1 && strong + weak >= 2
}

impl Wrangler {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            sections: 0,
            keys: 0,
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
            if s.starts_with('[') {
                c.sections += 1;
            } else if s.contains('=') {
                c.settings += 1;
            }
            if let Some(k) = toml_key(line) {
                if STRONG.contains(&k) || WEAK.contains(&k) {
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
        let b = b"name = \"worker\"\nmain = \"src/index.ts\"\ncompatibility_date = \"2024-01-01\"\n[[kv_namespaces]]\nbinding = \"KV\"\n";
        assert!(detect(b));
        let c = Wrangler::parse(b).unwrap();
        assert!(c.keys >= 2);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"name = \"pkg\"\nversion = \"1\"\n"));
        assert!(!detect(b"# compatibility_date = \"x\"\nname = \"x\"\n"));
    }
}
