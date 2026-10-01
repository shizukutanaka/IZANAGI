//! Arkime (旧 Moloch) 設定ファイル (`config.ini`) パーサ。
//!
//! `[default]`/`[cache]`/`[overrides.<host>]` セクションと
//! `elasticsearch`/`interface`/`pcapDir`/`passwordSecret`/`parsersDir` 等キーを計数する。
//!
//! ```
//! use izanagi_kit::arkimeconf;
//! let conf = b"[default]\nelasticsearch = http://es:9200\ninterface = eth0\npcapDir = /data/pcap\npluginsDir = /data/moloch/plugins\n";
//! assert!(arkimeconf::detect(conf));
//! let c = arkimeconf::parse(conf).unwrap();
//! assert_eq!(c.known_sections, 1);
//! assert_eq!(c.known_keys, 4);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[...]` セクション数。
    pub sections: usize,
    /// 既知セクション数 (`default`/`cache`/`overrides.*`)。
    pub known_sections: usize,
    /// `key = value` 行数。
    pub entries: usize,
    /// 既知キー行数。
    pub known_keys: usize,
}

const KNOWN_KEYS: &[&str] = &[
    "interface",
    "pcapReadMethod",
    "pcapWriteMethod",
    "snapLen",
    "plugins",
    "parsersDir",
    "pluginsDir",
    "dropUser",
    "dropGroup",
    "passwordSecret",
    "httpRealm",
    "pcapDir",
    "rotateIndex",
    "maxFileSizeG",
    "elasticsearch",
    "viewClass",
    "viewUrl",
    "userPlugins",
    "geoLite2Country",
    "geoLite2ASN",
    "geoLite2City",
    "rirFile",
    "ouiFile",
    "spiDataMaxIndices",
    "yara",
    "hostIpMap",
    "maxStreams",
    "asyncNNTimeout",
    "dbBulkSize",
    "dbFlushTimeout",
    "dbPingTimeout",
    "maxReqQS",
    "maxESConns",
    "maxESRequests",
    "esRequestTimeout",
    "caTrustFile",
    "insecure",
    "keyFile",
    "certFile",
    "magicMode",
    "fileCloseTimeout",
    "maxConnections",
    "readMethod",
    "packetThreads",
    "pcapWriteSize",
    "pcapWriteMethod",
    "dryRun",
    "copyPcap",
    "panicDump",
    "extraOps",
    "extraTags",
];

/// 簡易判定 (`default`/`cache`/`overrides.*` セクション + 既知キー)。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    (c.known_sections >= 1 && c.known_keys >= 3) || c.known_keys >= 5
}

/// パースして計数を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        known_sections: 0,
        entries: 0,
        known_keys: 0,
    };
    let mut found = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() > 2 {
            c.sections += 1;
            let name = t[1..t.len() - 1].trim();
            if name == "default" || name == "cache" || name.starts_with("overrides") {
                c.known_sections += 1;
            }
            found = true;
            continue;
        }
        let Some(eq) = t.find('=') else {
            continue;
        };
        let key = t[..eq].trim();
        if key.is_empty() || key.bytes().any(|b| b == b' ') {
            continue;
        }
        c.entries += 1;
        if KNOWN_KEYS.contains(&key) {
            c.known_keys += 1;
        }
        found = true;
    }
    found.then_some(c)
}

#[cfg(test)]
mod tests {
    use super::{detect, parse};

    const SAMPLE: &[u8] = b"[default]\nelasticsearch = http://localhost:9200\npasswordSecret = secret123\ninterface = eth0\npcapDir = /data/moloch/raw\npluginsDir = /data/moloch/plugins\nparsersDir = /data/moloch/parsers\nyara = /data/moloch/yara/all.yar\ngeoLite2Country = /usr/share/GeoIP/GeoLite2-Country.mmdb\ngeoLite2ASN = /usr/share/GeoIP/GeoLite2-ASN.mmdb\nrirFile = /usr/share/moloch/etc/ipv4-address-space.csv\nouiFile = /usr/share/moloch/etc/oui.txt\n\n[overrides.capture]\ninterface = eth1\n";

    #[test]
    fn detects_arkimeconf() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.known_sections, 2);
        assert_eq!(c.entries, 12);
        assert_eq!(c.known_keys, 12);
    }

    #[test]
    fn rejects_generic_ini() {
        assert!(!detect(b"[main]\nhost = x\nport = 1\nfoo = y\n"));
        assert!(!detect(b"[server]\nlisten = 0.0.0.0\nroot = /var/www\n"));
    }
}
