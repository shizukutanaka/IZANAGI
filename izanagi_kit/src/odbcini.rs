//! ODBC `odbc.ini` / `odbcinst.ini` census.
//!
//! `[ODBC Data Sources]` maps `DSN = driver name`, `[DSN]` sections
//! carry `Driver`/`Server`/`ServerName`/`Address`/`Network Address`/
//! `Database`/`Uid`/`UID`/`Pwd`/`PWD`/`Port`/`Trusted_Connection`/
//! `TrustServerCertificate`/`Encryption`/`TDS_Version`/`APP`/`WSID`/
//! `Description`/`QuotedId`/`AnsiNPW`/`Language`/`Region`/`FalBACK`/
//! `setup`/`filedsn`/`readonly`/`charset`/`client charset`/`DateTime`/
//! `Timeouts`/`Trace`/`TraceFile`/`TraceLib`/`Debug`/`Option`/`Socket`/
//! `Reconnect`/`Prepare Method`/`DescribeParam`/`Quoted Identifier`/
//! `UseRegionalSettings`/`AutoTranslate`/`SQLDumpPath`/`SQLGetTypeInfo`/
//! `Metadata`/`fetch_buffer_size`/`scroll`/`cache`/`compress`/`synt`/
//! `dialect`/`dsn`/`SERVER`/`DATABASE`/`HOST`/`SERVICE`/`PROTOCOL`/
//! `Options`/`UserName`/`Password`/`Integrated`/`Kerberos`/`SSL`/`SSLMode`/
//! `SSLCA`/`SSLCert`/`SSLKey`/`CertificateFile`/`CertificatePassword`/
//! `Locale`/`Product`/`Provider`/`UsageCount`/`SERVERNAME`/`LOG`/`LOGFILE`/
//! `LOGSERVER`/`LOGPROTOCOL`/`AUTO_RECONNECT`/`MAXCONN`/`CATALOG`/
//! `PACKETSIZE`/`COMPRESS`/`DRIVER`/`DESCRIPTION`/`SERVERNAME`/`API`/
//! `network`/`NetLib`/`address`/`ODBC_LANG`/`LANG`/`interfaces`/
//! `Connections`/`TLSEnabled`/`KeyStore`/`KeyStorePassword`/`TrustStore`/
//! `TrustStorePassword`/`MARS_Connection`/`MultiSubnetFailover`/
//! `ApplicationIntent`/`ConnectRetryCount`/`ConnectRetryInterval`/
//! `Authentication`/`AccessToken`/`ColumnEncryption`/`KeyStoreAuthentication`/
//! `KeyStorePrincipalId`/`KeyStoreSecret`/`HostnameInCertificate`/
//! `IpAddressPreference`/`ServerCertificate`/`PacketSize`/`CommandLogOnTime`/
//! `ConnectionStringFormat`/`UseFMTONLY`/`AppendRdatap`/`Filter`/
//! `UserDefaultSchema`/`Ansi`/`TranslationDLL`/`TranslationName`/
//! `TranslationOption`/`No characterset conversion`/`AsIs`/`Ignore`/
//! `FastSQLPrepare`/`AutoStopEngine`/`CommLinks`/`COMPRESS`/`Compression`/
//! `PrefetchBuffer`/`PrefetchRows`/`PrefetchMemory`/`SimpleEncryption`/
//! `Start`/`EngineName`/`DBN`/`DBF`/`UID`/`PWD`/`ASTOP`/`INT`/`CON`/
//! `LINKS`/`PREFETCH`/`CommBufferSize`/`CommBufferSpace`/`liveness_timeout`/
//! `idle_timeout`/`srv`/`update_statistics`/`arithmetic_abort`/
//! `quoted_identifier`/`ansi_nulls`/`ansi_warnings`/`ansi_padding`/
//! `concat_null_yields_null`/`numeric_roundabort`/`implicit_transactions`/
//! `cursor_close_on_commit`/`ansi_defaults`/`workstation`/`version`/
//! `text_size`/`net_address`/`packet_size`/`clientname`/`cpres`.
//!
//! `odbcinst.ini` has `[ODBC Drivers]` + `[driver]` sections with
//! `Driver`/`Setup`/`UsageCount`/`Description`/`FileUsage`/`FileExtns`/
//! `Threading`/`APILevel`/`ConnectFunctions`/`SQLLevel`/`DriverODBCVer`.
//!
//! ```rust
//! let o = "[ODBC Data Sources]\nmydb = PostgreSQL\n[mydb]\nDriver = /usr/lib/psqlodbc.so\nServer = db1\nDatabase = app\n";
//! let c = izanagi_kit::odbcini::Odbcini::parse(o.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.dsns, 1);
//! ```

/// ODBC ini census.
#[derive(Debug, Clone)]
pub struct Odbcini {
    /// `[section]` headers.
    pub sections: usize,
    /// Entries inside `[ODBC Data Sources]` (DSN → driver) or `[ODBC Drivers]` (name → Y).
    pub dsns: usize,
    /// All `key = value` entries.
    pub settings: usize,
    /// Recognised ODBC attribute names.
    pub named: usize,
}

const KEYS: &[&str] = &[
    "driver",
    "server",
    "servername",
    "address",
    "network address",
    "database",
    "uid",
    "pwd",
    "port",
    "trusted_connection",
    "trustservercertificate",
    "encryption",
    "tds_version",
    "app",
    "wsid",
    "description",
    "quotedid",
    "ansinpw",
    "language",
    "region",
    "setup",
    "filedsn",
    "readonly",
    "charset",
    "client charset",
    "trace",
    "tracefile",
    "tracelib",
    "debug",
    "option",
    "socket",
    "reconnect",
    "use regional settings",
    "autotranslate",
    "locale",
    "product",
    "provider",
    "usagecount",
    "log",
    "logfile",
    "catalog",
    "packetsize",
    "compress",
    "api",
    "network",
    "netlib",
    "interfaces",
    "connections",
    "tlsenabled",
    "keystore",
    "keystorepassword",
    "truststore",
    "truststorepassword",
    "mars_connection",
    "multisubnetfailover",
    "applicationintent",
    "connectretrycount",
    "connectretryinterval",
    "authentication",
    "accesstoken",
    "columnencryption",
    "keystoreauthentication",
    "keystoreprincipalid",
    "keystoresecret",
    "hostnameincertificate",
    "ipaddresspreference",
    "servercertificate",
    "commandlogontime",
    "connectionstringformat",
    "usefmtonly",
    "userdefaultschema",
    "ansi",
    "translationdll",
    "translationname",
    "translationoption",
    "fastsqlprepare",
    "autostopengine",
    "commlinks",
    "compression",
    "prefetchbuffer",
    "prefetchrows",
    "prefetchmemory",
    "simpleencryption",
    "start",
    "enginename",
    "dbn",
    "dbf",
    "astop",
    "int",
    "con",
    "links",
    "prefetch",
    "commbuffersize",
    "commbufferspace",
    "liveness_timeout",
    "idle_timeout",
    "srv",
    "update_statistics",
    "arithmetic_abort",
    "quoted_identifier",
    "ansi_nulls",
    "ansi_warnings",
    "ansi_padding",
    "concat_null_yields_null",
    "numeric_roundabort",
    "implicit_transactions",
    "cursor_close_on_commit",
    "ansi_defaults",
    "workstation",
    "version",
    "text_size",
    "net_address",
    "packet_size",
    "clientname",
    "cpres",
    "host",
    "service",
    "protocol",
    "options",
    "username",
    "password",
    "integrated",
    "kerberos",
    "ssl",
    "sslmode",
    "sslca",
    "sslcert",
    "sslkey",
    "certificatefile",
    "certificatepassword",
    "dsn",
    "fileusage",
    "fileextns",
    "threading",
    "apilevel",
    "connectfunctions",
    "sqllevel",
    "driverodbcver",
];

/// Whether the buffer looks like an ODBC ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let l = t.to_lowercase();
    l.contains("[odbc data sources]")
        || l.contains("[odbc drivers]")
        || l.contains("[odbc]")
        || l.contains("trusted_connection")
        || l.contains("mars_connection")
        || l.contains("usagecount")
        || l.contains("driver") && l.contains("server") && l.contains("database")
}

impl Odbcini {
    /// Parse an ODBC ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            dsns: 0,
            settings: 0,
            named: 0,
        };
        let mut in_list = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with(';') || s.starts_with('#') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                let name = s[1..s.len() - 1].to_lowercase();
                in_list = name == "odbc data sources" || name == "odbc drivers";
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            c.settings += 1;
            if in_list {
                c.dsns += 1;
            }
            if KEYS.contains(&s[..eq].trim().to_lowercase().as_str()) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_odbc_ini() {
        let b = concat!(
            "[ODBC Data Sources]\n",
            "mydb = PostgreSQL Driver\n",
            "legacy = FreeTDS\n",
            "[mydb]\n",
            "Driver = /usr/lib/x86_64-linux-gnu/odbc/psqlodbcw.so\n",
            "Server = db1.internal\n",
            "Port = 5432\n",
            "Database = app\n",
            "SSLmode = verify-full\n",
            "[legacy]\n",
            "Driver = /usr/lib/libtdsodbc.so\n",
            "Server = sql1\n",
            "Port = 1433\n",
            "TDS_Version = 7.4\n",
            "Trusted_Connection = No\n",
        );
        let c = Odbcini::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.dsns, 2);
        assert_eq!(c.settings, 12);
        assert!(c.named >= 9);
    }

    #[test]
    fn parses_odbcinst() {
        let b = concat!(
            "[ODBC Drivers]\n",
            "PostgreSQL = Installed\n",
            "FreeTDS = Installed\n",
            "[PostgreSQL]\n",
            "Driver = /usr/lib/psqlodbcw.so\n",
            "Setup = /usr/lib/libodbcpsqlS.so\n",
            "UsageCount = 2\n",
            "Threading = 2\n",
        );
        let c = Odbcini::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.dsns, 2);
        assert_eq!(c.settings, 6);
    }

    #[test]
    fn rejects_other() {
        assert!(Odbcini::parse(b"[x]\nfoo=1").is_none());
    }
}
