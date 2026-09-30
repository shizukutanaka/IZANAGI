//! IBM DB2 `db2cli.ini` census.
//!
//! `[dbname]` sections (one per cataloged database / DSN) plus a
//! `[COMMON]` section, with `key=value` entries:
//! `Hostname`/`HostnameType`/`Port`/`Protocol`/`Database`/`DB2CODEPAGE`/
//! `SchemaList`/`TableTypes`/`PWD`/`UID`/`PWDPlugin`/`DBALIAS`/`Alias`/
//! `BIDI`/`BITDATA`/`BLOCKFORNROWS`/`BLOCKLOBS`/`BLOCKSIZE`/`CATALOG`/
//! `CLISCHEMA`/`CONNECTTYPE`/`CURRENTSCHEMA`/`CURSORHOLD`/`DATEDELTA`/
//! `DATELITERAL`/`DECFLOAT`/`DEFERREDPREPARE`/`DESCRIBEPARAMSTATIC`/
//! `DIAGLEVEL`/`DISABLEKEYSETCURSOR`/`EARLTERMINATE`/`ENCRYPT`/`FILEDSN`/
//! `GRANTEE`/`GRANTOR`/`HOSTNAME`/`IGNOREWARNINGS`/`INSTANCE`/`INT`/
//! `INTERRUPT`/`LOBMAXCOLUMNSIZE`/`LONGDATACOMPAT`/`MAP2DESCRIBE`/
//! `MAXCONN`/`MAXRSET`/`MODE`/`NOTICEDELAY`/`NOTIFICATION`/`OPTIMIZEFOR`/
//! `OPTIMIZEFORNROWS`/`PATCH1`/`PATCH2`/`PROGRAM`/`QUERYTIMEOUT`/
//! `RECEIVEBUFFER`/`REOPT`/`RETCATALOG`/`RETCATALOGASCURRSERVER`/
//! `RETCATALOGASCURRSERVER`/`RETCATALOGASCURRSERVER`/`RETCATALOGAS`/
//! `RETCATALOGCURSOR`/`RETURNSCHEMA`/`REUSEMEMORY`/`SAVEPOINTNAME`/
//! `SCHEMALIST`/`SERVERTYPE`/`SERVICE`/`SQL_ATTR`/`SQLWARNING`/
//! `STATICCAPFILE`/`STATICLOGFILE`/`STATICMODE`/`STATICPACKAGE`/
//! `STATICPROFILE`/`STATICSYSPARM`/`SYNONYM`/`TABLETYPE`/`TCPIPGENTRIES`/
//! `TEMPDIR`/`TIMEDELTA`/`TIMELITERAL`/`TRACECOMM`/`TRACEFILENAME`/
//! `TRACEFLUSH`/`TRACEMODE`/`TRACEREFRESHINTERVAL`/`TRACETIMESTAMP`/
//! `TRANSLATEDLL`/`TRANSLATEOPTION`/`TXNISOLATION`/`UNDERSCORE`/
//! `UNIOQE`/`USELIBRARYLIST`/`USENAMESPACE`/`USEOLDSTP`/`USERCURENTSCHEMA`/
//! `USETRUSTEDCONNECTION`/`VARCHARMAX`/`WCHARTYPE`/`XMLSTRUCTURE`/
//! `CODEPAGECONVERTER`/`AUTHENTICATION`/`CLIENTENCALG`/`CLIENTHOSTNAME`/
//! `CLIENTUSERID`/`CLIENTWRKSTNNAME`/`CLIENTACCTSTR`/`CLIENTAPPLCOMPAT`/
//! `CLIENTAPPLNAME`/`CONNECT_NODE`/`CURRENTFUNCTIONPATH`/`CURRENTLOCALE`/
//! `CURRENTMAINTAINED`/`CURRENTOPTIMIZATIONPROFILE`/`CURRENTPACKAGESERIES`/
//! `CURRENTREFRESHAGE`/`CURRENTSCHEMA`/`CURRENTSQLID`/`DB2NODE`/
//! `DB2TERRITORY`/`DBALIAS`/`DBNAME`/`DBPATH`/`DFTPROTOCOL`/`DFTDB`/
//! `DIRECTORY`/`DISABLECLIENTREBIND`/`DISABLEEXTLBLS`/`DISABLEKEYSET`/
//! `DISABLESCROL`/`DISABLEUPSCROL`/`FETCHFIRST`/`FET_BUF_SIZE`/
//! `FET_BUF_SIZE`/`HOST`/`IGNTAPE`/`INHAERITANCES`/`KEEPDYNAMIC`/
//! `KEYWORDS`/`LOCALE`/`LOCKTIMEOUT`/`LOGONAUTH`/`MACHINE`/
//! `MEMORY`/`MULTICONNECT`/`NODE`/`ODM`/`OVERWRITE`/`PASSWORD`/
//! `PATH`/`PERF`/`PERM`/`PLEMODE`/`PORT`/`PROC`/`PROFILE`/`PROGRAM`/
//! `PROTOCOL`/`PROVIDER`/`QUERY`/`READONLY`/`REFRESH`/`REMOTE`/
//! `REPL`/`REPORT`/`REQ`/`RES`/`RETRY`/`REUSE`/`ROL`/`ROW`/`RPC`/
//! `RS`/`SAA`/`SANITIZE`/`SCHEMA`/`SECURITY`/`SERIES`/`SERVER`/
//! `SERVICE`/`SESSION`/`SHARE`/`SIGNAL`/`SIZE`/`SKIP`/`SNA`/`SOCKET`/
//! `SPECIAL`/`SQL`/`SQLSTATE`/`SSL`/`STMT`/`STORAGE`/`STREAM`/`STRING`/
//! `STRUCTURE`/`SWITCH`/`SYNCPOINT`/`SYSNAME`/`TAB`/`TABLE`/`TABLESPACE`/
//! `TARGET`/`TASK`/`TCP`/`TERMINATE`/`TERR`/`THR`/`TID`/`TIME`/`TIMEOUT`/
//! `TIMESTAMP`/`TLS`/`TMP`/`TRACE`/`TRANS`/`TRUST`/`TYPE`/`TYPE`/`UID`/
//! `USER`/`VERSION`/`WINDOWS`/`WORKSPACE`/`WRKSTN`/`XA`/`XML`/`ZONE`.
//!
//! ```rust
//! let d = "[SAMPLE]\nDatabase=SAMPLE\nHostname=db2.internal\nPort=50000\nProtocol=TCPIP\n";
//! let c = izanagi_kit::db2cli::Db2cli::parse(d.as_bytes()).unwrap();
//! assert_eq!(c.sections, 1);
//! assert_eq!(c.settings, 4);
//! ```

/// db2cli.ini census.
#[derive(Debug, Clone)]
pub struct Db2cli {
    /// `[dbname]`/`[COMMON]` section headers.
    pub sections: usize,
    /// `key=value` entries.
    pub settings: usize,
    /// Recognised DB2 CLI keyword names (case-insensitive).
    pub named: usize,
}

const KEYS: &[&str] = &[
    "database",
    "hostname",
    "hostnametype",
    "port",
    "protocol",
    "uid",
    "pwd",
    "schemalist",
    "tabletypes",
    "pwdsplugin",
    "dbalias",
    "alias",
    "bidi",
    "bitdata",
    "blockfornrows",
    "blocklobs",
    "blocksize",
    "catalog",
    "clischema",
    "connecttype",
    "currentschema",
    "cursorhold",
    "datedelta",
    "dateliteral",
    "decfloat",
    "deferredprepare",
    "describeparamstatic",
    "diaglevel",
    "disablekeysetcursor",
    "encrypt",
    "filedsn",
    "grantee",
    "grantor",
    "instance",
    "interrupt",
    "lobmaxcolumnsize",
    "longdatacompat",
    "map2describe",
    "maxconn",
    "maxrset",
    "mode",
    "noticedelay",
    "notification",
    "optimizefor",
    "optimizefornrows",
    "patch1",
    "patch2",
    "program",
    "querytimeout",
    "receivebuffer",
    "reopt",
    "retcatalog",
    "retcatalogcursor",
    "returnschema",
    "reusememory",
    "savepointname",
    "servertype",
    "service",
    "sqlwarning",
    "staticcapfile",
    "staticlogfile",
    "staticmode",
    "staticpackage",
    "staticprofile",
    "staticsysparm",
    "synonym",
    "tabletype",
    "tcpipgentries",
    "tempdir",
    "timedelta",
    "timeliteral",
    "tracecomm",
    "tracefilename",
    "traceflush",
    "tracemode",
    "tracerefreshinterval",
    "tracetimestamp",
    "translatedll",
    "translateoption",
    "txnisolation",
    "underscore",
    "uselibrarylist",
    "usenamespace",
    "useoldstp",
    "usetrustedconnection",
    "varcharmax",
    "wchartype",
    "xmlstructure",
    "codepageconverter",
    "authentication",
    "clientencalg",
    "clienthostname",
    "clientuserid",
    "clientwrkstnname",
    "clientacctstr",
    "clientapplcompat",
    "clientapplname",
    "connect_node",
    "currentfunctionpath",
    "currentlocale",
    "currentmaintained",
    "currentoptimizationprofile",
    "currentpackageseries",
    "currentrefreshage",
    "currentsqlid",
    "db2codepage",
    "db2node",
    "db2territory",
    "dbname",
    "dbpath",
    "dftprotocol",
    "dftdb",
    "directory",
    "disableclientrebind",
    "disableextlbls",
    "disablescrol",
    "disableupscrol",
    "fetchfirst",
    "fet_buf_size",
    "host",
    "keepdynamic",
    "keywords",
    "locale",
    "locktimeout",
    "logonauth",
    "machine",
    "memory",
    "multiconnect",
    "node",
    "odm",
    "overwrite",
    "password",
    "path",
    "perf",
    "perm",
    "plemode",
    "proc",
    "profile",
    "provider",
    "query",
    "readonly",
    "refresh",
    "remote",
    "repl",
    "report",
    "req",
    "res",
    "retry",
    "reuse",
    "rol",
    "row",
    "rpc",
    "rs",
    "saa",
    "sanitize",
    "schema",
    "security",
    "series",
    "server",
    "session",
    "share",
    "signal",
    "size",
    "skip",
    "sna",
    "socket",
    "special",
    "sql",
    "sqlstate",
    "ssl",
    "stmt",
    "storage",
    "stream",
    "string",
    "structure",
    "switch",
    "syncpoint",
    "sysname",
    "tab",
    "table",
    "tablespace",
    "target",
    "task",
    "tcp",
    "terminate",
    "terr",
    "thr",
    "tid",
    "time",
    "timeout",
    "timestamp",
    "tls",
    "tmp",
    "trace",
    "trans",
    "trust",
    "type",
    "user",
    "version",
    "windows",
    "workspace",
    "wrkstn",
    "xa",
    "xml",
    "zone",
    "common",
];

/// Whether the buffer looks like db2cli.ini.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let l = t.to_lowercase();
    l.contains("hostname") && l.contains("port") && l.contains("protocol")
        || l.contains("db2codepage")
        || l.contains("txnisolation")
        || l.contains("clischema")
        || l.contains("schemalist")
        || l.contains("[common]") && l.contains("db2")
}

impl Db2cli {
    /// Parse a db2cli.ini into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            named: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with(';') || s.starts_with('#') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                continue;
            }
            let Some(eq) = s.find('=') else {
                continue;
            };
            let key = s[..eq].trim();
            if !key.is_empty() {
                c.settings += 1;
                if KEYS.contains(&key.to_lowercase().as_str()) {
                    c.named += 1;
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
    fn parses_ini() {
        let b = concat!(
            "[SAMPLE]\n",
            "Database=SAMPLE\n",
            "Hostname=db2.internal\n",
            "Port=50000\n",
            "Protocol=TCPIP\n",
            "UID=svc\n",
            "PWD=hunter2\n",
            "[COMMON]\n",
            "QueryTimeout=60\n",
            "CurrentSchema=APP\n",
            "TraceComm=0\n",
        );
        let c = Db2cli::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 2);
        assert_eq!(c.settings, 9);
        assert_eq!(c.named, 9);
    }

    #[test]
    fn rejects_other() {
        assert!(Db2cli::parse(b"[x]\nfoo=1").is_none());
    }
}
