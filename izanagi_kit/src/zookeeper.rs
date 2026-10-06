//! Apache ZooKeeper `zoo.cfg` census.
//!
//! zoo.cfg is `key=value` lines (`#` comments): `tickTime`,
//! `initLimit`, `syncLimit`, `dataDir`, `dataLogDir`, `clientPort`,
//! `server.N=host:peer:elect`, `maxClientCnxns`,
//! `autopurge.snapRetainCount`, `autopurge.purgeInterval`,
//! `preAllocSize`, `snapCount`, `leaderServes`,
//! `globalOutstandingLimit`, `electionAlg`, `minSessionTimeout`,
//! `maxSessionTimeout`, `admin.enableServer`,
//! `4lw.commands.whitelist`, `metricsProvider.*`, `ssl.*`,
//! `authProvider.*`, `quorum.auth.*`, `jute.maxbuffer`.
//!
//! ```rust
//! let k = b"tickTime=2000\ninitLimit=10\nsyncLimit=5\ndataDir=/var/lib/zk\nclientPort=2181\nserver.1=z1:2888:3888\nserver.2=z2:2888:3888\n";
//! assert!(izanagi_kit::zookeeper::detect(k));
//! ```

/// ZooKeeper `zoo.cfg` census.
#[derive(Debug, Clone)]
pub struct Zookeeper {
    /// `key=value` assignments.
    pub settings: usize,
    /// recognised keys present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const STRONG: &[&str] = &[
    "tickTime",
    "initLimit",
    "syncLimit",
    "dataLogDir",
    "maxClientCnxns",
    "autopurge.snapRetainCount",
    "autopurge.purgeInterval",
    "preAllocSize",
    "snapCount",
    "leaderServes",
    "globalOutstandingLimit",
    "electionAlg",
    "minSessionTimeout",
    "maxSessionTimeout",
    "jute.maxbuffer",
    "commitLogCount",
    "4lw.commands.whitelist",
    "standaloneEnabled",
    "reconfigEnabled",
    "quorumListenOnAllIPs",
    "peerType",
    "syncEnabled",
    "dynamicConfigFile",
    "skipACL",
    "enforce_quota",
    "readonlymode.enabled",
    "zookeeper.datadir.autocreate",
    "largeRequestMaxBytes",
    "outstandingHandshake.limit",
    "learner.asyncSending",
    "forward_learner_requests_to_commit_processor_disabled",
];

const WEAK: &[&str] = &[
    "dataDir",
    "clientPort",
    "clientPortAddress",
    "secureClientPort",
    "secureClientPortAddress",
    "admin.enableServer",
    "admin.serverAddress",
    "admin.serverPort",
    "admin.idleTimeout",
    "admin.commandURL",
    "connectToLearnerMasterLimit",
    "metricsProvider.className",
    "metricsProvider.httpPort",
    "metricsProvider.exportJvmInfo",
    "ssl.clientAuth",
    "ssl.keyStore.location",
    "ssl.trustStore.location",
    "ssl.hostnameVerification",
    "sslQuorum",
    "authProvider.1",
    "quorum.auth.enableSasl",
    "quorum.auth.learnerRequireSasl",
    "quorum.auth.serverRequireSasl",
    "sessionRequireClientSASLAuth",
    " DigestAuthenticationProvider.superDigest",
    "closeSessionTxn.enabled",
    "flushDelay",
    "maxWriteFlushPeriod",
    "requestLogThresholdMs",
    "throttledOpWaitTime",
    "snapshot.trust.empty",
    "audit.enable",
];

fn assign_key(line: &str) -> Option<&str> {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') || s.starts_with('[') {
        return None;
    }
    match s.find('=') {
        Some(i) => {
            let k = s[..i].trim().trim_matches('"');
            if k.is_empty() || k.contains(' ') {
                None
            } else {
                Some(k)
            }
        }
        None => None,
    }
}

fn is_zk_key(k: &str) -> bool {
    STRONG.contains(&k)
        || k.starts_with("server.")
        || k.starts_with("group.")
        || k.starts_with("weight.")
        || k.starts_with("metricsProvider.")
        || k.starts_with("ssl.")
        || k.starts_with("sslQuorum")
        || k.starts_with("authProvider.")
        || k.starts_with("quorum.auth.")
        || k.starts_with("kerberos.")
        || k.starts_with("client.portUnification")
        || k.starts_with("multiAddress.")
}

/// Detect a ZooKeeper `zoo.cfg`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `tickTime`/`initLimit`/`server.N`/`autopurge.*` are
    // ZooKeeper-exclusive keys.
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(k) = assign_key(line) {
            if is_zk_key(k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    strong >= 2 || (strong >= 1 && weak >= 2)
}

impl Zookeeper {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            keys: 0,
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
            if let Some(k) = assign_key(line) {
                c.settings += 1;
                if is_zk_key(k) || WEAK.contains(&k) {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"tickTime=2000\ninitLimit=10\nsyncLimit=5\ndataDir=/var/lib/zk\nclientPort=2181\nserver.1=z1:2888:3888\nserver.2=z2:2888:3888\n";
        assert!(detect(b));
        let c = Zookeeper::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"dataDir=/x\nclientPort=1\n"));
        assert!(!detect(b"# tickTime=1\n# initLimit=2\ndataDir=/x\n"));
    }
}
