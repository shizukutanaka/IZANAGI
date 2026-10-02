//! Thanos bucket/objstore YAML(`type:` + `config:`)の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::thanosconf::parse(b"type: S3\nconfig:\n  bucket: metrics\n  endpoint: s3.amazonaws.com\n  access_key: AKIA\n  secret_key: xxx\n").unwrap();
//! assert_eq!(c.objstore_type, 1);
//! assert_eq!(c.config, 4);
//! ```

/// `type:` の既知オブジェクトストア種別。
const TYPES: &[&str] = &[
    "S3",
    "GCS",
    "AZURE",
    "SWIFT",
    "COS",
    "ALIYUNOSS",
    "FILESYSTEM",
    "BOS",
    "OBS",
    "OSS",
    "OCI",
    "LINODE",
    "LOCALSTACK",
];
/// config 配下の既知キー(先頭語一致)。
const KEYS: &[&str] = &[
    "bucket",
    "endpoint",
    "region",
    "access_key",
    "secret_key",
    "insecure",
    "signature_version2",
    "secret_key_file",
    "access_key_file",
    "http_config",
    "trace",
    "part_size",
    "bucket_lookup_type",
    "sse_config",
    "sts_endpoint",
    "aws_sdk_auth",
    "service_account",
    "container",
    "storage_account",
    "storage_account_key",
    "connection_string",
    "container_name",
    "endpoint_suffix",
    "max_retries",
    "user_domain_id",
    "identity",
    "password",
    "tenant_id",
    "tenant_name",
    "domain_id",
    "domain_name",
    "region_name",
    "container_url",
    "timeout",
    "chunk_size_bytes",
    "service_token",
    "directory",
    "endpoints",
    "user",
    "key",
    "retries",
    "project_id",
    "api_url",
    "project",
    "zone",
    "bucket_name",
    "auth_type",
    "role_arn",
    "web_identity_token_file",
    "role_session_name",
    "insecure_skip_verify",
    "send_timeout",
    "idle_conn_timeout",
    "response_header_timeout",
    "tls_handshake_timeout",
    "expect_continue_timeout",
    "max_idle_conns",
    "max_idle_conns_per_host",
    "max_conns_per_host",
    "tls_insecure_skip_verify",
    "cert_file",
    "key_file",
    "ca_file",
    "server_name",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `type:` の既知種別数(0 または 1)。
    pub objstore_type: usize,
    /// config 配下の既知キー数。
    pub config: usize,
    /// `key:` / `key: v` 行の総数。
    pub entries: usize,
    /// `- ` リスト項目数。
    pub items: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Thanos bucket YAML かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.objstore_type == 1 && c.config >= 1)
}

/// `b` を Thanos bucket YAML として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        objstore_type: 0,
        config: 0,
        entries: 0,
        items: 0,
        misc: 0,
    };
    let mut in_config = false;
    let mut sindent = 0_usize;
    for line in text.lines() {
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if t.starts_with('-') {
            c.items += 1;
            c.entries += 1;
            continue;
        }
        let Some(colon) = t.find(':') else {
            c.misc += 1;
            continue;
        };
        c.entries += 1;
        let key = &t[..colon];
        let value = t[colon + 1..].trim();
        if indent == 0 {
            if key == "type" && TYPES.contains(&value) {
                c.objstore_type = 1;
            }
            in_config = key == "config";
            sindent = indent;
        } else if in_config && indent == sindent + 2 && KEYS.contains(&key) {
            c.config += 1;
        } else if indent == 0 && !matches!(key, "type" | "config" | "prefix") {
            c.misc += 1;
        }
    }
    (c.objstore_type == 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thanos() {
        let cfg = b"type: S3\nconfig:\n  bucket: metrics\n  endpoint: s3.amazonaws.com\n  region: us-east-1\n  access_key: AKIA\n  secret_key: xxx\n  insecure: false\n  http_config:\n    idle_conn_timeout: 90s\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.objstore_type, 1);
        assert_eq!(c.config, 7);
    }

    #[test]
    fn not_thanos() {
        assert!(!detect(b"key: value\n"));
    }
}
