//! MongoDB `mongod.conf`/`mongos.conf` (YAML) parser.
//!
//! Detects the characteristic top-level sections (`systemLog`, `storage`,
//! `net`, `replication`, `sharding`, `security`, `operationProfiling`,
//! `setParameter`, `processManagement`, `cloud`/`snmp`) and camelCase
//! sub-keys (`dbPath`, `bindIp`, `replSetName`, `wiredTiger`, `destination`,
//! `enableMajorityReadConcern` …), and counts sections, keys, bool values,
//! and `#` comments.
//!
//! ```
//! let b = br#"systemLog:
//!   destination: file
//!   path: /var/log/mongodb/mongod.log
//!   logAppend: true
//! storage:
//!   dbPath: /var/lib/mongo
//!   wiredTiger:
//!     engineConfig:
//!       cacheSizeGB: 1
//! net:
//!   port: 27017
//!   bindIp: 127.0.0.1
//! replication:
//!   replSetName: rs0"#;
//! assert!(izanagi_kit::mongod::detect(b));
//! let c = izanagi_kit::mongod::Mongod::parse(b).unwrap();
//! assert_eq!(c.sections, 4);
//! assert_eq!(c.keys, 14);
//! ```

/// Parsed mongod.conf summary.
#[derive(Debug, Clone)]
pub struct Mongod {
    /// Top-level config sections present.
    pub sections: usize,
    /// `key:`/`key: value` lines anywhere.
    pub keys: usize,
    /// CamelCase sub-keys (MongoDB naming style).
    pub camel_keys: usize,
    /// `true`/`false`/`enabled`/`disabled` values.
    pub bool_values: usize,
    /// `key = value`-style or `setParameter` entries.
    pub params: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Top-level section keys.
const SECTIONS: &[&str] = &[
    "systemLog",
    "storage",
    "net",
    "replication",
    "sharding",
    "security",
    "operationProfiling",
    "setParameter",
    "processManagement",
    "cloud",
    "snmp",
    "auditLog",
    "profile",
    "startupWarnings",
];

/// Well-known camelCase sub-keys.
const KEYS: &[&str] = &[
    "dbPath",
    "bindIp",
    "bindIpAll",
    "replSetName",
    "replSet",
    "wiredTiger",
    "destination",
    "logAppend",
    "logRotate",
    "verbosity",
    "quiet",
    "traceExceptions",
    "journal",
    "enabled",
    "engineConfig",
    "cacheSizeGB",
    "collectionConfig",
    "indexConfig",
    "directoryPerDB",
    "directoryForIndexes",
    "syncPeriodSecs",
    "port",
    "unixDomainSocket",
    "filePermissions",
    "pathPrefix",
    "ipv6",
    "maxIncomingConnections",
    "wireObjectCheck",
    "serviceExecutor",
    "compression",
    "transportLayer",
    "clusterIpSourceWhitelist",
    "tls",
    "mode",
    "certificateKeyFile",
    "CAFile",
    "allowConnectionsWithoutCertificates",
    "clientCertMode",
    "authorization",
    "keyFile",
    "clusterAuthMode",
    "javascriptEnabled",
    "enableLocalhostAuthBypass",
    "transitionToAuth",
    "configDB",
    "configsvr",
    "shardsvr",
    "archiveMovedChunks",
    "autoSplit",
    "chunkSize",
    "slowOpThresholdMs",
    "slowOpSampleRate",
    "profile",
    "mode",
    "oplogSizeMB",
    "secondaryIndexPrefetch",
    "enableMajorityReadConcern",
    "commitQuorum",
    "electionTimeoutMillis",
    "heartbeatIntervalMillis",
    "fork",
    "pidFilePath",
    "timeZoneInfo",
    "monitoring",
    "free",
    "sasl",
    "serviceName",
    "hostName",
    "ldap",
    "servers",
    "transportSecurity",
    "timeoutMS",
    "auditLog",
    "filter",
    "kmip",
    "keyIdentifier",
    "serverName",
    "enterpriseModule",
];

fn key_of_line(tr: &str) -> Option<(&str, &str)> {
    let colon = tr.find(':')?;
    if colon == 0 {
        return None;
    }
    let key = tr[..colon].trim_end();
    if !key
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '@'))
    {
        return None;
    }
    Some((key, tr[colon + 1..].trim_start()))
}

fn is_camel(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars().any(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether the buffer looks like a mongod/mongos YAML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut section_hits = 0usize;
    let mut camel_hits = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with('#') {
            continue;
        }
        if let Some((k, _)) = key_of_line(tr) {
            if SECTIONS.contains(&k) {
                section_hits += 1;
            }
            if is_camel(k) && KEYS.contains(&k) {
                camel_hits += 1;
            }
        }
    }
    section_hits >= 1 && camel_hits >= 1 || section_hits >= 2 || camel_hits >= 4
}

impl Mongod {
    /// Count sections/keys in a mongod YAML config. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            keys: 0,
            camel_keys: 0,
            bool_values: 0,
            params: 0,
            comments: 0,
        };
        let mut in_setparam = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = l.len() - tr.len();
            let Some((key, v)) = key_of_line(tr) else {
                continue;
            };
            c.keys += 1;
            if is_camel(key) {
                c.camel_keys += 1;
            }
            if indent == 0 {
                in_setparam = key == "setParameter";
                if SECTIONS.contains(&key) {
                    c.sections += 1;
                }
            }
            if in_setparam && indent > 0 {
                c.params += 1;
            }
            if matches!(v, "true" | "false" | "enabled" | "disabled" | "yes" | "no") {
                c.bool_values += 1;
            }
            if v.contains('=') && !v.starts_with('"') && !v.starts_with('\'') {
                c.params += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = br#"# mongod
systemLog:
  destination: file
  logAppend: true
  path: /var/log/mongodb/mongod.log
storage:
  dbPath: /var/lib/mongo
  journal:
    enabled: true
  wiredTiger:
    engineConfig:
      cacheSizeGB: 1
processManagement:
  fork: true
  pidFilePath: /var/run/mongodb/mongod.pid
  timeZoneInfo: /usr/share/zoneinfo
net:
  port: 27017
  bindIp: 127.0.0.1
security:
  authorization: enabled
replication:
  replSetName: rs0
setParameter:
  enableLocalhostAuthBypass: false
  syncdelay: 60
"#;
        assert!(detect(b));
        let c = Mongod::parse(b).unwrap();
        assert_eq!(c.sections, 7);
        assert_eq!(c.keys, 25);
        assert_eq!(c.bool_values, 5);
        assert_eq!(c.params, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_generic_yaml() {
        assert!(!detect(b"name: x\nversion: y\ndependencies:\n  - a\n"));
    }
}
