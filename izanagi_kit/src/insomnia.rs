//! Insomnia REST client export (YAML or JSON) —
//! `_type: export` + `__export_format` + `resources:` with `_type:` kinds.

use core::str::from_utf8;

#[derive(Debug, Clone)]
/// Parsed census of an Insomnia export.
pub struct Insomnia {
    /// `__export_format` (e.g. 4).
    pub format: usize,
    /// `_type: workspace` / `"_type": "workspace"` resources.
    pub workspaces: usize,
    /// `_type: request` resources.
    pub requests: usize,
    /// `_type: request_group` folders.
    pub groups: usize,
    /// `_type: environment` resources.
    pub environments: usize,
    /// `_type: api_spec` resources.
    pub api_specs: usize,
    /// `_type: cookie_jar` resources.
    pub cookie_jars: usize,
    /// `_type: unit_test*` resources.
    pub unit_tests: usize,
    /// `_type: grpc_request` resources.
    pub grpc_requests: usize,
    /// `_type: websocket_*` resources.
    pub websockets: usize,
    /// `method:` entries inside requests.
    pub methods: usize,
    /// `url:` entries inside requests.
    pub urls: usize,
    /// `name:` entries.
    pub names: usize,
    /// Total `resources:` items (counted by `_id:`).
    pub resources: usize,
}

fn count_yaml(t: &str, kind: &str) -> usize {
    let yaml = ["_type: ", kind].concat();
    let json = ["\"_type\": \"", kind, "\""].concat();
    t.lines()
        .filter(|l| l.trim() == yaml || l.trim().starts_with(&json))
        .count()
}

fn count_lines(t: &str, key: &str) -> usize {
    t.lines()
        .filter(|l| l.trim().starts_with(key) && l.trim().contains(':'))
        .count()
}

/// `true` when the text looks like an Insomnia export.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = from_utf8(b) else {
        return false;
    };
    (t.contains("_type: export") || t.contains("\"_type\": \"export\"")) && t.contains("resources")
}

impl Insomnia {
    #[must_use]
    /// Parses `b` into `Insomnia`.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let fmt = t
            .find("__export_format")
            .and_then(|i| {
                t[i + "__export_format".len()..]
                    .trim_start_matches(|c: char| c == ':' || c == '"' || c.is_whitespace())
                    .split(['\n', ','])
                    .next()
                    .and_then(|v| v.trim_matches('"').parse::<usize>().ok())
            })
            .unwrap_or(0);
        Some(Self {
            format: fmt,
            workspaces: count_yaml(t, "workspace"),
            requests: count_yaml(t, "request"),
            groups: count_yaml(t, "request_group"),
            environments: count_yaml(t, "environment"),
            api_specs: count_yaml(t, "api_spec"),
            cookie_jars: count_yaml(t, "cookie_jar"),
            unit_tests: count_yaml(t, "unit_test") + count_yaml(t, "unit_test_suite"),
            grpc_requests: count_yaml(t, "grpc_request"),
            websockets: count_yaml(t, "websocket_request") + count_yaml(t, "websocket_payload"),
            methods: count_lines(t, "method") + t.matches("\"method\"").count(),
            urls: count_lines(t, "url") + t.matches("\"url\"").count(),
            names: count_lines(t, "name") + t.matches("\"name\"").count(),
            resources: t
                .lines()
                .filter(|l| {
                    l.trim()
                        .trim_start_matches('-')
                        .trim_start()
                        .starts_with("_id:")
                })
                .count()
                + t.matches("\"_id\"").count(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &[u8] = b"_type: export\n__export_format: 4\n__export_source: insomnia.desktop\nresources:\n  - _id: req_1\n    _type: request\n    name: list\n    method: GET\n    url: https://x/u\n  - _id: req_2\n    _type: request\n    name: create\n    method: POST\n    url: https://x/u\n  - _id: fld_1\n    _type: request_group\n    name: users\n  - _id: env_1\n    _type: environment\n    name: base\n  - _id: wrk_1\n    _type: workspace\n    name: shop\n";

    #[test]
    fn detects_insomnia() {
        assert!(detect(FIX));
        assert!(!detect(b"resources: []"));
    }

    #[test]
    fn parses_insomnia() {
        let i = Insomnia::parse(FIX).unwrap();
        assert_eq!(i.format, 4);
        assert_eq!(i.workspaces, 1);
        assert_eq!(i.requests, 2);
        assert_eq!(i.groups, 1);
        assert_eq!(i.environments, 1);
        assert_eq!(i.methods, 2);
        assert_eq!(i.urls, 2);
        assert_eq!(i.resources, 5);
        assert!(Insomnia::parse(b"").is_none());
    }
}
