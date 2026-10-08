//! Lightning Network Daemon `lnd.conf` の認識と計数。
//!
//! `;`/`#` コメントと `[Application Options]`/`[Bitcoin]`/`[Btcd]`/`[Neutrino]`/
//! `[Litecoin]`/`[autopilot]`/`[watchtower]`/`[wtclient]`/`[routerrpc]`/`[workers]`/
//! `[caches]`/`[protocol]`/`[sweeper]`/`[healthcheck]`/`[signrpc]`/`[walletrpc]`/
//! `[chainrpc]`/`[invoices]`/`[bolt]`/`[db]`/`[fee]`/`[middleware]`/`[remotesigner]`/
//! `[monitoring]`/`[htlcswitch]`/`[gossip]`/`[bitcoind]`/`[ltcd]` 系セクション、
//! セクション内の `key=value`(`bitcoin.active=1`/`btcd.rpchost=…` 等ドットキーを含む)。
//!
//! ```
//! let b = b"; lnd\n[Application Options]\ndebuglevel=info\nrpclisten=localhost:10009\nmaxpendingchannels=10\n[Bitcoin]\nbitcoin.active=1\nbitcoin.node=btcd\n[Btcd]\nbtcd.rpchost=localhost:8334\nbtcd.rpcuser=user\n";
//! assert!(izanagi_kit::lndconf::detect(b));
//! let c = izanagi_kit::lndconf::parse(b).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.assigns, 7);
//! assert_eq!(c.dotkey_assigns, 4); // bitcoin.* / btcd.*
//! assert_eq!(c.known_sections, 3);
//! ```

use crate::textutil::strip_bom;
/// [`parse`] が返す計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[Section]` ヘッダ数。
    pub sections: usize,
    /// 既知 lnd セクション名の数。
    pub known_sections: usize,
    /// `key=value` 代入行数。
    pub assigns: usize,
    /// `chain.sub=key` 形式のドットキー代入数。
    pub dotkey_assigns: usize,
    /// 値が `0`/`1`/`true`/`false` の代入数。
    pub bool_assigns: usize,
    /// `;`/`#` コメント行数。
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "application options",
    "bitcoin",
    "btcd",
    "neutrino",
    "litecoin",
    "ltcd",
    "signet",
    "autopilot",
    "gossip",
    "htlcswitch",
    "monitoring",
    "protocol",
    "sweeper",
    "bitcoind",
    "healthcheck",
    "signrpc",
    "walletrpc",
    "chainrpc",
    "invoices",
    "watchtower",
    "wtclient",
    "routerrpc",
    "workers",
    "caches",
    "bolt",
    "db",
    "fee",
    "middleware",
    "remotesigner",
    "tarod",
    "itest",
    "regtest",
    "simnet",
    "mainnet",
    "testnet",
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

/// `lnd.conf` らしさを返す。`Application Options`/`Bitcoin`/`Btcd`/`Neutrino`/`Litecoin`
/// 系セクション、またはドットキー代入。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let mut strong = 0usize;
    let mut dots = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.starts_with('[') && s.ends_with(']') {
            let n = s[1..s.len() - 1].trim().to_ascii_lowercase();
            if SECTIONS.contains(&n.as_str()) {
                strong += 1;
            }
            continue;
        }
        if let Some(k) = key_of(s) {
            let kl = k.to_ascii_lowercase();
            if kl.contains('.')
                && (kl.starts_with("bitcoin.")
                    || kl.starts_with("btcd.")
                    || kl.starts_with("litecoin.")
                    || kl.starts_with("ltcd.")
                    || kl.starts_with("neutrino.")
                    || kl.starts_with("watchtower.")
                    || kl.starts_with("wtclient.")
                    || kl.starts_with("autopilot.")
                    || kl.starts_with("routerrpc.")
                    || kl.starts_with("sweeper.")
                    || kl.starts_with("protocol."))
            {
                dots += 1;
            }
        }
    }
    strong >= 1 || dots >= 2
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
    let t = strip_bom(t);
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        assigns: 0,
        dotkey_assigns: 0,
        bool_assigns: 0,
        comments: 0,
    };
    for line in t.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with(';') || s.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') && s.ends_with(']') {
            c.sections += 1;
            if SECTIONS.contains(&s[1..s.len() - 1].trim().to_ascii_lowercase().as_str()) {
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
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'.' || ch == b'_' || ch == b'-')
        {
            continue;
        }
        c.assigns += 1;
        if k.contains('.') {
            c.dotkey_assigns += 1;
        }
        if v == "0" || v == "1" || v == "true" || v == "false" {
            c.bool_assigns += 1;
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"[Application Options]\nalias=node\ncolor=#3399FF\n[Bitcoin]\nbitcoin.active=true\nbitcoin.mainnet=1\n[watchtower]\nwatchtower.active=1\n";
        let c = parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.known_sections, 3);
        assert_eq!(c.assigns, 5);
        assert_eq!(c.dotkey_assigns, 3);
        assert_eq!(c.bool_assigns, 3);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(parse(b"[server]\nhost=x\nport=1\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
