//! Bitcoin Core `bitcoin.conf` の認識と計数。
//!
//! `key=value` 行(値なし `key=0`/`key` 単体指定は `key=1` 扱い)、`#` コメント、
//! ネットワーク別セクション `[main]`/`[test]`/`[testnet3]`/`[signet]`/`[regtest]` を扱う。
//! 既知オプション(`datadir`/`server`/`rpcuser`/`rpcport`/`zmqpub*`/`addnode`/
//! `prune`/`maxmempool`/`onlynet`/`bind`/`loadwallet` 等)と繰返し可能オプションを区別する。
//!
//! ```
//! let b = b"# bitcoind\ndatadir=/var/lib/bitcoin\nserver=1\nprune=550\n[signet]\nrpcport=38332\naddnode=seed.signet.bitcoin.sprovoost.nl\n";
//! assert!(izanagi_kit::bitcoinconf::detect(b));
//! let c = izanagi_kit::bitcoinconf::parse(b).unwrap();
//! assert_eq!(c.entries, 5);
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.section_entries, 2);
//! assert_eq!(c.repeatable_entries, 1);
//! assert_eq!(c.bool_entries, 1); // server=1
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `key=value`(および値なし `key`)行の総数。
    pub entries: usize,
    /// `[main]`/`[test]`/`[signet]`/`[regtest]`/`[testnet3]` 等のセクション数。
    pub sections: usize,
    /// セクション内のエントリ数。
    pub section_entries: usize,
    /// 既知 bitcoind オプション名を使うエントリ数。
    pub known_entries: usize,
    /// 値が `0`/`1` のエントリ数。
    pub bool_entries: usize,
    /// 繰返し可能オプション(`addnode`/`connect`/`zmqpub*`/`includeconf`/`loadwallet`/
    /// `whitelist`/`whitebind`/`bind`/`debug`/`onlynet`/`externalip`/`debugexclude`)のエントリ数。
    pub repeatable_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

const KNOWN: &[&str] = &[
    "acceptnonstdtxn",
    "addnode",
    "asmap",
    "assumevalid",
    "bind",
    "blockfilterindex",
    "blockreconstructionextratxn",
    "blocksdir",
    "blocksonly",
    "bumpfee",
    "chain",
    "checklevel",
    "checkmempool",
    "checkpoints",
    "coinstatsindex",
    "conf",
    "connect",
    "daemon",
    "daemonwait",
    "datadir",
    "dbcache",
    "debug",
    "debugexclude",
    "deprecatedrpc",
    "disablewallet",
    "discardfee",
    "dns",
    "dnsseed",
    "dustrelayfee",
    "externalip",
    "fallbackfee",
    "fixedseeds",
    "forcednsseed",
    "i2pacceptincoming",
    "i2psam",
    "includeconf",
    "keypool",
    "listen",
    "listenonion",
    "loadwallet",
    "logips",
    "logtimestamps",
    "maxconnections",
    "maxmempool",
    "maxorphantx",
    "maxuploadtarget",
    "mempoolexpiry",
    "minrelaytxfee",
    "mocktime",
    "natpmp",
    "netpermission",
    "onlynet",
    "par",
    "paytxfee",
    "peertimeout",
    "persistmempool",
    "pid",
    "port",
    "printtoconsole",
    "proxy",
    "proxyrandomize",
    "prune",
    "rest",
    "reindex",
    "rpcallowip",
    "rpcauth",
    "rpcbind",
    "rpccookiefile",
    "rpcpassword",
    "rpcport",
    "rpcserialversion",
    "rpcthreads",
    "rpcuser",
    "rpcwhitelist",
    "rpcwhitelistdefault",
    "rpcworkqueue",
    "sandwich",
    "seednode",
    "server",
    "settings",
    "signet",
    "spendzeroconfchange",
    "statsenable",
    "statshost",
    "statsport",
    "stopafterblockimport",
    "testnet",
    "timeout",
    "torcontrol",
    "torpassword",
    "txconfirmtarget",
    "txindex",
    "uacomment",
    "upnp",
    "wallet",
    "walletbroadcast",
    "walletdir",
    "walletnotify",
    "walletrbf",
    "whitebind",
    "whitelist",
    "zmqpubhashblock",
    "zmqpubhashtx",
    "zmqpubrawblock",
    "zmqpubrawtx",
    "zmqpubsequence",
    "zmqpubhashblockhwm",
];

const REPEATABLE: &[&str] = &[
    "addnode",
    "bind",
    "connect",
    "debug",
    "debugexclude",
    "externalip",
    "includeconf",
    "loadwallet",
    "onlynet",
    "whitebind",
    "whitelist",
];

const SECTIONS: &[&str] = &[
    "main", "mainnet", "test", "testnet", "testnet3", "signet", "regtest",
];

fn key_of(s: &str) -> Option<&str> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(k)
}

/// `bitcoin.conf` らしさを返す。既知オプション行 ≥2。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut known = 0usize;
    let mut sections = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.starts_with('[') && s.ends_with(']') {
            if SECTIONS.contains(&s[1..s.len() - 1].trim()) {
                sections += 1;
            }
            continue;
        }
        let bare = s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_' || c == b'-')
            && !s.is_empty();
        let k = match key_of(s) {
            Some(k) => k,
            None if bare => s,
            None => continue,
        };
        let kl = k.to_ascii_lowercase();
        if KNOWN.contains(&kl.as_str()) || kl.starts_with("zmqpub") || kl.starts_with("-debug") {
            known += 1;
        }
    }
    known >= 2 || (sections >= 1 && known >= 1)
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
        sections: 0,
        section_entries: 0,
        known_entries: 0,
        bool_entries: 0,
        repeatable_entries: 0,
        comments: 0,
    };
    let mut in_section = false;
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') && s.ends_with(']') {
            c.sections += 1;
            in_section = true;
            continue;
        }
        let (key, val) = match s.find('=') {
            Some(i) => (&s[..i], s[i + 1..].trim()),
            None => (s, "1"),
        };
        let k = key.trim();
        if k.is_empty()
            || !k
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'.' || ch == b'_' || ch == b'-')
        {
            continue;
        }
        c.entries += 1;
        if in_section {
            c.section_entries += 1;
        }
        let kl = k.to_ascii_lowercase();
        if KNOWN.contains(&kl.as_str()) || kl.starts_with("zmqpub") {
            c.known_entries += 1;
        }
        if REPEATABLE.contains(&kl.as_str()) || kl.starts_with("zmqpub") {
            c.repeatable_entries += 1;
        }
        if val == "0" || val == "1" || val.is_empty() {
            c.bool_entries += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"datadir=/x\nserver=1\ntxindex=1\n# note\n[test]\nrpcport=18332\nconnect=a:8333\nconnect=b:8333\n";
        let c = parse(b).unwrap();
        assert_eq!(c.entries, 6);
        assert_eq!(c.sections, 1);
        assert_eq!(c.section_entries, 3);
        assert_eq!(c.known_entries, 6);
        assert_eq!(c.bool_entries, 2);
        assert_eq!(c.repeatable_entries, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn bare_key_is_enabled() {
        let c = parse(b"server\ndaemon\n").unwrap();
        assert_eq!(c.bool_entries, 2);
    }

    #[test]
    fn rejects_other_conf() {
        assert!(parse(b"foo=1\nbar=2\n").is_none());
        assert!(parse(b"listen = 80\nroot /var/www\n").is_none());
    }
}
