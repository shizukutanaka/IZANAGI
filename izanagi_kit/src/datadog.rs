//! Parser for Datadog Agent configuration (`datadog.yaml`, and the shared
//! shape of `iot-security-agent.yaml` / `system-probe.yaml` / `security-agent.yaml`).
//!
//! Counts top-level keys (`api_key`, `site`, `dd_url`, `hostname`, `tags`,
//! `log_level`, `proxy`, `dogstatsd_*`), named config sections
//! (`apm_config:`, `process_config:`, `network_config:`, `logs_config:`,
//! `runtime_security_config:`, `system_probe_config:`, `sbom:`,
//! `cloud_security_posture:`, `container_image:`), list entries under
//! `listeners:`/`config_providers:`/`autoconf`/`additional_endpoints`,
//! `logs_enabled`/`apm_enabled`-style feature toggles, and `#` comments.
//!
//! ```
//! let b = b"api_key: aaa\nsite: datadoghq.eu\napm_config:\n  enabled: true\n";
//! assert!(izanagi_kit::datadog::detect(b));
//! let c = izanagi_kit::datadog::Datadog::parse(b).unwrap();
//! assert_eq!(c.keys, 4);
//! assert_eq!(c.sections, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed datadog agent conf summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Datadog {
    /// Total `key: value` entries (any depth).
    pub keys: usize,
    /// Named block sections (`*_config:`/`logs:`/`sbom:`/... `key:` ending a line).
    pub sections: usize,
    /// Feature toggles (`logs_enabled`, `enabled`, `*_enabled`, `use_*`, `cloud_provider_metadata` bool-ish keys).
    pub toggles: usize,
    /// Identity/connectivity keys (`api_key`, `site`, `dd_url`, `hostname`, `tags`, `proxy.*`, `no_proxy`, `endpoint_*`, `additional_endpoints` member lines, `bind_host`, `cmd_port`, `expvar_port`, `dogstatsd_port`).
    pub connectivity: usize,
    /// `- item` list entries under `listeners:`/`config_providers:`/`autoconf`/`check_runners`/`additional_endpoints`.
    pub list_items: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTION_KEYS: &[&str] = &[
    "apm_config",
    "process_config",
    "process-agent",
    "network_config",
    "logs_config",
    "logs",
    "runtime_security_config",
    "system_probe_config",
    "sbom",
    "cloud_security_posture",
    "container_image",
    "container_image_collection",
    "container_lifecycle",
    "container_network_performance",
    "workloadmeta",
    "remote_configuration",
    "inventory",
    "metadata_providers",
    "fleet_policies",
    "otel_receiver",
    "forwarder",
    "serializer",
    "aggregator",
    "diagnose",
    "health_port",
    "tracemalloc_debug",
];

const CONNECTIVITY_KEYS: &[&str] = &[
    "api_key",
    "site",
    "dd_url",
    "hostname",
    "hostname_fqdn",
    "tags",
    "tag_value_split_separator",
    "proxy",
    "no_proxy",
    "skip_ssl_validation",
    "force_tls_12",
    "bind_host",
    "cmd_port",
    "expvar_port",
    "dogstatsd_port",
    "dogstatsd_socket",
    "dogstatsd_origin_detection",
    "dogstatsd_stats_enable",
    "dogstatsd_capture_enabled",
    "dogstatsd_tags",
    "dogstatsd_mapper_profiles",
    "dogstatsd_metrics_stats_enable",
    "statsd_metric_namespace",
    "statsd_forward_host",
    "statsd_forward_port",
    "statsd_metric_namespace_blacklist",
    "jmx_statsd_port",
    "additional_endpoints",
    "conf_path",
    "confd_path",
    "checks.d",
    "gce_tags",
    "collect_gce_tags",
    "collect_ec2_tags",
    "ec2_prefer_instance_id_as_hostname",
    "cloud_provider_metadata",
];

fn key_of(s: &str) -> &str {
    match s.find(':') {
        Some(c) => s[..c]
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-'))
            .next()
            .unwrap_or(""),
        None => "",
    }
}

fn is_key(s: &str, key: &str) -> bool {
    // `key :` (コロン前の空白)も YAML では合法。
    s.strip_prefix(key)
        .is_some_and(|r| r.trim_start().starts_with(':'))
}

/// Detects Datadog agent yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("api_key") && s.contains(':') {
            hits += 2;
        }
        if s.starts_with("site") && s.contains("datadoghq") {
            hits += 2;
        }
        if s.starts_with("dd_url")
            || s.starts_with("dogstatsd_")
            || is_key(s, "apm_config")
            || is_key(s, "process_config")
        {
            hits += 1;
        }
        if s.starts_with("check_runners") || s.starts_with("python_version") && s.contains(':') {
            hits += 1;
        }
        if s.contains("datadoghq") || s.contains("app.datadoghq") {
            hits += 1;
        }
    }
    hits >= 3
}

impl Datadog {
    /// Parse a datadog agent yaml; `None` when no `key:` entries found.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let t = strip_bom(t);
        let mut c = Self {
            keys: 0,
            sections: 0,
            toggles: 0,
            connectivity: 0,
            list_items: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("- ") || s == "-" {
                c.list_items += 1;
                continue;
            }
            let key = key_of(s);
            if key.is_empty() {
                continue;
            }
            c.keys += 1;
            if SECTION_KEYS.contains(&key) && l.trim_end().ends_with(':') {
                c.sections += 1;
            }
            if key == "enabled" || key.ends_with("_enabled") || key.starts_with("use_") {
                c.toggles += 1;
            }
            if CONNECTIVITY_KEYS.contains(&key)
                || key.starts_with("proxy")
                || key.starts_with("endpoint")
                || key.starts_with("dogstatsd")
            {
                c.connectivity += 1;
            }
        }
        (c.keys > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_space_before_colon() {
        // YAML では `key :` も合法。
        assert!(detect(
            b"api_key: x\nsite: datadoghq.com\napm_config :\n  enabled: true\n"
        ));
    }

    #[test]
    fn detects_datadog() {
        assert!(detect(
            b"api_key: x\nsite: datadoghq.com\napm_config:\n  enabled: true\n"
        ));
        assert!(!detect(b"key: value\n"));
    }

    #[test]
    fn counts_sections_and_toggles() {
        let c = Datadog::parse(
            b"api_key: x\nlogs_enabled: true\napm_config:\n  enabled: true\nprocess_config:\n  enabled: false\nlisteners:\n  - name: docker\n",
        )
        .unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.toggles, 3);
        assert_eq!(c.list_items, 1);
        assert_eq!(c.connectivity, 1);
    }

    #[test]
    fn rejects_empty() {
        assert!(Datadog::parse(b"# only\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
