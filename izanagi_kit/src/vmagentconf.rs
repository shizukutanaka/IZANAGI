//! VictoriaMetrics `vmagent` スクレイプ設定 YAML の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::vmagentconf::parse(b"global:\n  scrape_interval: 15s\nscrape_configs:\n  - job_name: node\nremote_write:\n  - url: http://vm:8429/api/v1/write\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.remotes, 1);
//! ```

/// 既知トップレベルキー。
const SECTIONS: &[&str] = &[
    "global",
    "scrape_configs",
    "remote_write",
    "remoteRead",
    "rule_files",
    "scrape_config_files",
    "influx_consistency",
    "authorization",
    "tls_config",
    "oauth2",
    "basic_auth",
    "relabeling",
];
/// scrape_configs 項目の既知名。
const SC_KEYS: &[&str] = &[
    "job_name",
    "job-name",
    "honor_labels",
    "honor_timestamps",
    "params",
    "scrape_interval",
    "scrape_timeout",
    "metrics_path",
    "scheme",
    "follow_redirects",
    "basic_auth",
    "bearer_token",
    "bearer_token_file",
    "tls_config",
    "proxy_url",
    "static_configs",
    "relabel_configs",
    "metric_relabel_configs",
    "sample_limit",
    "label_limit",
    "label_name_length_limit",
    "label_value_length_limit",
    "body_size_limit",
    "stream_parse",
    "series_limit",
    "no_stale_markers",
    "scrape_align_interval",
    "scrape_offset",
    "vm_scrape_params",
    "kubernetes_sd_configs",
    "file_sd_configs",
    "consul_sd_configs",
    "dns_sd_configs",
    "ec2_sd_configs",
    "gce_sd_configs",
    "azure_sd_configs",
    "openstack_sd_configs",
    "dockerswarm_sd_configs",
    "kuma_sd_configs",
    "eureka_sd_configs",
    "digitalocean_sd_configs",
    "yandexcloud_sd_configs",
    "vultr_sd_configs",
    "nomad_sd_configs",
    "ovhcloud_sd_configs",
    "lightsail_sd_configs",
    "hetzner_sd_configs",
    "linode_sd_configs",
    "http_sd_configs",
    "static_targets",
    "kubernetes_sd_role",
];
/// remote_write 項目の既知名。
const RW_KEYS: &[&str] = &[
    "url",
    "name",
    "basic_auth",
    "bearer_token",
    "bearer_token_file",
    "headers",
    "send_timeout",
    "tls_config",
    "proxy_url",
    "queue_config",
    "remote_timeout",
    "write_relabel_configs",
    "follow_relabelling",
    "metadata_config",
    "sigv4",
    "oauth2",
    "azure_ad",
    "max_datapoints",
    "stream_parsing",
    "disable_retry_on_http_429",
    "round_digits",
    "significant_figures",
    "show_url",
    "stateless",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルセクション数。
    pub sections: usize,
    /// scrape_configs の job 項目数。
    pub jobs: usize,
    /// remote_write の `- url:` 項目数。
    pub remotes: usize,
    /// `- ` リスト項目の総数。
    pub items: usize,
    /// `key:` / `key: v` 行の総数。
    pub entries: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が vmagent 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.jobs + c.remotes >= 1)
}

/// `b` を vmagent 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        jobs: 0,
        remotes: 0,
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
            if section == "remote_write" && name == "url" {
                c.remotes += 1;
            } else if section == "scrape_configs" && (name == "job_name" || name == "job-name") {
                c.jobs += 1;
            } else if (section == "scrape_configs" && !SC_KEYS.contains(&name))
                || (section == "remote_write" && !RW_KEYS.contains(&name))
            {
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
        } else if indent == sindent + 2 && section == "scrape_configs" && key == "job_name" {
            c.jobs += 1;
        }
    }
    (known >= 1 && c.entries >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmagent() {
        let cfg = b"global:\n  scrape_interval: 15s\n  external_labels:\n    env: prod\nscrape_configs:\n  - job_name: node\n    static_configs:\n      - targets: [localhost:9100]\n    scrape_interval: 10s\n  - job_name: app\n    kubernetes_sd_configs:\n      - role: pod\nremote_write:\n  - url: http://vm:8429/api/v1/write\n    send_timeout: 30s\n  - url: http://vm2:8429/api/v1/write\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.jobs, 2);
        assert_eq!(c.remotes, 2);
        assert!(c.items >= 5);
    }

    #[test]
    fn not_vmagent() {
        assert!(!detect(b"key: value\n"));
    }
}
