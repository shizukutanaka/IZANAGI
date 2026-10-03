//! Grafana Mimir `mimir.yaml` の検出と構造カウント。
//!
//! ```
//! let c = izanagi_kit::mimirconf::parse(b"server:\n  http_listen_port: 9009\ndistributor:\n  pool:\n    health_check_distributors: true\nblocks_storage:\n  backend: filesystem\n  filesystem:\n    dir: /data\n").unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.nested, 3);
//! ```

/// 既知トップレベルキー。
const SECTIONS: &[&str] = &[
    "server",
    "activity_tracker",
    "alertmanager",
    "alertmanager_storage",
    "api",
    "blocks_storage",
    "compactor",
    "distributor",
    "frontend",
    "ingester",
    "ingester_client",
    "limits",
    "memberlist",
    "multitenancy_enabled",
    "querier",
    "query_frontend",
    "query_scheduler",
    "ruler",
    "ruler_storage",
    "store_gateway",
    "usage_stats",
    "tenant_federation",
    "query_engine",
    "worker",
    "overrides_exporter",
    "continuous_test",
    "kafka",
    "common",
    "flusher",
    "vault",
    "license",
    "graphite",
    "meta_monitoring",
    "log_level",
    "target",
    "auth",
    "no_auth_tenant",
    "configdb",
];
/// 各セクション配下の既知ネスト名。
const NESTED: &[&str] = &[
    "backend",
    "filesystem",
    "s3",
    "gcs",
    "azure",
    "swift",
    "bucket_store",
    "tsdb",
    "ring",
    "kvstore",
    "consul",
    "etcd",
    "memberlist",
    "lifecycler",
    "pool",
    "limits",
    "store_limits",
    "instance_limits",
    "halo_config",
    "sharding_ring",
    "rule_group",
    "store",
    "job_queue",
    "worker",
    "scheduler_worker",
    "query_scheduler",
    "frontend_worker",
    "rulers",
    "alertmanagers",
    "notifier",
    "external_url",
    "data_dir",
    "retention",
    "peer",
    "join_members",
    "rejoin_interval",
    "cluster_label",
    "cluster_verification",
    "left",
    "right",
    "addrs",
    "dns",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベルセクション数。
    pub sections: usize,
    /// セクション直下の既知ネスト名数。
    pub nested: usize,
    /// `- ` リスト項目数。
    pub items: usize,
    /// `key:` / `key: v` 行の総数。
    pub entries: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が Mimir 設定かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.nested >= 1)
}

/// `b` を Mimir 設定として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        nested: 0,
        items: 0,
        entries: 0,
        misc: 0,
    };
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
            sindent = indent;
        } else if indent == sindent + 2 && NESTED.contains(&key) {
            c.nested += 1;
        }
    }
    (known >= 1 && c.entries >= 2).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mimir() {
        let cfg = b"server:\n  http_listen_port: 9009\ndistributor:\n  pool:\n    health_check_distributors: true\n  ring:\n    kvstore:\n      store: memberlist\ningester:\n  ring:\n    replication_factor: 3\nblocks_storage:\n  backend: filesystem\n  filesystem:\n    dir: /data\n  tsdb:\n    dir: /data/tsdb\nruler_storage:\n  backend: filesystem\n  filesystem:\n    dir: /data/rules\nmemberlist:\n  join_members: [a]\nusage_stats:\n  enabled: false\n";
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 7);
        assert!(c.nested >= 7);
    }

    #[test]
    fn not_mimir() {
        assert!(parse(b"key: value\n").is_none());
    }
}
