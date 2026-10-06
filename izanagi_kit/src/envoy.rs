//! Envoy static/dynamic configuration (`static_resources:`/
//! `dynamic_resources:`/`admin:`/`node:`) detection and census.
//!
//! Counts resource keys (`static_resources`/`dynamic_resources`/`admin`/
//! `node`/`cluster`/`id`/`metadata`/`locality`/`region`/`zone`/`sub_zone`/
//! `listeners`/`clusters`/`secrets`/`endpoints`/`extension_configs`/
//! `lds_config`/`cds_config`/`ads_config`/`hds_config`/`rsds_config`/
//! `ecds_config`/`pcds_config`/`tracing`/`layered_runtime`/`runtime`/
//! `static_layer`/`health_checks`/`access_log`/`rate_limit_service`/
//! `grpc_service`/`api_config_source`/`initial_fetch_timeout`/
//! `resource_api_version`/`api_type`/`config_source`/`path`/`subscriptions`),
//! listener keys (`name`/`address`/`socket_address`/`port_value`/
//! `filter_chains`/`filters`/`typed_config`/`transport_socket`/
//! `listener_filters`/`use_original_dst`/`per_connection_buffer_limit_bytes`/
//! `drain_type`/`stat_prefix`/`traffic_direction`/`reuse_port`/
//! `connection_balance_config`/`transparent`/`freebind`/
//! `listener_filters_timeout`/`continue_on_listener_filters_timeout`/
//! `default_filter_chain`/`internal_listener`/`udp_listener_config`/
//! `quic_options`/`api_listener`/`listener_specifier`/`downstream_protocols`/
//! `upstream_protocol`/`connection_balance`/`ipv4_compat`/`tcp_backlog_size`/
//! `open_connections`/`socket_options`/`additional_addresses`/`source_prefix_ranges`/
//! `source_ports`/`suffix`/`direct_response`/`tls_context`/`route_config`/
//! `virtual_hosts`/`routes`/`domains`/`prefix`/`path`/`regex`/`cluster`/
//! `host`/`weighted_clusters`/`request_headers_to_add`/`request_headers_to_remove`/
//! `typed_per_filter_config`/`http_filters`/`http_connection_manager`/
//! `codec_type`/`http2_protocol_options`/`rds`/`common_http_protocol_options`/
//! `generate_request_id`/`server_header_transformation`/`stream_idle_timeout`/
//! `idle_timeout`/`normalize_path`/`merge_slashes`/`path_with_escaped_slashes_action`/
//! `proxy_protocol`/`xff_num_trusted_hops`/`original_ip_detection`/
//! `internal_address_config`/`upgrade_configs`/`server_name`/
//! `preserve_external_request_id`/`exact`/`path_safe_prefix`/`prefix_rewrite`/
//! `host_rewrite_literal`/`host_rewrite_header`/`auto_host_rewrite`/
//! `regex_rewrite`/`retry_policy`/`cors`/`max_stream_duration`/`rate_limits`/
//! `actions`/`generic_key`/`descriptor_value`/`header_value_match`/
//! `remote_address`/`masked_remote_address`/`descriptor_key`/`jwt_authn`/
//! `providers`/`issuer`/`audiences`/`local_jwks`/`remote_jwks`/`from_headers`/
//! `from_params`/`forward`/`forward_payload_header`/`payload_in_metadata`/
//! `fractional_percent`/`numerator`/`denominator`/`include`/`custom_config`/
//! `inline_string`/`filename`/`inline_bytes`/`inline_string`/`environment_variable`),
//! cluster keys (`type`/`EDS`/`STATIC`/`STRICT_DNS`/`LOGICAL_DNS`/`ORIGINAL_DST`/
//! `cluster_discovery_type`/`eds_cluster_config`/`eds_config`/`service_name`/
//! `connect_timeout`/`per_connection_buffer_limit_bytes`/`lb_policy`/`hosts`/
//! `load_assignment`/`lb_endpoints`/`endpoint`/`filter_metadata`/`timeout`/
//! `interval`/`unhealthy_threshold`/`healthy_threshold`/`reuse_connection`/
//! `no_traffic_interval`/`http_health_check`/`codec_client_type`/`alt_stat_name`/
//! `upstream_connection_options`/`tcp_keepalive`/`typed_extension_protocol_options`/
//! `circuit_breakers`/`thresholds`/`priority`/`max_connections`/
//! `max_pending_requests`/`max_requests`/`max_retries`/`track_remaining`/
//! `cross_priority`/`load_balancing_config`/`locality_weighted_lb_config`/
//! `consecutive_5xx`/`base_ejection_time`/`max_ejection_percent`/
//! `enforcing_consecutive_5xx`/`enforcing_success_rate`/
//! `success_rate_minimum_hosts`/`success_rate_request_volume`/
//! `success_rate_stdev_factor`/`consecutive_gateway_failure`/
//! `consecutive_local_origin_failure`/`failure_percentage_threshold`/
//! `enforcing_failure_percentage`/`max_ejection_time`/`healthy_panic_threshold`/
//! `fail_traffic_on_panic`/`lrs_report_endpoint_metrics`/`grpc_health_check`/
//! `authority`/`initial_metadata`/`value`/`channel_args`/`args`/
//! `max_requests_per_connection`/`transport_socket_matches`/`cluster_name`/
//! `proximity`/`panic_threshold`/`failover_priority`/`max_connection_pools`/
//! `dns_refresh_rate`/`dns_lookup_family`/`dns_resolution_config`/`resolvers`/
//! `honor_dns_ttl`/`idle_timeout`/`per_upstream_idle_timeout`/
//! `connection_pool_per_downstream_connection`/`merged`/`lb_subset_config`/
//! `fallback_policy`/`default_subset`/`subset_selectors`/`keys`/`ring_hash_lb_config`/
//! `minimum_ring_size`/`maximum_ring_size`/`hash_function`/`xx_hash`/
//! `murmur_hash_2`/`least_request_lb_config`/`choice_count`/`slow_start_config`/
//! `slow_start_window`/`aggression`/`min_weight_percent`/`random_lb_config`/
//! `maglev_lb_config`/`table_size`/`upstream_bind_config`/`source_address`/
//! `pipe`/`socket_options`/`transport_socket`/`common_tls_context`/
//! `validation_context`/`alpn_protocols`/`tls_params`/`tls_minimum_protocol_version`/
//! `tls_maximum_protocol_version`/`cipher_suites`/`ecdh_curves`/`verified_san`/
//! `trusted_ca`/`certificate_chain`/`private_key`/`private_key_provider`/
//! `tls_certificate_sds_secret_configs`/`sds_config`/`combined_validation_context`/
//! `default_validation_context`/`verify_certificate_hash`/`verify_certificate_spiffe`/
//! `require_client_certificate`/`require_sni`/`session_ticket_keys`/`disable_stateful_session_resumption`/
//! `tap`/`tls_handshake_timeout`/`watch_parent_sds_secret`/`upstream_http_protocol_options`/
//! `http_protocol_options`/`accept_http_10`/`default_host_for_http_10`/
//! `allow_absolute_url`/`header_key`/`max_requests_per_connection`/
//! `upstream_http_protocol_options`), filter keys (`envoy.filters.*`/
//! `envoy.filters.http.*`/`envoy.filters.network.*`/`envoy.filters.listener.*`/
//! `envoy.filters.udp_listener.*`/`envoy.extensions.*`/`typed_config`/`@type`), and `#`
//! comment lines.
//!
//! ```
//! let b = b"static_resources:\n  listeners:\n  - name: listener_0\n    address:\n      socket_address:\n        address: 127\n        port_value: 10000\n  clusters:\n  - name: svc\n    type: STRICT_DNS\n    lb_policy: ROUND_ROBIN\n    load_assignment:\n      cluster_name: svc\n      endpoints: []\n";
//! assert!(izanagi_kit::envoy::detect(b));
//! let c = izanagi_kit::envoy::Envoy::parse(b).unwrap();
//! assert!(c.listener_keys >= 3);
//! ```

/// Parsed Envoy config summary.
#[derive(Debug, Clone)]
pub struct Envoy {
    /// top-level resource keys (`static_resources`/`lds_config`/…).
    pub resource_keys: usize,
    /// listener/route keys (`socket_address`/`filter_chains`/`virtual_hosts`/…).
    pub listener_keys: usize,
    /// cluster/load-balancing keys (`lb_policy`/`eds_config`/`health_checks`/…).
    pub cluster_keys: usize,
    /// `envoy.filters`/`envoy.extensions`/`@type` filter references.
    pub filter_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const RESOURCE_KEYS: &[&str] = &[
    "static_resources:",
    "dynamic_resources:",
    "admin:",
    "node:",
    "cluster:",
    "id:",
    "metadata:",
    "locality:",
    "region:",
    "zone:",
    "sub_zone:",
    "listeners:",
    "clusters:",
    "secrets:",
    "endpoints:",
    "extension_configs:",
    "lds_config:",
    "cds_config:",
    "ads_config:",
    "hds_config:",
    "rsds_config:",
    "ecds_config:",
    "pcds_config:",
    "tracing:",
    "layered_runtime:",
    "runtime:",
    "static_layer:",
    "health_checks:",
    "access_log:",
    "rate_limit_service:",
    "grpc_service:",
    "api_config_source:",
    "initial_fetch_timeout:",
    "resource_api_version:",
    "api_type:",
    "config_source:",
    "path:",
    "subscriptions:",
];

const LISTENER_KEYS: &[&str] = &[
    "name:",
    "address:",
    "socket_address:",
    "port_value:",
    "filter_chains:",
    "filters:",
    "typed_config:",
    "transport_socket:",
    "listener_filters:",
    "use_original_dst:",
    "per_connection_buffer_limit_bytes:",
    "drain_type:",
    "stat_prefix:",
    "traffic_direction:",
    "reuse_port:",
    "connection_balance_config:",
    "transparent:",
    "freebind:",
    "listener_filters_timeout:",
    "continue_on_listener_filters_timeout:",
    "default_filter_chain:",
    "internal_listener:",
    "udp_listener_config:",
    "quic_options:",
    "api_listener:",
    "ipv4_compat:",
    "tcp_backlog_size:",
    "socket_options:",
    "additional_addresses:",
    "source_prefix_ranges:",
    "source_ports:",
    "suffix:",
    "direct_response:",
    "tls_context:",
    "route_config:",
    "virtual_hosts:",
    "routes:",
    "domains:",
    "prefix:",
    "path:",
    "regex:",
    "host:",
    "weighted_clusters:",
    "request_headers_to_add:",
    "request_headers_to_remove:",
    "typed_per_filter_config:",
    "http_filters:",
    "http_connection_manager:",
    "codec_type:",
    "http2_protocol_options:",
    "rds:",
    "common_http_protocol_options:",
    "generate_request_id:",
    "server_header_transformation:",
    "stream_idle_timeout:",
    "idle_timeout:",
    "normalize_path:",
    "merge_slashes:",
    "path_with_escaped_slashes_action:",
    "proxy_protocol:",
    "xff_num_trusted_hops:",
    "original_ip_detection:",
    "internal_address_config:",
    "upgrade_configs:",
    "server_name:",
    "preserve_external_request_id:",
    "exact:",
    "path_safe_prefix:",
    "prefix_rewrite:",
    "host_rewrite_literal:",
    "host_rewrite_header:",
    "auto_host_rewrite:",
    "regex_rewrite:",
    "retry_policy:",
    "cors:",
    "max_stream_duration:",
    "rate_limits:",
    "actions:",
    "generic_key:",
    "descriptor_value:",
    "header_value_match:",
    "remote_address:",
    "masked_remote_address:",
    "descriptor_key:",
    "jwt_authn:",
    "providers:",
    "issuer:",
    "audiences:",
    "local_jwks:",
    "remote_jwks:",
    "from_headers:",
    "from_params:",
    "forward:",
    "forward_payload_header:",
    "payload_in_metadata:",
    "fractional_percent:",
    "numerator:",
    "denominator:",
    "include:",
    "custom_config:",
    "inline_string:",
    "filename:",
    "inline_bytes:",
    "environment_variable:",
];

const CLUSTER_KEYS: &[&str] = &[
    "type:",
    "cluster_discovery_type:",
    "eds_cluster_config:",
    "eds_config:",
    "service_name:",
    "connect_timeout:",
    "lb_policy:",
    "hosts:",
    "load_assignment:",
    "lb_endpoints:",
    "endpoint:",
    "filter_metadata:",
    "timeout:",
    "interval:",
    "unhealthy_threshold:",
    "healthy_threshold:",
    "reuse_connection:",
    "no_traffic_interval:",
    "http_health_check:",
    "codec_client_type:",
    "alt_stat_name:",
    "upstream_connection_options:",
    "tcp_keepalive:",
    "typed_extension_protocol_options:",
    "circuit_breakers:",
    "thresholds:",
    "priority:",
    "max_connections:",
    "max_pending_requests:",
    "max_requests:",
    "max_retries:",
    "track_remaining:",
    "cross_priority:",
    "load_balancing_config:",
    "locality_weighted_lb_config:",
    "consecutive_5xx:",
    "base_ejection_time:",
    "max_ejection_percent:",
    "enforcing_consecutive_5xx:",
    "enforcing_success_rate:",
    "success_rate_minimum_hosts:",
    "success_rate_request_volume:",
    "success_rate_stdev_factor:",
    "consecutive_gateway_failure:",
    "consecutive_local_origin_failure:",
    "failure_percentage_threshold:",
    "enforcing_failure_percentage:",
    "max_ejection_time:",
    "healthy_panic_threshold:",
    "fail_traffic_on_panic:",
    "lrs_report_endpoint_metrics:",
    "grpc_health_check:",
    "authority:",
    "initial_metadata:",
    "value:",
    "channel_args:",
    "args:",
    "max_requests_per_connection:",
    "transport_socket_matches:",
    "cluster_name:",
    "proximity:",
    "panic_threshold:",
    "failover_priority:",
    "max_connection_pools:",
    "dns_refresh_rate:",
    "dns_lookup_family:",
    "dns_resolution_config:",
    "resolvers:",
    "honor_dns_ttl:",
    "per_upstream_idle_timeout:",
    "connection_pool_per_downstream_connection:",
    "merged:",
    "lb_subset_config:",
    "fallback_policy:",
    "default_subset:",
    "subset_selectors:",
    "keys:",
    "ring_hash_lb_config:",
    "minimum_ring_size:",
    "maximum_ring_size:",
    "hash_function:",
    "xx_hash:",
    "murmur_hash_2:",
    "least_request_lb_config:",
    "choice_count:",
    "slow_start_config:",
    "slow_start_window:",
    "aggression:",
    "min_weight_percent:",
    "random_lb_config:",
    "maglev_lb_config:",
    "table_size:",
    "upstream_bind_config:",
    "source_address:",
    "pipe:",
    "common_tls_context:",
    "validation_context:",
    "alpn_protocols:",
    "tls_params:",
    "tls_minimum_protocol_version:",
    "tls_maximum_protocol_version:",
    "cipher_suites:",
    "verified_san:",
    "trusted_ca:",
    "certificate_chain:",
    "private_key:",
    "private_key_provider:",
    "tls_certificate_sds_secret_configs:",
    "sds_config:",
    "combined_validation_context:",
    "default_validation_context:",
    "verify_certificate_hash:",
    "verify_certificate_spiffe:",
    "require_client_certificate:",
    "require_sni:",
    "session_ticket_keys:",
    "disable_stateful_session_resumption:",
    "tap:",
    "tls_handshake_timeout:",
    "watch_parent_sds_secret:",
    "upstream_http_protocol_options:",
    "http_protocol_options:",
    "accept_http_10:",
    "default_host_for_http_10:",
    "allow_absolute_url:",
    "header_key:",
];
fn is_key(tr: &str, key: &str) -> bool {
    let k = tr.trim_start_matches(['"', '\'']);
    k.strip_prefix(key)
        .is_some_and(|r| r.starts_with(':') || r.starts_with("\":") || r.starts_with("':"))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim_start(), key))
}

/// Detects Envoy configuration files.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = String::from_utf8_lossy(b);
    [
        "static_resources",
        "dynamic_resources",
        "lds_config",
        "cds_config",
        "ads_config",
    ]
    .iter()
    .any(|k| has_key(&t, k))
}

impl Envoy {
    /// Parses an Envoy config, counting its structural elements.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = String::from_utf8_lossy(b);
        let mut c = Self {
            resource_keys: 0,
            listener_keys: 0,
            cluster_keys: 0,
            filter_keys: t.matches("envoy.filters").count()
                + t.matches("envoy.extensions").count()
                + t.matches("@type").count(),
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let tr = tr.trim_start_matches("- ").trim_start();
            if RESOURCE_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.resource_keys += 1;
            }
            if LISTENER_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.listener_keys += 1;
            }
            if CLUSTER_KEYS.iter().any(|k| tr.starts_with(k)) {
                c.cluster_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_marker_in_comment() {
        assert!(!detect(b"# static_resources:\n# listeners: []\n"));
    }

    #[test]
    fn detects_and_counts() {
        let b = b"static_resources:\n  listeners:\n  - name: listener_0\n    address:\n      socket_address:\n        address: 127\n        port_value: 10000\n    filter_chains:\n    - filters:\n      - name: envoy.filters.network.http_connection_manager\n        typed_config:\n          \"@type\": type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager\n  clusters:\n  - name: svc\n    type: STRICT_DNS\n    lb_policy: ROUND_ROBIN\n    connect_timeout: 5s\n    load_assignment:\n      cluster_name: svc\n      endpoints: []\n";
        let c = Envoy::parse(b).unwrap();
        assert!(c.resource_keys >= 2);
        assert!(c.listener_keys >= 5);
        assert!(c.cluster_keys >= 4);
        assert!(c.filter_keys >= 3);
    }

    #[test]
    fn rejects_plain_yaml() {
        assert!(Envoy::parse(b"key: value\n").is_none());
        assert!(!detect(b"apiVersion: v1\nkind: Pod\n"));
    }
}
