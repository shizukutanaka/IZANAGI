//! Sui `fullnode.yaml` / validator 設定の認識と計数。
//!
//! kebab-case のトップキー(`db-path`/`network-address`/`metrics-address`/
//! `admin-interface-port`/`json-rpc-address`/`websocket-address`/`enable-event-processing`/
//! `genesis:`/`p2p-config`/`authority-store-pruning-config`/`checkpoint-executor-config`/
//! `expensive-safety-check-config`/`transaction-deny-config`/`state-debug-dump-config`/
//! `policy-config`/`protocol-config`/`zklogin-providers`/`supported-protocol-versions`…)と
//! ネストしたマップ値(インデントされたサブキー)を YAML 風に走査する。
//!
//! ```
//! let b = b"db-path: /opt/sui/db\nnetwork-address: /dns/localhost/tcp/8080/http\nmetrics-address: 0.0.0.0:9184\njson-rpc-address: 0.0.0.0:9000\nenable-event-processing: true\ngenesis:\n  genesis-file-location: /opt/sui/genesis.blob\np2p-config:\n  listen-address: 0.0.0.0:8084\n  seed-peers: []\n";
//! assert!(izanagi_kit::suiconf::detect(b));
//! let c = izanagi_kit::suiconf::parse(b).unwrap();
//! assert_eq!(c.entries, 10);
//! assert_eq!(c.top_entries, 7);
//! assert_eq!(c.sections, 2); // genesis + p2p-config
//! assert_eq!(c.known_top, 7);
//! assert_eq!(c.bool_entries, 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key: value`/`key:` エントリの総数(ネスト含む)。
    pub entries: usize,
    /// インデント 0 のトップレベルキー数。
    pub top_entries: usize,
    /// 値を持たずネストマップを開くトップレベルキー数。
    pub sections: usize,
    /// 既知の Sui 設定キーを使うトップエントリ数。
    pub known_top: usize,
    /// 値が `true`/`false` のエントリ数。
    pub bool_entries: usize,
    /// 値が `[…]`/`{…}` フロー値のエントリ数。
    pub flow_entries: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

const KNOWN_TOP: &[&str] = &[
    "db-path",
    "network-address",
    "metrics-address",
    "admin-interface-port",
    "json-rpc-address",
    "websocket-address",
    "websocket-port",
    "enable-event-processing",
    "enable-index-processing",
    "supported-protocol-versions",
    "genesis",
    "grpc-load-shed",
    "grpc-concurrency-limit",
    "p2p-config",
    "authority-store-pruning-config",
    "end-of-epoch-broadcast-channel-capacity",
    "checkpoint-executor-config",
    "expensive-safety-check-config",
    "transaction-deny-config",
    "state-debug-dump-config",
    "policy-config",
    "protocol-config",
    "zklogin-providers",
    "db-checkpoint-config",
    "object-cache-config",
    "transaction-kv-store-config",
    "enable-experimental-rest-api",
    "rpc-config",
    "indexer-v2-config",
    "consensus-config",
    "authority-key-pair",
    "protocol-key-pair",
    "account-key-pair",
    "worker-key-pair",
    "network-key-pair",
];

fn indent_of(s: &str) -> usize {
    s.bytes().take_while(|c| *c == b' ').count()
}

/// `key:` / `key: value` のキーを返す。`:` までの部分は kebab/snake 文字のみ。
fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find(':')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'/')
    {
        return None;
    }
    // `http://x` のようなスキーム値混入を避けるため直後は空白か行末。
    let rest = &s[i + 1..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some((k, rest.trim()))
}

/// Sui fullnode/validator yaml らしさを返す。既知トップキー ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut known = 0usize;
    for line in t.lines() {
        if indent_of(line) == 0 {
            if let Some((k, _)) = key_of(line.trim_end()) {
                if KNOWN_TOP.contains(&k) {
                    known += 1;
                }
            }
        }
    }
    known >= 2
}

/// ファイル全体を走査して [`Counts`] を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !detect(b) {
        return None;
    }
    let Ok(t) = std::str::from_utf8(b) else {
        return None;
    };
    let mut c = Counts {
        entries: 0,
        top_entries: 0,
        sections: 0,
        known_top: 0,
        bool_entries: 0,
        flow_entries: 0,
        comments: 0,
    };
    for line in t.lines() {
        if line.trim_start().starts_with('#') {
            c.comments += 1;
            continue;
        }
        let s = line.trim_end();
        if s.trim().is_empty() {
            continue;
        }
        let Some((k, v)) = key_of(s) else {
            continue;
        };
        c.entries += 1;
        if indent_of(line) == 0 {
            c.top_entries += 1;
            if KNOWN_TOP.contains(&k) {
                c.known_top += 1;
            }
            if v.is_empty() {
                c.sections += 1;
                continue;
            }
        }
        if v == "true" || v == "false" {
            c.bool_entries += 1;
        }
        if v.starts_with('[') || v.starts_with('{') {
            c.flow_entries += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_fullnode() {
        let b = b"db-path: /db\njson-rpc-address: 0.0.0.0:9000\n# cmt\nmetrics-address: 0.0.0.0:9184\np2p-config:\n  seed-peers: []\n  listen-address: /x\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 6);
        assert_eq!(c.top_entries, 4);
        assert_eq!(c.sections, 1);
        assert_eq!(c.known_top, 4);
        assert_eq!(c.flow_entries, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other_yaml() {
        assert!(parse(b"name: x\nversion: 1\non: push\n").is_none());
        assert!(parse(b"url: http://x\n").is_none());
    }
}
