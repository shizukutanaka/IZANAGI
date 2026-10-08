//! Airbyte コネクタ/接続設定(JSON / YAML)の検出と構造カウント。
//!
//! `connectionConfiguration`/`source`/`destination`/`catalog`/`sync_mode`/
//! `cursor_field`/`primary_key`/`auth_type` 等の Airbyte コネクタ設定キーと、
//! `apiKey`/`token`/`oauth2.0` 等のクレデンシャルフィールドを識別する。
//!
//! ```
//! let c = izanagi_kit::airbyteconf::parse(
//!     b"{\n  \"source_type\": \"postgres\",\n  \"connectionConfiguration\": {\n    \"host\": \"db\",\n    \"port\": 5432,\n    \"database\": \"app\",\n    \"username\": \"u\",\n    \"password\": \"p\"\n  },\n  \"sync_mode\": \"incremental\",\n  \"cursor_field\": \"updated_at\"\n}\n").unwrap();
//! assert!(c.options >= 8);
//! assert!(izanagi_kit::airbyteconf::detect(
//!     b"connectionConfiguration:\n  host: h\n  database: d\nsync_mode: incremental\ncursor_field: ts\n"));
//! ```

use crate::textutil::strip_bom;
/// Airbyte 既知設定キー。
const KEYS: &[&str] = &[
    "airbyte_secret",
    "api_key",
    "apiKey",
    "auth_type",
    "authorization",
    "bucket",
    "catalog",
    "client_id",
    "client_secret",
    "configured_catalog",
    "connectionConfiguration",
    "connection_id",
    "connection_type",
    "credentials",
    "cursor_field",
    "database",
    "dataset_id",
    "destination",
    "destination_type",
    "format",
    "host",
    "method",
    "name",
    "namespace_definition",
    "namespace_format",
    "oauth_token",
    "operation_type",
    "operations",
    "password",
    "path",
    "port",
    "prefix",
    "primary_key",
    "project_id",
    "provider",
    "replication_method",
    "role_arn",
    "schedule",
    "schema",
    "secret_access_key",
    "source",
    "source_type",
    "ssl",
    "ssl_mode",
    "start_date",
    "stream",
    "streams",
    "sync_mode",
    "tenant_id",
    "tunnel_method",
    "url",
    "username",
    "validation_method",
    "workspace_id",
];

/// 構造カウント。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 既知キー出現行数(同一行複数は 1)。
    pub options: usize,
    /// コメント行数(`#`/`//`)。
    pub comments: usize,
    /// その他の行数。
    pub misc: usize,
}

/// Airbyte 固有キー(単体でも強い証拠)。汎用キー(name/host/path…)との
/// 区別のため、最低1件の出現を要求する。
const EXCLUSIVE_KEYS: &[&str] = &[
    "airbyte_secret",
    "configured_catalog",
    "connectionConfiguration",
    "connection_id",
    "cursor_field",
    "destination_type",
    "namespace_definition",
    "namespace_format",
    "operation_type",
    "operations",
    "primary_key",
    "replication_method",
    "source_type",
    "sync_mode",
    "tunnel_method",
    "validation_method",
    "workspace_id",
];

fn key_hits(t: &str) -> usize {
    // JSON `"key":` と YAML `key:` の両方を走査(`- ` リスト項目も先頭キーとみなす)。
    let t = t.strip_prefix("- ").map_or(t, |s| s.trim_start());
    let mut n = 0usize;
    for k in KEYS {
        let pat = format!("\"{}\":", k);
        if t.contains(&pat) {
            n += 1;
            continue;
        }
        if t.starts_with(&format!("{k}:")) {
            n += 1;
        }
    }
    n
}

fn exclusive_hits(t: &str) -> usize {
    let t = t.strip_prefix("- ").map_or(t, |s| s.trim_start());
    EXCLUSIVE_KEYS
        .iter()
        .filter(|k| t.contains(&format!("\"{}\":", k)) || t.starts_with(&format!("{k}:")))
        .count()
}

/// Airbyte 設定らしさを判定する。
pub fn detect(input: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(input) else {
        return false;
    };
    let text = strip_bom(text);
    let mut hits = 0usize;
    let mut exclusive = 0usize;
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        hits += key_hits(t);
        exclusive += exclusive_hits(t);
    }
    hits >= 3 && exclusive >= 1
}

/// 構造をカウントする。
pub fn parse(input: &[u8]) -> Option<Counts> {
    if !detect(input) {
        return None;
    }
    let text = std::str::from_utf8(input).ok()?;
    let text = strip_bom(text);
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
        if t.starts_with('#') || t.starts_with("//") {
            c.comments += 1;
            continue;
        }
        if key_hits(t) > 0 {
            c.options += 1;
        } else {
            c.misc += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"// airbyte source config\nsource_type: postgres\nconnectionConfiguration:\n  host: db.internal\n  port: 5432\n  database: app\n  username: ro\n  password: s3\n  ssl_mode: require\nsync_mode: incremental\ncursor_field: updated_at\nprimary_key: id\nschedule:\n  units: 24\nstreams:\n- stream: users\n- stream: orders\n";

    #[test]
    fn airbyteconf() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.options, 15);
        assert_eq!(c.comments, 1);
        assert_eq!(c.misc, 1);
    }

    #[test]
    fn not_airbyte() {
        assert!(!detect(b"{\"a\": 1}\n"));
        assert!(parse(b"hello\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
