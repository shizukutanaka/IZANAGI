//! Parser for Zabbix agent/server configuration files (`zabbix_agentd.conf`,
//! `zabbix_server.conf`, `zabbix_proxy.conf`, `zabbix_agent2.conf`).
//!
//! Counts `Option=Value` directives: `Server`, `ServerActive`, `Hostname`,
//! `ListenIP`/`ListenPort`/`StartAgents`/`StartPollers`/`StartTrappers`,
//! `LogFile`/`DebugLevel`/`Timeout`, `DBHost`/`DBName`/`DBUser`,
//! `TLSConnect`/`TLSAccept`/`TLSCertFile`, `UserParameter`/`Alias`,
//! `Include`/`LoadModule`, `AllowKey`/`DenyKey`, `EnableRemoteCommands`,
//! `UnsafeUserParameters`, `ProxyMode`/`BufferSize` family, and `#` comments.
//!
//! ```
//! let b = b"Server=127.0.0.1\nHostname=zbx-agent\nUserParameter=mem.free,free\n";
//! assert!(izanagi_kit::zabbix::detect(b));
//! let c = izanagi_kit::zabbix::Zabbix::parse(b).unwrap();
//! assert_eq!(c.options, 3);
//! assert_eq!(c.user_parameters, 1);
//! ```

/// Parsed zabbix conf summary.
#[derive(Debug, Clone)]
pub struct Zabbix {
    /// Total `Option=Value` lines.
    pub options: usize,
    /// `Server`/`ServerActive`/`Server`/`ListenIP`/`ListenPort`/`ListenBacklog`/`SourceIP`/`StatsAllowedIP`/`ProxyMode`/`HanodeName`/`NodeAddress`/`ProxyOfflineBuffer`/`ProxyLocalBuffer`/`HeartBeatFrequency`/`ConfigFrequency`/`DataSenderFrequency` network/connectivity keys.
    pub network_keys: usize,
    /// `StartAgents`/`StartPollers`/`StartIPMIPollers`/`StartTrappers`/`StartPingers`/`StartDiscoverers`/`StartHTTPPollers`/`StartTimers`/`StartEscalators`/`StartAlerters`/`StartLLDProcessors`/`StartDBSyncers`/`StartHistoryPollers`/`StartODBCRequests`/`StartProxyPollers`/`StartReportWriters`/`ServiceManagerSyncFrequency`/`ProblemHousekeepingFrequency`/`HousekeepingFrequency`/`MaxHousekeeperDelete` poller/cache keys.
    pub poller_keys: usize,
    /// `LogFile`/`LogFileSize`/`LogType`/`DebugLevel`/`PidFile`/`SocketDir`/`ControlSocket`/`TmpDir`/`FpingLocation`/`Fping6Location`/`SSHKeyLocation`/`SSLCertLocation`/`SSLKeyLocation`/`SSLCALocation`/`SlowLogFile`/`SlowLogTime`/`GlobalScriptPath`/`ExternalScripts`/`LogRemoteCommands`/`EnableRemoteCommands`/`AllowRoot`/`User`/`AllowUnsupportedDBVersions`/`SNMPTrapperFile`/`ExportDir`/`ExportFileSize`/`ExportType`/`ProblemName`/`OKTriggers`/`NotClassifiedTriggers`/`InformationTriggers`/`WarningTriggers`/`AverageTriggers`/`HighTriggers`/`DisasterTriggers` logging/infra keys.
    pub infra_keys: usize,
    /// `Timeout`/`TrapperTimeout`/`UnreachablePeriod`/`UnavailableDelay`/`UnreachableDelay`/`SleepTime`/`BufferSize`/`BufferSend`/`CacheSize`/`CacheUpdateFrequency`/`HistoryCacheSize`/`HistoryIndexCacheSize`/`TrendCacheSize`/`ValueCacheSize`/`ProxyMemoryBufferSize`/`ProxyBufferMode` timing/buffer keys.
    pub buffer_keys: usize,
    /// `DBHost`/`DBName`/`DBSchema`/`DBUser`/`DBPassword`/`DBSocket`/`DBPort`/`DBTLSConnect`/`DBTLSCertFile`/`DBTLSKeyFile`/`DBTLSCAFile`/`DBTLSCipher`/`DBTLSCipher13`/`HistoryStorageURL`/`HistoryStorageTypes`/`HistoryStorageDateIndex`/`Vault`/`VaultToken`/`VaultURL`/`VaultDBPath`/`VaultTLSCertFile`/`VaultTLSKeyFile`/`VertexDBObject`/`VertexDBJunction` database/vault keys.
    pub db_keys: usize,
    /// `TLSConnect`/`TLSAccept`/`TLSCAFile`/`TLSCRLFile`/`TLSServerCertIssuer`/`TLSServerCertSubject`/`TLSCertFile`/`TLSKeyFile`/`TLSPSKIdentity`/`TLSPSKFile`/`AllowKey`/`DenyKey`/`UnsafeUserParameters`/`OEMode` security keys.
    pub security_keys: usize,
    /// `UserParameter`/`Alias` lines.
    pub user_parameters: usize,
    /// `Include`/`LoadModulePath`/`LoadModule`/`Plugin`/`Plugins.` plugin lines.
    pub include_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const NET_KEYS: &[&str] = &[
    "Server",
    "ServerActive",
    "Hostname",
    "HostnameItem",
    "ListenIP",
    "ListenPort",
    "ListenBacklog",
    "SourceIP",
    "StatsAllowedIP",
    "ProxyMode",
    "HanodeName",
    "NodeAddress",
    "ProxyOfflineBuffer",
    "ProxyLocalBuffer",
    "HeartBeatFrequency",
    "ConfigFrequency",
    "DataSenderFrequency",
];

const POLLER_KEYS: &[&str] = &[
    "StartAgents",
    "StartPollers",
    "StartIPMIPollers",
    "StartTrappers",
    "StartPingers",
    "StartDiscoverers",
    "StartHTTPPollers",
    "StartTimers",
    "StartEscalators",
    "StartAlerters",
    "StartLLDProcessors",
    "StartDBSyncers",
    "StartHistoryPollers",
    "StartODBCRequests",
    "StartProxyPollers",
    "StartReportWriters",
    "ServiceManagerSyncFrequency",
    "ProblemHousekeepingFrequency",
    "HousekeepingFrequency",
    "MaxHousekeeperDelete",
];

const INFRA_KEYS: &[&str] = &[
    "LogFile",
    "LogFileSize",
    "LogType",
    "DebugLevel",
    "PidFile",
    "SocketDir",
    "ControlSocket",
    "TmpDir",
    "FpingLocation",
    "Fping6Location",
    "SSHKeyLocation",
    "SSLCertLocation",
    "SSLKeyLocation",
    "SSLCALocation",
    "SlowLogFile",
    "SlowLogTime",
    "GlobalScriptPath",
    "ExternalScripts",
    "LogRemoteCommands",
    "EnableRemoteCommands",
    "AllowRoot",
    "User",
    "AllowUnsupportedDBVersions",
    "SNMPTrapperFile",
    "ExportDir",
    "ExportFileSize",
    "ExportType",
    "ProblemName",
    "OKTriggers",
    "NotClassifiedTriggers",
    "InformationTriggers",
    "WarningTriggers",
    "AverageTriggers",
    "HighTriggers",
    "DisasterTriggers",
];

const BUFFER_KEYS: &[&str] = &[
    "Timeout",
    "TrapperTimeout",
    "UnreachablePeriod",
    "UnavailableDelay",
    "UnreachableDelay",
    "BufferSize",
    "BufferSend",
    "CacheSize",
    "CacheUpdateFrequency",
    "HistoryCacheSize",
    "HistoryIndexCacheSize",
    "TrendCacheSize",
    "ValueCacheSize",
    "ProxyMemoryBufferSize",
    "ProxyBufferMode",
];

const DB_KEYS: &[&str] = &[
    "DBHost",
    "DBName",
    "DBSchema",
    "DBUser",
    "DBPassword",
    "DBSocket",
    "DBPort",
    "DBTLSConnect",
    "DBTLSCertFile",
    "DBTLSKeyFile",
    "DBTLSCAFile",
    "DBTLSCipher",
    "DBTLSCipher13",
    "HistoryStorageURL",
    "HistoryStorageTypes",
    "HistoryStorageDateIndex",
    "Vault",
    "VaultToken",
    "VaultURL",
    "VaultDBPath",
    "VaultTLSCertFile",
    "VaultTLSKeyFile",
    "VertexDBObject",
    "VertexDBJunction",
];

const SEC_KEYS: &[&str] = &[
    "TLSConnect",
    "TLSAccept",
    "TLSCAFile",
    "TLSCRLFile",
    "TLSServerCertIssuer",
    "TLSServerCertSubject",
    "TLSCertFile",
    "TLSKeyFile",
    "TLSPSKIdentity",
    "TLSPSKFile",
    "AllowKey",
    "DenyKey",
    "UnsafeUserParameters",
    "OEMode",
];

/// Returns `true` when the bytes look like a zabbix conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let sig = [
        "Server=",
        "ServerActive=",
        "Hostname=",
        "UserParameter=",
        "Include=",
    ]
    .iter()
    .filter(|s| t.contains(**s))
    .count();
    sig >= 2
}

impl Zabbix {
    /// Parses a zabbix conf, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            options: 0,
            network_keys: 0,
            poller_keys: 0,
            infra_keys: 0,
            buffer_keys: 0,
            db_keys: 0,
            security_keys: 0,
            user_parameters: 0,
            include_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some((k, _)) = tr.split_once('=') {
                let key = k.trim();
                c.options += 1;
                if NET_KEYS.contains(&key) {
                    c.network_keys += 1;
                } else if POLLER_KEYS.contains(&key) {
                    c.poller_keys += 1;
                } else if INFRA_KEYS.contains(&key) {
                    c.infra_keys += 1;
                } else if BUFFER_KEYS.contains(&key) {
                    c.buffer_keys += 1;
                } else if DB_KEYS.contains(&key) {
                    c.db_keys += 1;
                } else if SEC_KEYS.contains(&key) {
                    c.security_keys += 1;
                } else if key == "UserParameter" || key == "Alias" {
                    c.user_parameters += 1;
                } else if key == "Include"
                    || key == "LoadModulePath"
                    || key == "LoadModule"
                    || key.starts_with("Plugins.")
                {
                    c.include_keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# agent\nServer=127.0.0.1\nServerActive=zbx.local\nHostname=zbx-agent\nListenIP=0.0.0.0\nListenPort=10050\nStartAgents=3\nLogFile=/var/log/zabbix/zabbix_agentd.log\nDebugLevel=3\nTimeout=30\nTLSCertFile=/etc/zabbix/cert.pem\nUserParameter=mem.free,free\nInclude=/etc/zabbix/zabbix_agentd.d/\n";

    #[test]
    fn parses_zabbix() {
        let c = Zabbix::parse(CONF).unwrap();
        assert_eq!(c.options, 12);
        assert_eq!(c.network_keys, 5);
        assert_eq!(c.poller_keys, 1);
        assert_eq!(c.infra_keys, 2);
        assert_eq!(c.buffer_keys, 1);
        assert_eq!(c.security_keys, 1);
        assert_eq!(c.user_parameters, 1);
        assert_eq!(c.include_keys, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_zabbix() {
        assert!(!detect(b"key=value\nother=thing"));
        assert!(Zabbix::parse(b"x").is_none());
    }
}
