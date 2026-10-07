//! Monero `monerod.conf` / `bitmonero.conf` の認識と計数。
//!
//! 小文字・ハイフン区切りの `key=value` 行(`p2p-bind-ip`/`rpc-bind-port`/
//! `data-dir`/`log-level`/`db-sync-mode`/`limit-rate-up`/`add-priority-node`/
//! `zmq-pub`/`restricted-rpc`/`confirm-external-bind`…)と `#` コメントで構成。
//! 先頭セグメント(`p2p-*`/`rpc-*`/`log-*`/`bg-mining-*`/`bootstrap-*`/`zmq-*`/
//! `db-*`/`limit-*`/`add-*`/`block-*`)のグループを重複排除で数える。
//!
//! ```
//! let b = b"# monerod\ndata-dir=/home/x/.bitmonero\nlog-level=0\nrpc-bind-port=18081\nrestricted-rpc=1\np2p-bind-ip=0.0.0.0\nlimit-rate-up=4096\nlimit-rate-down=4096\nadd-peer=node.moneroworld.com:18089\nzmq-pub=tcp://127.0.0.1:18083\n";
//! assert!(izanagi_kit::monero::detect(b));
//! let c = izanagi_kit::monero::parse(b).unwrap();
//! assert_eq!(c.entries, 9);
//! assert_eq!(c.known_entries, 9);
//! assert_eq!(c.groups, 8); // data/log/rpc/restricted/p2p/limit/add/zmq
//! assert_eq!(c.bool_entries, 2); // log-level=0 + restricted-rpc=1
//! assert_eq!(c.comments, 1);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value` 行数(値なし `key` も1行として計数)。
    pub entries: usize,
    /// 既知 monerod オプション名を使うエントリ数。
    pub known_entries: usize,
    /// キー先頭セグメント(`rpc`/`p2p`/`log`/`zmq`/`db`/`limit`/`bg`/`bootstrap`…)の種類数。
    pub groups: usize,
    /// 値が `0`/`1`/`yes`/`no`/`true`/`false` のエントリ数。
    pub bool_entries: usize,
    /// 値が数値のみのエントリ数。
    pub number_entries: usize,
    /// 値に `:` を含むエンドポイント的エントリ数(`host:port`/`URL`)。
    pub endpoint_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

const KNOWN: &[&str] = &[
    "add-exclusive-node",
    "add-peer",
    "add-priority-node",
    "allow-local-ip",
    "ban-list",
    "bg-mining-enable",
    "bg-mining-idle-threshold",
    "bg-mining-ignore-battery",
    "bg-mining-min-idle-interval",
    "bg-mining-miner-target",
    "block-notify",
    "block-rate-notify",
    "block-sync-size",
    "bootstrap-daemon-address",
    "bootstrap-daemon-login",
    "check-updates",
    "checkpoints-path",
    "confirm-external-bind",
    "config-file",
    "count-blocks",
    "data-dir",
    "db-salvage",
    "db-sync-mode",
    "detach",
    "enable-dns-blocklist",
    "enforce-dns-checkpointing",
    "extra-messages-file",
    "fast-block-sync",
    "fixed-difficulty",
    "fluffy-blocks",
    "hide-my-port",
    "igd",
    "in-peers",
    "keep-fakechain",
    "limit-connections-per-ip",
    "limit-rate",
    "limit-rate-down",
    "limit-rate-up",
    "log-file",
    "log-level",
    "max-log-file-size",
    "max-log-files",
    "max-txpool-weight",
    "max-threads-per-miner",
    "mining-threads",
    "no-igd",
    "no-sync",
    "no-zmq",
    "non-interactive",
    "offline",
    "os-version",
    "out-peers",
    "p2p-bind-ipv6-address",
    "p2p-bind-ip",
    "p2p-bind-port",
    "p2p-external-port",
    "p2p-use-ipv6",
    "pad-transactions",
    "pidfile",
    "prep-blocks-threads",
    "proxy",
    "public-node",
    "regtest",
    "reorg-notify",
    "restricted-rpc",
    "rpc-access-control-origins",
    "rpc-bind-ip",
    "rpc-bind-ipv6-address",
    "rpc-bind-port",
    "rpc-login",
    "rpc-max-connections",
    "rpc-max-connections-per-ip",
    "rpc-payment-address",
    "rpc-payment-credits",
    "rpc-payment-difficulty",
    "rpc-restricted-bind-ip",
    "rpc-restricted-bind-ipv6-address",
    "rpc-restricted-bind-port",
    "rpc-ssl",
    "rpc-ssl-allow-any-cert",
    "rpc-ssl-allowed-fingerprints",
    "rpc-ssl-allow-chained",
    "rpc-ssl-ca-certificates",
    "rpc-ssl-certificate",
    "rpc-ssl-private-key",
    "rpc-use-ipv6",
    "seed-node",
    "show-time-stats",
    "start-mining",
    "stagenet",
    "sync-pruned-blocks",
    "testnet",
    "tos-flag",
    "tx-proxy",
    "version",
    "zmq-pub",
    "zmq-rpc-bind-ip",
    "zmq-rpc-bind-ipv6-address",
    "zmq-rpc-bind-port",
    "dev-fee",
];

fn key_of(s: &str) -> Option<&str> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return None;
    }
    Some(k)
}

/// `monerod.conf` らしさを返す。既知 kebab-key エントリ ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines()
        .filter(|l| {
            let s = l.trim();
            key_of(s).is_some_and(|k| KNOWN.contains(&k) || k.contains('-'))
        })
        .count()
        >= 2
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
        known_entries: 0,
        groups: 0,
        bool_entries: 0,
        number_entries: 0,
        endpoint_entries: 0,
        comments: 0,
    };
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        let (k, v) = match s.find('=') {
            Some(i) => (s[..i].trim(), s[i + 1..].trim()),
            None => (s, ""),
        };
        if k.is_empty()
            || !k.bytes().all(|ch| {
                ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'-' || ch == b'.'
            })
        {
            continue;
        }
        c.entries += 1;
        if KNOWN.contains(&k) {
            c.known_entries += 1;
        }
        let g = k.split('-').next().unwrap_or(k);
        if seen.insert(g) {
            c.groups += 1;
        }
        if v == "0" || v == "1" || v == "yes" || v == "no" || v == "true" || v == "false" {
            c.bool_entries += 1;
        } else if !v.is_empty()
            && v.bytes()
                .all(|ch| ch.is_ascii_digit() || ch == b'-' || ch == b'.')
        {
            c.number_entries += 1;
        } else if v.contains(':') {
            c.endpoint_entries += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_groups() {
        let b = b"data-dir=/x\nrpc-bind-port=18081\nrpc-login=u:p\np2p-bind-port=18080\nzmq-pub=tcp://127.0.0.1:18083\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 5);
        assert_eq!(c.known_entries, 5);
        assert_eq!(c.groups, 4); // data/rpc/p2p/zmq
        assert_eq!(c.number_entries, 2);
        assert_eq!(c.endpoint_entries, 2);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(parse(b"ServerName example.com\nListen 80\n").is_none());
        assert!(parse(b"[section]\nkey=value\n").is_none());
    }
}
