//! Promtail `config.yaml` の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::promtailconf::parse(b"server:\n  http_listen_port: 9080\npositions:\n  filename: /tmp/positions.yaml\nclients:\n  - url: http://loki:3100/loki/api/v1/push\nscrape_configs:\n  - job_name: system\n").unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.clients, 1);
//! ```

/// 既知トップレベルキー。
const SECTIONS: &[&str] = &[
    "server",
    "positions",
    "clients",
    "scrape_configs",
    "target_config",
    "limits_config",
    "wal",
    "tracing",
    "options",
    "global",
];
/// `- ` リスト項目名の既知名。
const LIST_ITEMS: &[&str] = &[
    "job_name",
    "job-name",
    "url",
    "tenant_id",
    "basic_auth",
    "bearer_token",
    "bearer_token_file",
    "backoff_config",
    "batchwait",
    "batchsize",
    "external_labels",
    "static_configs",
    "relabel_configs",
    "pipeline_stages",
    "loki_push_api",
    "file_sd_configs",
    "kubernetes_sd_configs",
    "docker_sd_configs",
    "journal",
    "syslog",
    "gcplog",
    "windows",
    "azure_event_hubs",
    "cloudflare",
    "gelf",
    "heroku_drain",
    "kafka",
    "mqtt",
    "otelcol",
    "webhook",
    "name",
    "labels",
    "source",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルセクション数。
    pub sections: usize,
    /// scrape_configs の job 項目数。
    pub jobs: usize,
    /// clients の `- url:` 項目数。
    pub clients: usize,
    /// `- ` リスト項目の総数。
    pub items: usize,
    /// `key:` / `key: v` 行の総数。
    pub entries: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Promtail 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.clients + c.jobs >= 1)
}

/// `b` を Promtail 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        jobs: 0,
        clients: 0,
        items: 0,
        entries: 0,
        misc: 0,
    };
    let mut section = "";
    let mut sindent = 0_usize;
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
            if section == "scrape_configs" && (name == "job_name" || name == "job-name") {
                c.jobs += 1;
            } else if section == "clients" && name == "url" {
                c.clients += 1;
            } else if !LIST_ITEMS.contains(&name) {
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
            sindent = indent;
        } else if indent == sindent + 2 && key == "job_name" && section == "scrape_configs" {
            c.jobs += 1;
        }
    }
    (known >= 1 && c.entries >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn promtail() {
        let cfg = b"server:\n  http_listen_port: 9080\n  grpc_listen_port: 0\npositions:\n  filename: /tmp/positions.yaml\nclients:\n  - url: http://loki:3100/loki/api/v1/push\n    tenant_id: t1\n  - url: http://loki2:3100/loki/api/v1/push\nscrape_configs:\n  - job_name: system\n    static_configs:\n      - targets: [localhost]\n        labels:\n          job: varlogs\n          __path__: /var/log/*.log\n  - job_name: docker\n    pipeline_stages:\n      - docker: {}\nwal:\n  enabled: true\nlimits_config:\n  readline_rate_enabled: true\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.jobs, 2);
        assert_eq!(c.clients, 2);
        assert!(c.items >= 4);
    }

    #[test]
    fn not_promtail() {
        assert!(!detect(b"key: value\nfoo: bar\n"));
    }
}
