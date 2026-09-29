//! Parser for Vector configuration files (`vector.toml` / `vector.yaml`).
//!
//! Counts `[sources.x]`/`[transforms.x]`/`[sinks.x]`/enrichment/secret
//! components, `type = "…"` values, `inputs = […]` wiring references,
//! `encoding`/`healthcheck` options, and comments.
//!
//! ```
//! let b = b"[sources.in]\ntype = \"stdin\"\n[sinks.out]\ntype = \"console\"\ninputs = [\"in\"]\nencoding.codec = \"json\"\n";
//! assert!(izanagi_kit::vector::detect(b));
//! let c = izanagi_kit::vector::Vector::parse(b).unwrap();
//! assert_eq!(c.sources, 1);
//! assert_eq!(c.sinks, 1);
//! assert_eq!(c.wiring_refs, 1);
//! ```

/// Parsed Vector configuration summary.
#[derive(Debug, Clone)]
pub struct Vector {
    /// `[sources.X]` components.
    pub sources: usize,
    /// `[transforms.X]` components.
    pub transforms: usize,
    /// `[sinks.X]` components.
    pub sinks: usize,
    /// `[enrichment_tables.X]`/`[secret.X]` components.
    pub other_components: usize,
    /// `type = "…"` declarations.
    pub type_decls: usize,
    /// `inputs = […]` wiring lines.
    pub wiring_refs: usize,
    /// `encoding*`/`codec`/`compression`/`batch`/`buffer`/`request`/`tls` option lines.
    pub io_options: usize,
    /// `healthcheck*` options.
    pub healthchecks: usize,
    /// `[tests.X]` unit test blocks.
    pub tests: usize,
    /// `remap`/`vrl` transform blocks with `.` expressions.
    pub remaps: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const IO_KEYS: &[&str] = &[
    "encoding",
    "codec",
    "compression",
    "batch",
    "buffer",
    "request",
    "tls",
    "framing",
];

/// Returns `true` when the bytes look like a Vector config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[sources.") || t.contains("[sinks.") || t.contains("[transforms."))
        && (t.contains("type =") || t.contains("inputs ="))
}

impl Vector {
    /// Parses a Vector configuration, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            sources: 0,
            transforms: 0,
            sinks: 0,
            other_components: 0,
            type_decls: 0,
            wiring_refs: 0,
            io_options: 0,
            healthchecks: 0,
            tests: 0,
            remaps: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('[') {
                let inner = tr.trim_start_matches('[').trim_end_matches(']');
                let fam = inner.split('.').next().unwrap_or("");
                match fam {
                    "sources" => c.sources += 1,
                    "transforms" => c.transforms += 1,
                    "sinks" => c.sinks += 1,
                    "tests" => c.tests += 1,
                    "enrichment_tables" | "secret" => c.other_components += 1,
                    _ => {}
                }
                continue;
            }
            if let Some((k, v)) = tr.split_once('=') {
                let key = k.trim().trim_matches('"');
                let val = v.trim();
                if key == "type" {
                    c.type_decls += 1;
                }
                if key == "inputs" && val.starts_with('[') {
                    c.wiring_refs += 1;
                }
                let base = key.split('.').next().unwrap_or("");
                if IO_KEYS.contains(&base) || key.starts_with("encoding") {
                    c.io_options += 1;
                }
                if key.starts_with("healthcheck") {
                    c.healthchecks += 1;
                }
                if key == "source" {
                    c.remaps += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# v\n[sources.stdin_src]\ntype = \"stdin\"\n\n[transforms.sample]\ntype = \"remap\"\ninputs = [\"stdin_src\"]\nsource = '''\n  .parsed = parse_json!(.message)\n'''\n\n[sinks.console_out]\ntype = \"console\"\ninputs = [\"sample\"]\nencoding.codec = \"json\"\nhealthcheck.enabled = false\n";

    #[test]
    fn parses_vector() {
        let c = Vector::parse(CONF).unwrap();
        assert_eq!(c.sources, 1);
        assert_eq!(c.transforms, 1);
        assert_eq!(c.sinks, 1);
        assert_eq!(c.type_decls, 3);
        assert_eq!(c.wiring_refs, 2);
        assert_eq!(c.io_options, 1);
        assert_eq!(c.healthchecks, 1);
        assert_eq!(c.remaps, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_vector() {
        assert!(!detect(b"[server]\nport = 8080"));
        assert!(Vector::parse(b"x").is_none());
    }
}
