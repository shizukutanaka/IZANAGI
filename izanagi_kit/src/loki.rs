//! Loki `config.yaml` の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::loki::parse(b"server:\n  http_listen_port: 3100\nstorage_config:\n  boltdb_shipper:\n    active_index_directory: /loki/index\nschema_config:\n  configs:\n    - from: 2024-01-01\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.backends, 1);
//! ```

/// 既知トップレベルキー。
const SECTIONS: &[&str] = &[
    "server",
    "frontend",
    "query_range",
    "query_scheduler",
    "frontend_worker",
    "ingester",
    "ingester_client",
    "storage_config",
    "schema_config",
    "boltdb_shipper",
    "compactor",
    "ruler",
    "limits_config",
    "chunk_store_config",
    "table_manager",
    "memberlist",
    "distributor",
    "analytics",
    "querier",
    "index_gateway",
    "common",
    "pattern_ingester",
    "operational_config",
    "cache_config",
    "cos",
    "legacy_query_frontend",
    "tracing",
    "shutdown_marker",
    "ui",
    "volume",
    "aggregated_metrics",
    "bloom_build",
    "bloom_gateway",
    "dataobj",
    "index_gateway_client",
    "runtime_config",
];
/// storage_config 配下の既知バックエンド名。
const BACKENDS: &[&str] = &[
    "aws",
    "azure",
    "gcs",
    "boltdb_shipper",
    "filesystem",
    "cassandra",
    "bigtable",
    "swift",
    "cos",
    "alibabacloud",
    "tsdb",
    "named_stores",
    "grpc_store",
    "hedging",
    "index_queries_cache_config",
    "chunks_cache",
    "embedded_cache",
    "results_cache",
    "azure_cosmosdb",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルセクション数。
    pub sections: usize,
    /// storage_config 配下のバックエンド名数。
    pub backends: usize,
    /// schema_config/ruler/memberlist 配下のネスト名数。
    pub nested: usize,
    /// `key:` / `key: v` 行の総数(全レベル)。
    pub entries: usize,
    /// `- ` リスト項目数。
    pub items: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Loki 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.backends + c.nested >= 1)
}

/// `b` を Loki 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        backends: 0,
        nested: 0,
        entries: 0,
        items: 0,
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
        } else if indent == sindent + 2 {
            if section == "storage_config" && BACKENDS.contains(&key) {
                c.backends += 1;
            } else if matches!(
                section,
                "schema_config" | "ruler" | "memberlist" | "chunk_store_config" | "table_manager"
            ) {
                c.nested += 1;
            }
        }
    }
    (known >= 1 && c.entries >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loki() {
        let cfg = b"server:\n  http_listen_port: 3100\n  grpc_listen_port: 9095\ningester:\n  lifecycler:\n    ring:\n      kvstore:\n        store: inmemory\nstorage_config:\n  boltdb_shipper:\n    active_index_directory: /loki/index\n  filesystem:\n    directory: /loki/chunks\nschema_config:\n  configs:\n    - from: 2024-01-01\n      store: tsdb\n      schema: v13\nruler:\n  storage:\n    type: local\nmemberlist:\n  join_members: [a, b]\nlimits_config:\n  retention_period: 744h\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 7);
        assert_eq!(c.backends, 2);
        assert_eq!(c.nested, 3);
        assert_eq!(c.items, 1);
    }

    #[test]
    fn not_loki() {
        assert!(parse(b"key: value\n").is_none());
    }
}
