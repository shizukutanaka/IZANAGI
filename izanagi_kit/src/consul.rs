//! Consul agent configuration and service definitions (HCL/JSON) detection
//! and census.
//!
//! Detects `datacenter`/`data_dir`/`node_name`/`server`/`bind_addr`/
//! `client_addr`/`retry_join`/`leave_on_terminate` style HCL assignments plus
//! `service`/`check`/`acl`/`connect`/`config_entries`/`telemetry`/`dns_config`/
//! `limits`/`ports`/`addresses`/`autopilot`/`gossip_encryption`/`tls` blocks,
//! and counts blocks (`service`/`check`/`services`/`checks`/`connect`/
//! `config_entries`/`proxy_defaults`/`service_defaults`/`service_resolver`/
//! `service_splitter`/`service_router`/`service_intentions`/`ingress_gateway`/
//! `terminating_gateway`/`mesh`/`api_gateway`/`policy`/`acl`/`agent_token`/
//! `audit`/`dns_config`/`telemetry`/`http_config`/`limits`/`ports`/`addresses`/
//! `dns`/`recursors`/`node_meta`/`performance`/`session_ttl_min`/`autopilot`/
//! `gossip_encryption`/`tls`/`ca_file`/`cert_file`/`key_file`/`auto_encrypt`/
//! `defaults`), ACL keys (`acl`/`enabled`/`default_policy`/`down_policy`/
//! `tokens`/`agent`/`master`/`replication`/`initial_management`/
//! `managed_service_provider`/`agent_recovery`/`enable_token_replication`/
//! `enable_token_persistence`), service keys (`service`/`name`/`id`/`tags`/
//! `address`/`port`/`socket_path`/`meta`/`tagged_addresses`/`namespace`/
//! `partition`/`kind`/`destination`/`local_bind_address`/`local_bind_port`/
//! `local_bind_socket_path`/`mesh_gateway`/`mode`/`transparent_proxy`/`proxy`/
//! `upstreams`/`local_service_address`/`local_service_port`/`config`/
//! `envoy_bootstrap_json`/`envoy_cluster_json`/`envoy_listener_json`/
//! `envoy_extra_static_clusters_json`/`envoy_extra_static_listeners_json`/
//! `envoy_stats_bind_addr`/`envoy_prometheus_bind_addr`/`destination_type`/
//! `destination_service_name`/`destination_service_subset`/`check`/`checks`/
//! `args`/`grpc`/`grpc_use_tls`/`http`/`header`/`method`/`body`/`interval`/
//! `timeout`/`dereg_after_interval`/`alias_node`/`alias_service`/`service_id`/
//! `status`/`output`/`notes`/`logical`/`virtual`/`external_sni`/
//! `dialed_directly`/`initial_status`/`flags`/`enable_tag_override`/`tcp`/
//! `ttl`/`os_service`/`docker`/`container_id`/`shell`/`h2ping`/`h2ping_use_tls`/
//! `tls_skip_verify`/`success_before_passing`/`failures_before_warning`/
//! `failures_before_critical`/`check_id`/`token`/`weights`/`passing`/`warning`),
//! `=` assignment lines, and `#`/`//` comment lines.
//!
//! ```
//! let b = b"datacenter = \"dc1\"\ndata_dir = \"/opt/consul\"\nnode_name = \"n1\"\nserver = true\nbind_addr = \"127\"\nservice {\n  name = \"web\"\n  port = 8080\n  check {\n    http = \"http://127:8080/health\"\n    interval = \"10s\"\n  }\n}\n";
//! assert!(izanagi_kit::consul::detect(b));
//! let c = izanagi_kit::consul::Consul::parse(b).unwrap();
//! assert_eq!(c.blocks, 2);
//! ```

/// Parsed Consul config summary.
#[derive(Debug, Clone)]
pub struct Consul {
    /// `service`/`check`/`acl`/`connect`/`telemetry`/`ports` blocks.
    pub blocks: usize,
    /// ACL keys (`default_policy`/`tokens`/`initial_management`/…).
    pub acl_keys: usize,
    /// service/check keys (`tags`/`upstreams`/`interval`/`dereg_after_interval`/…).
    pub service_keys: usize,
    /// `key = value` assignment lines.
    pub config_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const BLOCKS: &[&str] = &[
    "service",
    "check",
    "services",
    "checks",
    "connect",
    "config_entries",
    "proxy_defaults",
    "service_defaults",
    "service_resolver",
    "service_splitter",
    "service_router",
    "service_intentions",
    "ingress_gateway",
    "terminating_gateway",
    "mesh",
    "api_gateway",
    "acl",
    "agent_token",
    "audit",
    "dns_config",
    "telemetry",
    "http_config",
    "limits",
    "ports",
    "addresses",
    "dns",
    "node_meta",
    "performance",
    "autopilot",
    "gossip_encryption",
    "tls",
    "auto_encrypt",
    "defaults",
    "tokens",
    "jwt",
    "bound_audience",
    "routes",
    "tcp_route",
    "udp_route",
    "http_route",
    "listeners",
    "certificates",
    "resources",
];

const ACL_KEYS: &[&str] = &[
    "default_policy",
    "down_policy",
    "extend_policy",
    "tokens",
    "agent",
    "master",
    "replication",
    "default",
    "initial_management",
    "managed_service_provider",
    "agent_recovery",
    "enable_token_replication",
    "enable_token_persistence",
    "enable_token_persistence",
    "enable_agent_tls_for_checks",
    "policy_ttl",
    "role_ttl",
    "token_ttl",
];

const SERVICE_KEYS: &[&str] = &[
    "name",
    "id",
    "tags",
    "address",
    "port",
    "socket_path",
    "meta",
    "tagged_addresses",
    "namespace",
    "partition",
    "kind",
    "destination",
    "local_bind_address",
    "local_bind_port",
    "local_bind_socket_path",
    "mesh_gateway",
    "mode",
    "transparent_proxy",
    "proxy",
    "upstreams",
    "local_service_address",
    "local_service_port",
    "config",
    "envoy_bootstrap_json",
    "envoy_cluster_json",
    "envoy_listener_json",
    "envoy_extra_static_clusters_json",
    "envoy_extra_static_listeners_json",
    "envoy_stats_bind_addr",
    "envoy_prometheus_bind_addr",
    "destination_type",
    "destination_service_name",
    "destination_service_subset",
    "args",
    "grpc",
    "grpc_use_tls",
    "http",
    "header",
    "method",
    "body",
    "interval",
    "timeout",
    "dereg_after_interval",
    "alias_node",
    "alias_service",
    "service_id",
    "status",
    "output",
    "notes",
    "logical",
    "virtual",
    "external_sni",
    "dialed_directly",
    "initial_status",
    "flags",
    "enable_tag_override",
    "tcp",
    "ttl",
    "os_service",
    "docker",
    "container_id",
    "shell",
    "h2ping",
    "h2ping_use_tls",
    "tls_skip_verify",
    "success_before_passing",
    "failures_before_warning",
    "failures_before_critical",
    "check_id",
    "token",
    "weights",
    "passing",
    "warning",
];

/// Detects Consul HCL/JSON config files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    let agent = (t.contains("datacenter")
        || t.contains("data_dir")
        || t.contains("node_name")
        || t.contains("retry_join"))
        && (t.contains("server")
            || t.contains("bind_addr")
            || t.contains("client_addr")
            || t.contains("bootstrap_expect"));
    let svc = (t.contains("service {") || t.contains("\"service\""))
        && (t.contains("port") || t.contains("check") || t.contains("tags"));
    agent || svc
}

fn block_line(l: &str) -> bool {
    let w: Vec<&str> = l
        .split(|c: char| c.is_whitespace() || c == '{' || c == '[')
        .filter(|s| !s.is_empty())
        .collect();
    !w.is_empty() && BLOCKS.contains(&w[0]) && (l.contains('{') || l.contains('[') || w.len() > 1)
}

impl Consul {
    /// Parses a Consul config, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            blocks: 0,
            acl_keys: 0,
            service_keys: 0,
            config_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if block_line(tr) {
                c.blocks += 1;
                continue;
            }
            if let Some(eq) = tr.find('=') {
                let key = tr[..eq]
                    .trim()
                    .trim_end_matches('"')
                    .trim_start_matches('"');
                c.config_keys += 1;
                if ACL_KEYS.contains(&key) {
                    c.acl_keys += 1;
                }
                if SERVICE_KEYS.contains(&key) {
                    c.service_keys += 1;
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
    fn detects_and_counts() {
        let b = b"datacenter = \"dc1\"\ndata_dir = \"/opt/consul\"\nnode_name = \"n1\"\nserver = true\nbind_addr = \"127\"\nretry_join = [\"127\"]\nservice {\n  name = \"web\"\n  port = 8080\n  tags = [\"api\"]\n  check {\n    http = \"http://127:8080/health\"\n    interval = \"10s\"\n  }\n}\n";
        let c = Consul::parse(b).unwrap();
        assert_eq!(c.blocks, 2);
        assert!(c.config_keys >= 9);
        assert!(c.service_keys >= 4);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Consul::parse(b"key: value\n").is_none());
        assert!(!detect(b"key = value\nother = 2\n"));
    }
}
