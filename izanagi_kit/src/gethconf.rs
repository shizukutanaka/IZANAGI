//! go-ethereum `geth dumpconfig` / `config.toml` の認識と計数。
//!
//! Geth の TOML 設定は `[Eth]`/`[Node]`/`[Node.P2P]`/`[Eth.TxPool]`/`[Eth.Miner]`/
//! `[Eth.Ethash]`/`[Dashboard]`/`[Metrics]` 等の PascalCase テーブルと
//! CamelCase キー(`NetworkId`/`SyncMode`/`DiscoveryURLs`/`MaxPeers`/`DatabaseCache`/
//! `Ethash`/`NoPruning`/`GasFloor`/`Etherbase`…)で構成される。
//! 値は TOML の bool/string/整数/配列。
//!
//! ```
//! let b = b"[Eth]\nNetworkId = 1\nSyncMode = \"snap\"\nNoPruning = false\n[Eth.TxPool]\nLocals = []\nNoLocals = false\nAccountSlots = 16\n[Node]\nDataDir = \"/data\"\nHTTPHost = \"\"\n";
//! assert!(izanagi_kit::gethconf::detect(b));
//! let c = izanagi_kit::gethconf::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.known_sections, 3);
//! assert_eq!(c.assigns, 8);
//! assert_eq!(c.bool_assigns, 2);
//! assert_eq!(c.nested_sections, 1); // Eth.TxPool
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[Section]`/`[Sub.Section]` テーブルヘッダ数。
    pub sections: usize,
    /// 既知 Geth テーブル(Eth/Node/P2P/TxPool/Miner/Ethash/Dashboard/Metrics/Les/…)の数。
    pub known_sections: usize,
    /// `.` を含むネストテーブル数。
    pub nested_sections: usize,
    /// `Key = value` 行数。
    pub assigns: usize,
    /// 値が `true`/`false` の行数。
    pub bool_assigns: usize,
    /// 値が `"…"` 文字列の行数。
    pub string_assigns: usize,
    /// 値が数値リテラルの行数。
    pub number_assigns: usize,
    /// 値が `[…]` 配列の行数。
    pub list_assigns: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

const KNOWN_TOP: &[&str] = &[
    "Eth",
    "Node",
    "P2P",
    "Node.P2P",
    "Eth.P2P",
    "Eth.Ethash",
    "Eth.Clique",
    "Eth.TxPool",
    "Eth.Miner",
    "Eth.GasPrice",
    "Eth.Snapshot",
    "Eth.Filter",
    "Eth.Core",
    "Eth.BlobPool",
    "Les",
    "Dashboard",
    "Monitor",
    "Metrics",
    "Metrics.InfluxDB",
    "Metrics.InfluxDBV2",
    "Db",
    "Shh",
    "GraphQL",
    "JSONRPC",
    "JsonRpc",
    "HTTP",
    "WS",
    "IPC",
    "Dev",
    "Otterscan",
    "Apy",
    "Wit",
    "Simulated",
];

fn table_name(s: &str) -> Option<&str> {
    if !(s.starts_with('[') && s.ends_with(']')) || s.starts_with("[[") {
        return None;
    }
    let n = s[1..s.len() - 1].trim();
    if n.is_empty()
        || !n
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(n)
}

fn pascal(s: &str) -> bool {
    let mut it = s.split('.');
    it.all(|p| p.bytes().next().is_some_and(|c| c.is_ascii_uppercase()))
}

/// `geth config.toml` らしさを返す。PascalCase セクション + CamelCase 代入。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut pascal_secs = 0usize;
    let mut camel_assigns = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if let Some(n) = table_name(s) {
            if pascal(n) {
                pascal_secs += 1;
            }
            continue;
        }
        if let Some(i) = s.find('=') {
            let k = s[..i].trim();
            if k.bytes().next().is_some_and(|c| c.is_ascii_uppercase())
                && k.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'_')
            {
                camel_assigns += 1;
            }
        }
    }
    pascal_secs >= 1 && camel_assigns >= 2
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
        sections: 0,
        known_sections: 0,
        nested_sections: 0,
        assigns: 0,
        bool_assigns: 0,
        string_assigns: 0,
        number_assigns: 0,
        list_assigns: 0,
        comments: 0,
    };
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if let Some(n) = table_name(s) {
            c.sections += 1;
            if n.contains('.') {
                c.nested_sections += 1;
            }
            if KNOWN_TOP.contains(&n) || n.split('.').next().is_some_and(|p| KNOWN_TOP.contains(&p))
            {
                c.known_sections += 1;
            }
            continue;
        }
        let Some(i) = s.find('=') else {
            continue;
        };
        let k = s[..i].trim();
        let v = s[i + 1..].trim();
        if k.is_empty()
            || !k
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'.' || ch == b'_')
        {
            continue;
        }
        c.assigns += 1;
        if v == "true" || v == "false" {
            c.bool_assigns += 1;
        } else if v.starts_with('"') {
            c.string_assigns += 1;
        } else if v.starts_with('[') {
            c.list_assigns += 1;
        } else if !v.is_empty()
            && v.bytes().all(|ch| {
                ch.is_ascii_digit() || ch == b'-' || ch == b'+' || ch == b'.' || ch == b'e'
            })
        {
            c.number_assigns += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_geth_toml() {
        let b = b"[Eth]\nNetworkId = 5\nSyncMode = \"snap\"\n[Eth.Miner]\nEtherbase = \"0x0\"\nGasFloor = 8000000\n[Node.P2P]\nMaxPeers = 50\nNoDiscovery = false\nBootstrapNodes = [\"enode://a\"]\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.nested_sections, 2);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.assigns, 7);
        assert_eq!(c.bool_assigns, 1);
        assert_eq!(c.string_assigns, 2);
        assert_eq!(c.number_assigns, 3);
        assert_eq!(c.list_assigns, 1);
    }

    #[test]
    fn rejects_other_toml() {
        assert!(parse(b"[package]\nname = \"x\"\nversion = \"1\"\n").is_none());
        assert!(parse(b"foo = 1\nbar = 2\n").is_none());
    }
}
