//! Apache Pulsar `broker.conf` / `bookkeeper.conf` parser.
//!
//! Detects Java-properties-style broker & bookie configuration
//! (`brokerServicePort`, `webServicePort`, `zookeeperServers`,
//! `clusterName`, `bookiePort`, `ledgerDirectories`, …) and counts keys by
//! category: broker service, metadata store, and bookie storage.
//!
//! ```
//! let b = b"brokerServicePort=6650\nwebServicePort=8080\nzookeeperServers=zk1:2181\nclusterName=prod\nmanagedLedgerDefaultEnsembleSize=2\n";
//! assert!(izanagi_kit::pulsar::detect(b));
//! let c = izanagi_kit::pulsar::Pulsar::parse(b).unwrap();
//! assert_eq!(c.kv_pairs, 5);
//! assert!(c.broker_keys >= 3);
//! ```

/// Parsed Pulsar config summary.
#[derive(Debug, Clone)]
pub struct Pulsar {
    /// `key=value` assignment lines.
    pub kv_pairs: usize,
    /// Broker service keys (`brokerServicePort`/`webServicePort`/`managedLedger*`/`defaultNumberOfNamespaceBundles`/`loadBalancer*`/`brokerDeleteInactiveTopics*`/`allowAuto*`/`backlogQuota*`).
    pub broker_keys: usize,
    /// Metadata/store keys (`zookeeperServers`/`configurationStoreServers`/`bookkeeperMetadataServiceUri`/`globalZookeeperServers`/`metadataStoreUrl`/`clusterName`/`brokerClient*`).
    pub store_keys: usize,
    /// Bookie keys (`bookiePort`/`journal*`/`ledger*`/`entryLog*`/`dbStorage_*`/`gcWaitTime`/`allowLoopback`/`bookie*`).
    pub bookie_keys: usize,
    /// `#`/`!` comment lines.
    pub comments: usize,
}

/// Broker service keys (prefix match, camelCase).
const BROKER_KEYS: &[&str] = &[
    "brokerServicePort",
    "webServicePort",
    "managedLedger",
    "defaultNumberOfNamespaceBundles",
    "loadBalancer",
    "brokerDeleteInactiveTopics",
    "allowAuto",
    "backlogQuota",
    "subscriptionExpiration",
    "maxConsumers",
    "maxProducers",
    "maxSubscriptions",
    "maxTopicsPerNamespace",
    "dispatcher",
    "numIOThreads",
    "numHttpServerThreads",
    "numExecutorThreadPoolSize",
    "authenticationEnabled",
    "authorizationEnabled",
    "tlsEnabled",
    "tlsCertificateFilePath",
    "tlsKeyFilePath",
    "tlsTrustCertsFilePath",
    "functionsWorkerEnabled",
    "pfConcurrent",
    "preciseTimeBasedBackoff",
];

/// Metadata store keys (prefix match).
const STORE_KEYS: &[&str] = &[
    "zookeeperServers",
    "zookeeperSessionTimeoutMs",
    "configurationStoreServers",
    "configurationMetadataStoreUrl",
    "bookkeeperMetadataServiceUri",
    "globalZookeeperServers",
    "metadataStoreUrl",
    "clusterName",
    "brokerClient",
    "brokerServiceUrl",
    "brokerWebServiceUrl",
    "serviceUrl",
    "webServiceUrl",
];

/// Bookkeeper keys (prefix match).
const BOOKIE_KEYS: &[&str] = &[
    "bookiePort",
    "journalDirectory",
    "journalDirectories",
    "ledgerDirectories",
    "entryLog",
    "journalMaxSizeMB",
    "journalMaxBackups",
    "journalSyncData",
    "dbStorage_",
    "gcWaitTime",
    "allowLoopback",
    "bookie",
    "indexDirectories",
    "ledgerStorageClass",
    "flushInterval",
    "serverStats",
];

/// Detect a `broker.conf`/`bookkeeper.conf`-style file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("brokerServicePort")
        || t.contains("webServicePort")
        || t.contains("zookeeperServers")
        || t.contains("clusterName")
        || t.contains("bookiePort")
        || t.contains("ledgerDirectories")
        || t.contains("managedLedger")
        || t.contains("bookkeeperMetadataServiceUri")
}

impl Pulsar {
    /// Count key categories in a Pulsar config. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            kv_pairs: 0,
            broker_keys: 0,
            store_keys: 0,
            bookie_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with('!') {
                c.comments += 1;
                continue;
            }
            let Some(eq) = tr.find('=') else { continue };
            let key = tr[..eq].trim();
            if key.is_empty() {
                continue;
            }
            c.kv_pairs += 1;
            if BROKER_KEYS.iter().any(|k| key.starts_with(k)) {
                c.broker_keys += 1;
            }
            if STORE_KEYS.iter().any(|k| key.starts_with(k)) {
                c.store_keys += 1;
            }
            if BOOKIE_KEYS.iter().any(|k| key.starts_with(k)) {
                c.bookie_keys += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_broker() {
        let b = b"# broker\nzookeeperServers=zk1:2181,zk2:2181\nconfigurationStoreServers=cs:2184\nbrokerServicePort=6650\nbrokerServicePortTls=6651\nwebServicePort=8080\nclusterName=prod\nmanagedLedgerDefaultEnsembleSize=2\nmanagedLedgerDefaultWriteQuorum=2\nmanagedLedgerDefaultAckQuorum=2\ndefaultNumberOfNamespaceBundles=16\nloadBalancerEnabled=true\nauthenticationEnabled=true\ntlsEnabled=true\n";
        assert!(detect(b));
        let c = Pulsar::parse(b).unwrap();
        assert_eq!(c.kv_pairs, 13);
        assert_eq!(c.comments, 1);
        assert_eq!(c.store_keys, 3);
        assert!(c.broker_keys >= 7);
    }

    #[test]
    fn detects_bookie() {
        let b = b"bookiePort=3181\njournalDirectory=data/journal\nledgerDirectories=data/ledgers\nallowLoopback=true\ngcWaitTime=300000\njournalMaxSizeMB=2048\n";
        assert!(detect(b));
        let c = Pulsar::parse(b).unwrap();
        assert_eq!(c.kv_pairs, 6);
        assert_eq!(c.bookie_keys, 6);
    }

    #[test]
    fn rejects_props() {
        assert!(!detect(b"foo=bar\nbaz=qux\n"));
        assert!(Pulsar::parse(b"a=b\n").is_none());
    }
}
