//! Parity / OpenEthereum `config.toml` の認識と計数。
//!
//! 小文字のセクション `[parity]`/`[network]`/`[rpc]`/`[websockets]`/`[ipc]`/
//! `[dapps]`/`[secretstore]`/`[ipfs]`/`[mining]`/`[footprint]`/`[snapshots]`/
//! `[misc]`/`[stratum]`/`[account]`/`[keys]`/`[ui]`/`[internal]` と
//! snake_case/kebab キー(`chain`/`base_path`/`db_path`/`bootnodes`/`min_peers`/
//! `max_peers`/`nat`/`apis`/`interface`/`authors`/`usd_per_tx`/`gas_floor_target`/
//! `tx_queue_size`/`fat_db`/`pruning`/`warp`/`no_discovery`…)の TOML 風代入。
//!
//! ```
//! let b = b"[parity]\nchain = \"foundation\"\nbase_path = \"$HOME/.local/share/io.parity.ethereum\"\n[network]\nport = 30303\nbootnodes = []\nmin_peers = 25\nmax_peers = 50\n[rpc]\ninterface = \"local\"\napis = [\"web3\", \"eth\", \"net\", \"parity\"]\n[mining]\nauthor = \"0x0000000000000000000000000000000000000000\"\n";
//! assert!(izanagi_kit::parityconf::detect(b));
//! let c = izanagi_kit::parityconf::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.known_sections, 4);
//! assert_eq!(c.assigns, 9);
//! assert_eq!(c.list_assigns, 2);
//! ```

/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[section]` ヘッダ数。
    pub sections: usize,
    /// 既知 Parity セクション名の数。
    pub known_sections: usize,
    /// `key = value` 代入数。
    pub assigns: usize,
    /// 値が `true`/`false` の代入数。
    pub bool_assigns: usize,
    /// 値が `"…"` 文字列の代入数。
    pub string_assigns: usize,
    /// 値が数値リテラルの代入数。
    pub number_assigns: usize,
    /// 値が `[…]` 配列の代入数。
    pub list_assigns: usize,
    /// `#` コメント行数。
    pub comments: usize,
}

const KNOWN_SECTIONS: &[&str] = &[
    "parity",
    "network",
    "rpc",
    "websockets",
    "ipc",
    "dapps",
    "secretstore",
    "ipfs",
    "mining",
    "footprint",
    "snapshots",
    "misc",
    "stratum",
    "account",
    "keys",
    "ui",
    "internal",
    "light",
    "cli",
    "versions",
];

const KNOWN_KEYS: &[&str] = &[
    "base_path",
    "db_path",
    "keys_path",
    "chain",
    "network_id",
    "identity",
    "auto_update",
    "release_track",
    "no_download",
    "no_consensus",
    "no_persistent_txqueue",
    "bootnodes",
    "min_peers",
    "max_peers",
    "max_pending_peers",
    "snapshot_peers",
    "allow_ips",
    "max_packet_size",
    "reserved_peers",
    "reserved_only",
    "nat",
    "no_discovery",
    "discovery",
    "enable_deadline",
    "port",
    "interface",
    "apis",
    "hosts",
    "origins",
    "server_threads",
    "processing_threads",
    "cors",
    "path",
    "disable",
    "author",
    "engine_signer",
    "reseal_on_txs",
    "reseal_min_period",
    "reseal_max_period",
    "reseal_on_external_tx",
    "force_sealing",
    "work_queue_size",
    "tx_gas_limit",
    "tx_queue_size",
    "tx_queue_gas",
    "tx_queue_strategy",
    "tx_queue_banning",
    "tx_queue_no_unfamiliar_locals",
    "tx_queue_per_sender",
    "tx_queue_mem_limit",
    "tx_time_limit",
    "extra_data",
    "gas_floor_target",
    "gas_cap",
    "usd_per_tx",
    "usd_per_eth",
    "price_update_period",
    "gas_price_percentile",
    "poll_gas_tokens",
    "poll_lifetime",
    "relay_set",
    "local_accounts",
    "work_notify",
    "notify_work",
    "refund_threshold",
    "remove_solved",
    "fat_db",
    "pruning",
    "pruning_history",
    "pruning_memory",
    "cache_size_db",
    "cache_size_blocks",
    "cache_size_queue",
    "cache_size_state",
    "db_compaction",
    "wal",
    "fast_and_loose",
    "mode",
    "mode_timeout",
    "mode_alarm",
    "auto_update_all",
    "warp",
    "light",
    "no_hardcoded_sync",
    "scale_verifiers",
    "num_verifiers",
    "enable_snapshotting",
    "periodic_snapshot",
    "threads",
    "logger",
    "log_file",
    "color",
    "file",
    "tick",
];

fn key_of(s: &str) -> Option<&str> {
    let i = s.find('=')?;
    let k = s[..i].trim();
    if k.is_empty()
        || !k
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(k)
}

fn table_name(s: &str) -> Option<&str> {
    if !(s.starts_with('[') && s.ends_with(']')) || s.starts_with("[[") {
        return None;
    }
    let n = s[1..s.len() - 1].trim();
    if n.is_empty()
        || !n
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c == b'_' || c == b'-')
    {
        return None;
    }
    Some(n)
}

/// `parity config.toml` らしさを返す。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut known_secs = 0usize;
    let mut known_keys = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if let Some(n) = table_name(s) {
            if KNOWN_SECTIONS.contains(&n) {
                known_secs += 1;
            }
            continue;
        }
        if let Some(k) = key_of(s) {
            if KNOWN_KEYS.contains(&k) {
                known_keys += 1;
            }
        }
    }
    known_secs >= 2 || (known_secs >= 1 && known_keys >= 2)
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
            if KNOWN_SECTIONS.contains(&n) {
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
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' || ch == b'.')
        {
            continue;
        }
        c.assigns += 1;
        if v == "true" || v == "false" {
            c.bool_assigns += 1;
        } else if v.starts_with('"') || v.starts_with('\'') {
            c.string_assigns += 1;
        } else if v.starts_with('[') {
            c.list_assigns += 1;
        } else if !v.is_empty()
            && v.bytes()
                .all(|ch| ch.is_ascii_digit() || ch == b'-' || ch == b'+' || ch == b'.')
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
    fn detects_parity_toml() {
        let b = b"[parity]\nchain = \"kovan\"\n[footprint]\ncache_size_db = 128\nfat_db = true\n[mining]\ntx_queue_size = 8192\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.assigns, 4);
        assert_eq!(c.bool_assigns, 1);
        assert_eq!(c.number_assigns, 2);
        assert_eq!(c.string_assigns, 1);
    }

    #[test]
    fn rejects_geth_and_cargo() {
        assert!(parse(b"[Eth]\nNetworkId = 1\nSyncMode = \"snap\"\n").is_none());
        assert!(parse(b"[package]\nname = \"x\"\nversion = \"0.1.0\"\n").is_none());
    }
}
