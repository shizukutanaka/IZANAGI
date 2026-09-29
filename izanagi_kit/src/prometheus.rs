//! Parser for Prometheus configuration files (`prometheus.yml`).
//!
//! Counts `global:`/`rule_files:`/`scrape_configs:`/`alerting:` sections,
//! `- job_name:` scrape jobs, `static_configs:`/`targets:`/`*_sd_configs:`
//! service discovery, `relabel_configs:`/`metric_relabel_configs:`/
//! `alert_relabel_configs:`, `remote_write`/`remote_read`/`alertmanagers`,
//! and `scrape_interval`/`evaluation_interval`/`scrape_timeout` keys.
//!
//! ```
//! let b = b"global:\n  scrape_interval: 15s\nscrape_configs:\n  - job_name: node\n    static_configs:\n      - targets: [\"h:9100\"]\n";
//! assert!(izanagi_kit::prometheus::detect(b));
//! let c = izanagi_kit::prometheus::Prometheus::parse(b).unwrap();
//! assert_eq!(c.scrape_configs, 1);
//! assert_eq!(c.jobs, 1);
//! ```

/// Parsed prometheus.yml summary.
#[derive(Debug, Clone)]
pub struct Prometheus {
    /// `global:` section present.
    pub global: usize,
    /// `rule_files:`/`recording_rules:`/`rules:` entries.
    pub rule_files: usize,
    /// `scrape_configs:` section.
    pub scrape_configs: usize,
    /// `- job_name:` scrape jobs.
    pub jobs: usize,
    /// `static_configs:`/`targets:` entries.
    pub static_configs: usize,
    /// `*_sd_configs:` service discovery configs (dns/file_sd/kubernetes/consul/ec2/…).
    pub sd_configs: usize,
    /// `relabel_configs:`/`metric_relabel_configs:`/`alert_relabel_configs:` blocks.
    pub relabel_blocks: usize,
    /// `alerting:`/`alertmanagers:`/`remote_write:`/`remote_read:`/`storage:`/`tracing:`/`exemplars:` sections.
    pub alerting: usize,
    /// `scrape_interval`/`evaluation_interval`/`scrape_timeout`/`scrape_protocols` keys.
    pub interval_keys: usize,
    /// `scheme`/`metrics_path`/`params`/`basic_auth`/`bearer_token*`/`tls_config`/`proxy_url` keys.
    pub auth_keys: usize,
    /// `honor_labels`/`honor_timestamps`/`follow_redirects`/`enable_compression`/`sample_limit`/`target_limit`/`label_limit`/`keep_dropped_targets` flags.
    pub flags: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SD_KEYS: &[&str] = &[
    "dns_sd_configs",
    "file_sd_configs",
    "kubernetes_sd_configs",
    "consul_sd_configs",
    "ec2_sd_configs",
    "azure_sd_configs",
    "gce_sd_configs",
    "openstack_sd_configs",
    "digitalocean_sd_configs",
    "docker_sd_configs",
    "dockerswarm_sd_configs",
    "eureka_sd_configs",
    "hetzner_sd_configs",
    "http_sd_configs",
    "ionos_sd_configs",
    "kuma_sd_configs",
    "lightsail_sd_configs",
    "linode_sd_configs",
    "marathon_sd_configs",
    "nerve_sd_configs",
    "nomad_sd_configs",
    "ovhcloud_sd_configs",
    "puppetdb_sd_configs",
    "scaleway_sd_configs",
    "serverset_sd_configs",
    "triton_sd_configs",
    "uyuni_sd_configs",
    "vultr_sd_configs",
    "xds_sd_configs",
];

const INTERVAL_KEYS: &[&str] = &[
    "scrape_interval",
    "evaluation_interval",
    "scrape_timeout",
    "scrape_protocols",
];

const AUTH_KEYS: &[&str] = &[
    "scheme",
    "metrics_path",
    "params",
    "basic_auth",
    "authorization",
    "oauth2",
    "bearer_token",
    "bearer_token_file",
    "tls_config",
    "proxy_url",
    "proxy_connect_header",
];

const FLAG_KEYS: &[&str] = &[
    "honor_labels",
    "honor_timestamps",
    "follow_redirects",
    "enable_compression",
    "sample_limit",
    "target_limit",
    "label_limit",
    "label_name_length_limit",
    "label_value_length_limit",
    "keep_dropped_targets",
    "body_size_limit",
];

const ALERT_KEYS: &[&str] = &[
    "alerting",
    "alertmanagers",
    "remote_write",
    "remote_read",
    "storage",
    "tracing",
    "exemplars",
    "query_log_file",
    "runtime",
];

/// Returns `true` when the bytes look like a prometheus.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("scrape_configs:") || t.contains("job_name:"))
        && (t.contains("static_configs:") || t.contains("targets:") || t.contains("global:"))
}

impl Prometheus {
    /// Parses a prometheus.yml, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            global: 0,
            rule_files: 0,
            scrape_configs: 0,
            jobs: 0,
            static_configs: 0,
            sd_configs: 0,
            relabel_blocks: 0,
            alerting: 0,
            interval_keys: 0,
            auth_keys: 0,
            flags: 0,
            comments: 0,
        };
        let mut in_scrape = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let key = tr
                .trim_start_matches('-')
                .trim()
                .split(':')
                .next()
                .unwrap_or("");
            if l.len() - l.trim_start().len() == 0 {
                in_scrape = key == "scrape_configs";
                match key {
                    "global" => c.global += 1,
                    "scrape_configs" => c.scrape_configs += 1,
                    "rule_files" | "recording_rules" | "rules" => c.rule_files += 1,
                    _ => {
                        if ALERT_KEYS.contains(&key) {
                            c.alerting += 1;
                        }
                    }
                }
                continue;
            }
            if key == "job_name" && tr.starts_with('-') {
                c.jobs += 1;
                continue;
            }
            if key == "static_configs" || key == "targets" {
                c.static_configs += 1;
            }
            if SD_KEYS.contains(&key) {
                c.sd_configs += 1;
            }
            if key.ends_with("_relabel_configs") || key == "relabel_configs" {
                c.relabel_blocks += 1;
            }
            if INTERVAL_KEYS.contains(&key) {
                c.interval_keys += 1;
            }
            if AUTH_KEYS.contains(&key) {
                c.auth_keys += 1;
            }
            if FLAG_KEYS.contains(&key) {
                c.flags += 1;
            }
            let _ = in_scrape;
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# prom\nglobal:\n  scrape_interval: 15s\n  evaluation_interval: 15s\n  scrape_timeout: 10s\nrule_files:\n  - rules/*.yml\nscrape_configs:\n  - job_name: prometheus\n    static_configs:\n      - targets: [\"localhost:9090\"]\n    relabel_configs:\n      - source_labels: [__address__]\n  - job_name: node\n    dns_sd_configs:\n      - names: [\"_srv._tcp.example.com\"]\n    honor_labels: true\nalerting:\n  alertmanagers:\n    - static_configs:\n        - targets: [\"am:9093\"]\n";

    #[test]
    fn parses_prometheus() {
        let c = Prometheus::parse(CONF).unwrap();
        assert_eq!(c.global, 1);
        assert_eq!(c.scrape_configs, 1);
        assert_eq!(c.jobs, 2);
        assert_eq!(c.static_configs, 4);
        assert_eq!(c.sd_configs, 1);
        assert_eq!(c.relabel_blocks, 1);
        assert_eq!(c.interval_keys, 3);
        assert_eq!(c.flags, 1);
        assert_eq!(c.rule_files, 1);
        assert_eq!(c.alerting, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_prometheus() {
        assert!(!detect(b"services:\n  web:"));
        assert!(Prometheus::parse(b"x").is_none());
    }
}
