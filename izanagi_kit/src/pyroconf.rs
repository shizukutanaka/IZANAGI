//! Pyroscope `server.yaml` の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::pyroconf::parse(b"server:\n  http_listen_port: 4040\nscrape-configs:\n  - job-name: app\n    enabled-profiles: [cpu, mem]\n").unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.jobs, 1);
//! ```

/// 既知トップレベルキー(ハイフン系とアンダースコア系の両方を受理)。
const SECTIONS: &[&str] = &[
    "server",
    "log-level",
    "log_level",
    "scrape-configs",
    "scrape_configs",
    "storage",
    "self-profiling",
    "self_profiling",
    "analytics",
    "multitenancy-enabled",
    "multitenancy_enabled",
    "api-base-url",
    "api_base_url",
    "phlaredb",
    "tenant",
    "auth",
    "limits",
    "querier",
    "distributor",
    "ingester",
    "frontend",
    "query-frontend",
    "memberlist",
    "metrics-generator",
    "compactor",
    "store-gateway",
    "shutdown-marker",
    "base-url",
    "auth-token",
    "retention-policy",
    "target",
];
/// scrape-configs 項目の既知名。
const JOB_KEYS: &[&str] = &[
    "job-name",
    "job_name",
    "enabled-profiles",
    "enabled_profiles",
    "scheme",
    "scrape-interval",
    "scrape_interval",
    "scrape-timeout",
    "scrape_timeout",
    "scrape-protocols",
    "scrape_protocols",
    "sample-limit",
    "sample_limit",
    "label-limit",
    "label_limit",
    "label-name-length-limit",
    "label-value-length-limit",
    "service-discovery",
    "static-configs",
    "static_configs",
    "kubernetes-nodes",
    "kubernetes-pods",
    "kubernetes-endpoints",
    "kubernetes-services",
    "kubernetes-ingresses",
    "consul-agent",
    "eureka-agent",
    "http-targets",
    "docker-targets",
    "targets",
    "relabel-configs",
    "relabel_configs",
    "profiling-configs",
    "profiling_configs",
    "tls-config",
    "tls_config",
    "authorization",
    "basic-auth",
    "basic_auth",
    "spied-name",
    "spied_name",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルセクション数。
    pub sections: usize,
    /// scrape-configs の job 項目数。
    pub jobs: usize,
    /// `- ` リスト項目の総数。
    pub items: usize,
    /// `key:` / `key: v` 行の総数。
    pub entries: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Pyroscope 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.jobs >= 1)
}

/// `b` を Pyroscope 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        jobs: 0,
        items: 0,
        entries: 0,
        misc: 0,
    };
    let mut section = "";
    let mut known = 0;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.starts_with('-') {
            c.items += 1;
            c.entries += 1;
            let item = t.trim_start_matches('-').trim();
            let name = item.split(':').next().unwrap_or("");
            if matches!(section, "scrape-configs" | "scrape_configs")
                && (name == "job-name" || name == "job_name")
            {
                c.jobs += 1;
            } else if !JOB_KEYS.contains(&name) && !name.is_empty() {
                c.misc += 1;
            }
            continue;
        }
        let Some(colon) = t.find(':') else {
            c.misc += 1;
            continue;
        };
        c.entries += 1;
        let key = &t[..colon];
        if indent == 0 {
            if SECTIONS.contains(&key) {
                c.sections += 1;
                known += 1;
            } else {
                c.misc += 1;
            }
            section = key;
        }
    }
    (known >= 1 && c.entries >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyro() {
        let cfg = b"server:\n  http_listen_port: 4040\n  grpc_listen_port: 4041\nlog-level: info\nscrape-configs:\n  - job-name: app\n    enabled-profiles: [cpu, mem]\n    static-configs:\n      - application: myapp\n        spy-name: gospy\n        targets: [localhost]\n    scrape-interval: 10s\n  - job-name: k8s\n    kubernetes-pods:\n      - {}\nself-profiling:\n  push-url: http://localhost:4040\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.jobs, 2);
        assert!(c.items >= 3);
    }

    #[test]
    fn not_pyro() {
        assert!(!detect(b"key: value\n"));
    }
}
