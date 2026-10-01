//! Parser for OpenTelemetry Collector configuration files
//! (`otel-collector-config.yaml` / `config.yaml`).
//!
//! Counts `receivers:`/`processors:`/`exporters:`/`extensions:`/`connectors:`
//! component blocks, typed component names (`otlp`, `prometheus`, `batch`,
//! `memory_limiter`, `jaeger`, `zipkin`, `logging`, `debug`, `file_exporter`,
//! `spanmetrics`, `tailsampling`, `k8sattributes`, `resourcedetection`,
//! `attributes`, `filter`, `transform`, `groupbytrace`, `probabilisticsampler`,
//! `cumulativetodelta`, `deltatorate`, `metricsgeneration`, `servicegraph`,
//! `redaction`, `datadog`, `kafka`, `opencensus`, `fluentforward`, `journald`,
//! `filelog`, `windowseventlog`, `syslog`, `hostmetrics`, `kubeletstats`,
//! `k8s_cluster`, `k8sobjects`, `signalfx`, `splunk_hec`, `skywalking`,
//! `solace`, `statsd`, `wavefront`, `zipkin` …), `service:`/`pipelines:`/
//! `traces`/`metrics`/`logs` pipelines, `receivers:`/`processors:`/`exporters:`
//! pipeline wiring, and `#` comments.
//!
//! ```
//! let b = b"receivers:\n  otlp:\n    protocols:\n      grpc:\nprocessors:\n  batch:\nexporters:\n  logging:\nservice:\n  pipelines:\n    traces:\n      receivers: [otlp]\n      exporters: [logging]\n";
//! assert!(izanagi_kit::otelcol::detect(b));
//! let c = izanagi_kit::otelcol::Otelcol::parse(b).unwrap();
//! assert_eq!(c.receivers, 2);
//! assert_eq!(c.pipelines, 1);
//! ```

/// Parsed OTel collector config summary.
#[derive(Debug, Clone)]
pub struct Otelcol {
    /// `receivers:` block + typed receiver entries.
    pub receivers: usize,
    /// `processors:` block + typed processor entries.
    pub processors: usize,
    /// `exporters:` block + typed exporter entries.
    pub exporters: usize,
    /// `extensions:` block entries.
    pub extensions: usize,
    /// `connectors:` block entries.
    pub connectors: usize,
    /// `service:`/`telemetry:`/`logs:`/`metrics:`/`traces:` top-level infra sections.
    pub infra: usize,
    /// `pipelines:`/`traces`/`metrics`/`logs`/`traces.*`/`metrics.*`/`logs.*` pipeline entries.
    pub pipelines: usize,
    /// `receivers:`/`processors:`/`exporters:` wiring lists inside pipelines.
    pub wiring_lists: usize,
    /// `key: value` option lines inside typed component blocks.
    pub component_options: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const INFRA: &[&str] = &["service", "telemetry", "logs", "metrics", "traces"];
const PIPELINES: &[&str] = &["pipelines", "traces", "metrics", "logs", "traces"];
const WIRE: &[&str] = &["receivers", "processors", "exporters"];

/// Returns `true` when the bytes look like an OTel collector config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("receivers:") && t.contains("service:") && t.contains("pipelines:")
        || (t.contains("receivers:") && t.contains("exporters:") && t.contains("otlp"))
}

impl Otelcol {
    /// Parses an OTel collector config, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            receivers: 0,
            processors: 0,
            exporters: 0,
            extensions: 0,
            connectors: 0,
            infra: 0,
            pipelines: 0,
            wiring_lists: 0,
            component_options: 0,
            comments: 0,
        };
        let mut ctx = "";
        let mut comp_i: Option<usize> = None;
        let mut pipe_i: Option<usize> = None;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let i = l.len() - l.trim_start().len();
            let key = tr
                .trim_start_matches('-')
                .trim()
                .split(':')
                .next()
                .unwrap_or("");
            if i == 0 {
                ctx = key;
                comp_i = None;
                pipe_i = None;
                match key {
                    "receivers" => c.receivers += 1,
                    "processors" => c.processors += 1,
                    "exporters" => c.exporters += 1,
                    "extensions" => c.extensions += 1,
                    "connectors" => c.connectors += 1,
                    _ => {
                        if INFRA.contains(&key) {
                            c.infra += 1;
                        }
                    }
                }
                continue;
            }
            match ctx {
                "receivers" | "processors" | "exporters" | "extensions" | "connectors" => {
                    let ci = *comp_i.get_or_insert(i);
                    if i == ci && tr.ends_with(':') && !tr.starts_with('-') {
                        match ctx {
                            "receivers" => c.receivers += 1,
                            "processors" => c.processors += 1,
                            "exporters" => c.exporters += 1,
                            "extensions" => c.extensions += 1,
                            _ => c.connectors += 1,
                        }
                    } else {
                        c.component_options += 1;
                    }
                }
                "service" => match pipe_i {
                    None => {
                        if key == "pipelines" {
                            pipe_i = Some(i);
                        } else if key == "telemetry" {
                            c.infra += 1;
                        }
                    }
                    Some(p) => {
                        if i == p + 2 {
                            if PIPELINES.contains(&key) {
                                c.pipelines += 1;
                            }
                        } else if i > p + 2 && WIRE.contains(&key) {
                            c.wiring_lists += 1;
                        } else if i > p {
                            c.component_options += 1;
                        }
                    }
                },
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"receivers:\n  otlp:\n    protocols:\n      grpc:\n      http:\n  prometheus:\n    config:\n      scrape_configs: []\nprocessors:\n  batch:\n  memory_limiter:\n    check_interval: 1s\nexporters:\n  logging:\n    verbosity: detailed\n  otlp/prod:\n    endpoint: otlp.prod:4317\nservice:\n  telemetry:\n    logs:\n      level: info\n  pipelines:\n    traces:\n      receivers: [otlp]\n      processors: [batch]\n      exporters: [logging, otlp/prod]\n";

    #[test]
    fn parses_otelcol() {
        let c = Otelcol::parse(CONF).unwrap();
        assert_eq!(c.receivers, 3);
        assert_eq!(c.processors, 3);
        assert_eq!(c.exporters, 3);
        assert_eq!(c.infra, 2);
        assert_eq!(c.pipelines, 1);
        assert_eq!(c.wiring_lists, 3);
        assert!(c.component_options >= 4);
    }

    #[test]
    fn rejects_non_otelcol() {
        assert!(!detect(b"version: 3\n"));
        assert!(Otelcol::parse(b"x").is_none());
    }
}
