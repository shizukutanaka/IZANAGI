//! Chroma 設定(`.chroma_env`/環境変数/Settings)の検出と構造カウント。
//!
//! `chroma_*`/`CHROMA_*` プレフィックス付き既知キーを `key=value` で分類する。
//!
//! ```
//! let c = izanagi_kit::chromaconf::parse(
//!     b"chroma_server_host=0.0.0.0\nchroma_server_http_port=8000\n").unwrap();
//! assert_eq!(c.options, 2);
//! assert!(izanagi_kit::chromaconf::detect(b"CHROMA_SERVER_HOST=localhost\nCHROMA_SERVER_AUTHN_CREDENTIALS=x\n"));
//! ```

/// 既知 Chroma 設定キー(小文字正規化後)。
const KEYS: &[&str] = &[
    "chroma_allow_reset",
    "chroma_api_impl",
    "chroma_batch_config",
    "chroma_cache_config",
    "chroma_client_auth_credential",
    "chroma_client_auth_credentials",
    "chroma_client_auth_header",
    "chroma_client_auth_provider",
    "chroma_client_host",
    "chroma_client_ssl",
    "chroma_cors_allow_origins",
    "chroma_db_impl",
    "chroma_default_auth_provider",
    "chroma_default_tenant_id",
    "chroma_default_topic_namespace",
    "chroma_enable_ssl",
    "chroma_grpc_service",
    "chroma_heartbeat_interval",
    "chroma_host_header",
    "chroma_log_config",
    "chroma_memberlist_provider",
    "chroma_otel_collection_endpoint",
    "chroma_otel_collection_required",
    "chroma_otel_exporter_endpoint",
    "chroma_otel_granularity",
    "chroma_otel_service_name",
    "chroma_persistent_data_path",
    "chroma_port",
    "chroma_product_telemetry_impl",
    "chroma_proxy_headers",
    "chroma_pulsar_tenant",
    "chroma_rate_limiting_config",
    "chroma_server_authn_credentials",
    "chroma_server_authn_credentials_file",
    "chroma_server_authn_provider",
    "chroma_server_authz_attribute",
    "chroma_server_authz_config_path",
    "chroma_server_authz_policy_path",
    "chroma_server_cors_allow_origins",
    "chroma_server_fqdn",
    "chroma_server_grpc_port",
    "chroma_server_host",
    "chroma_server_http_port",
    "chroma_server_index_config",
    "chroma_server_nofile",
    "chroma_server_ssl_certificate",
    "chroma_server_ssl_enabled",
    "chroma_server_ssl_private_key",
    "chroma_ssl_cert",
    "chroma_ssl_key",
    "chroma_sysdb_impl",
    "chroma_sysdb_sqlite",
    "chroma_telemetry_impl",
    "chroma_tenant",
    "chroma_token_auth_provider",
    "chroma_token_transport_header",
    "is_persistent",
    "persist_directory",
    "reset_state",
];

/// Chroma 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー行。
    pub options: usize,
    /// `#` コメント行。
    pub comments: usize,
    /// 分類不能行。
    pub misc: usize,
}

/// キーの正規化(小文字化 + `export ` 剥がし)が既知かどうか。
fn known(key: &str) -> bool {
    let k = key.trim().trim_start_matches("export ").trim();
    let lower: String = k.to_ascii_lowercase();
    KEYS.contains(&lower.as_str()) || lower.starts_with("chroma_")
}

/// b が Chroma 設定かどうか。
pub fn detect(b: &[u8]) -> bool {
    let text = core::str::from_utf8(b).unwrap_or("");
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.find('=').is_some_and(|p| known(&t[..p]))
        })
        .count()
        >= 2
}

/// Chroma 設定の構造を数える。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        options: 0,
        comments: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            c.comments += 1;
            continue;
        }
        let Some(pos) = t.find('=') else {
            c.misc += 1;
            continue;
        };
        if known(&t[..pos]) {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    (c.options >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# chroma\nCHROMA_SERVER_HOST=0.0.0.0\nCHROMA_SERVER_HTTP_PORT=8000\nCHROMA_SERVER_GRPC_PORT=50051\nCHROMA_SERVER_AUTHN_CREDENTIALS=test-token\nCHROMA_LOG_CONFIG=chroma_log_config.yml\npersist_directory=./chroma\nis_persistent=true\nchroma_allow_reset=false\nOTHER=x\n";

    #[test]
    fn chromaconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 8);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_chroma() {
        assert!(!detect(b"FOO=bar\nBAZ=1\n"));
        assert!(!detect(b"hello\n"));
    }
}
