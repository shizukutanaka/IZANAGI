//! Flyway `flyway.conf` 設定ファイルの検出・カウント。
//!
//! `flyway.<カテゴリ>.<名>` / `flyway.<名>` 形式の `key=value` フラットプロパティ
//! (Flyway Teams/Community 両系、`flyway.placeholders.*`/`flyway.oracle.*` 等)。
//!
//! ```
//! let cfg = b"# flyway\n\
//!             flyway.url = jdbc:postgresql://db:5432/app\n\
//!             flyway.user = migrate\n\
//!             flyway.locations = classpath:db/migration\n\
//!             flyway.baselineOnMigrate = true\n\
//!             flyway.placeholders.tenant = core\n";
//! assert!(izanagi_kit::flyway::detect(cfg));
//! let c = izanagi_kit::flyway::parse(cfg).unwrap();
//! assert_eq!(c.entries, 5);
//! assert_eq!(c.connection, 2);
//! assert_eq!(c.placeholders, 1);
//! ```

/// 接続系キー (`flyway.<key>` の第2要素)。
const CONNECTION_KEYS: &[&str] = &[
    "url",
    "user",
    "password",
    "driver",
    "connectRetries",
    "connectRetriesInterval",
    "initSql",
    "jdbcProperties",
    "kerberosConfigFile",
    "kerberosLoginFile",
    "oracleKerberosCacheFile",
    "oracleKerberosConfigFile",
];

/// ロケーション/スキーマ系キー。
const LOCATION_KEYS: &[&str] = &[
    "locations",
    "schemas",
    "defaultSchema",
    "createSchemas",
    "failOnMissingLocations",
    "jarDirs",
    "workingDirectory",
];

/// マイグレーション挙動系キー。
const BEHAVIOR_KEYS: &[&str] = &[
    "table",
    "tablespace",
    "baselineVersion",
    "baselineDescription",
    "baselineOnMigrate",
    "cleanDisabled",
    "outOfOrder",
    "validateOnMigrate",
    "validateMigrationNaming",
    "mixed",
    "group",
    "batch",
    "stream",
    "target",
    "cherryPick",
    "skipExecutingMigrations",
    "skipDefaultResolvers",
    "skipDefaultCallbacks",
    "ignoreMigrationPatterns",
    "ignoreMissingMigrations",
    "ignoreIgnoredMigrations",
    "ignoreFutureMigrations",
    "cleanOnValidationError",
    "communityDBSupportEnabled",
    "installedBy",
    "callbacks",
    "errorOverrides",
];

/// 命名規則系キー。
const NAMING_KEYS: &[&str] = &[
    "sqlMigrationPrefix",
    "undoSqlMigrationPrefix",
    "repeatableSqlMigrationPrefix",
    "baselineMigrationPrefix",
    "sqlMigrationSeparator",
    "sqlMigrationSuffixes",
    "scriptFileExtensions",
];

/// プレースホルダ/エンコード系キー。
const PLACEHOLDER_KEYS: &[&str] = &[
    "placeholderPrefix",
    "placeholderSuffix",
    "placeholderReplacement",
    "encoding",
    "detectEncoding",
    "resourceProvider",
    "classProvider",
    "javaMigrationClassProvider",
];

/// 出力・実行系キー。
const OUTPUT_KEYS: &[&str] = &[
    "outputQueryResults",
    "outputType",
    "dryRunOutput",
    "progress",
    "loggers",
    "color",
    "command",
    "executable",
    "licenseKey",
    "offlineMode",
];

/// `key=value` の先頭要素 (`.` まで) を除いた第2要素を返す。
fn key_tail(line: &str) -> Option<&str> {
    let (key, _val) = line.split_once('=')?;
    let key = key.trim();
    let key = key.strip_prefix("flyway.")?;
    Some(key)
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `flyway.*` エントリ総数。
    pub entries: usize,
    /// 接続系エントリ数。
    pub connection: usize,
    /// ロケーション/スキーマ系エントリ数。
    pub locations: usize,
    /// マイグレーション挙動系エントリ数。
    pub behavior: usize,
    /// 命名規則系エントリ数。
    pub naming: usize,
    /// `flyway.placeholders.*` + プレースホルダ/エンコード系エントリ数。
    pub placeholders: usize,
    /// 出力・実行系エントリ数。
    pub output: usize,
    /// `flyway.oracle.*`/`flyway.postgresql.*`/`flyway.sqlserver.*`/`flyway.mysql.*`/`flyway.db2.*`/`flyway.redshift.*`/`flyway.snowflake.*`/`flyway.sybasease.*`/`flyway.informix.*`/`flyway.hsql.*`/`flyway.firebird.*`/`flyway.derby.*`/`flyway.sqlite.*` 等 DB 固有セクションのエントリ数。
    pub vendor: usize,
    /// その他 (`flyway.command.*` 等) エントリ数。
    pub misc: usize,
    /// `flyway.` 接頭辞なしの `key=value` 行数。
    pub plain: usize,
}

/// `b` が `flyway.conf` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries >= 2)
}

/// `b` を `flyway.conf` として解析し、キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        connection: 0,
        locations: 0,
        behavior: 0,
        naming: 0,
        placeholders: 0,
        output: 0,
        vendor: 0,
        misc: 0,
        plain: 0,
    };
    let mut saw_flyway = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with('!')
            || line.starts_with("//")
        {
            continue;
        }
        if let Some(tail) = key_tail(line) {
            saw_flyway = true;
            c.entries += 1;
            let head = tail.split('.').next().unwrap_or("");
            if tail.starts_with("placeholders.") {
                c.placeholders += 1;
            } else if CONNECTION_KEYS.contains(&tail) {
                c.connection += 1;
            } else if LOCATION_KEYS.contains(&tail) {
                c.locations += 1;
            } else if BEHAVIOR_KEYS.contains(&tail) {
                c.behavior += 1;
            } else if NAMING_KEYS.contains(&tail) {
                c.naming += 1;
            } else if PLACEHOLDER_KEYS.contains(&tail) {
                c.placeholders += 1;
            } else if OUTPUT_KEYS.contains(&tail) {
                c.output += 1;
            } else if matches!(
                head,
                "oracle"
                    | "postgresql"
                    | "sqlserver"
                    | "mysql"
                    | "mariadb"
                    | "db2"
                    | "redshift"
                    | "snowflake"
                    | "sybasease"
                    | "informix"
                    | "hsql"
                    | "firebird"
                    | "derby"
                    | "sqlite"
                    | "hana"
                    | "cockroachdb"
                    | "tidb"
                    | "clickhouse"
                    | "databricks"
                    | "bigquery"
                    | "spanner"
                    | "mongodb"
                    | "cassandra"
                    | "singlestoredb"
                    | "timescaledb"
                    | "yellowbrick"
                    | "yugabytedb"
                    | "oceanbase"
                    | "testContainers"
            ) {
                c.vendor += 1;
            } else {
                c.misc += 1;
            }
        } else if line.contains('=') && !line.starts_with("flyway") {
            c.plain += 1;
        }
    }
    if saw_flyway {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# flyway.conf\n\
        flyway.url = jdbc:postgresql://db.internal:5432/app\n\
        flyway.user = migrate\n\
        flyway.password = s3cret\n\
        flyway.locations = classpath:db/migration,filesystem:/opt/sql\n\
        flyway.schemas = public,audit\n\
        flyway.table = flyway_schema_history\n\
        flyway.baselineOnMigrate = true\n\
        flyway.outOfOrder = false\n\
        flyway.sqlMigrationPrefix = V\n\
        flyway.repeatableSqlMigrationPrefix = R\n\
        flyway.placeholderReplacement = true\n\
        flyway.placeholders.tenant = core\n\
        flyway.outputQueryResults = false\n\
        flyway.oracle.sqlplus = true\n\
        flyway.oracle.sqlplusWarn = true\n";

    #[test]
    fn detects_flyway() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 15);
        assert_eq!(c.connection, 3);
        assert_eq!(c.locations, 2);
        assert_eq!(c.behavior, 3);
        assert_eq!(c.naming, 2);
        assert_eq!(c.placeholders, 2);
        assert_eq!(c.output, 1);
        assert_eq!(c.vendor, 2);
    }

    #[test]
    fn rejects_other_props() {
        assert!(!detect(b"key = value\nother = thing\n"));
        assert!(!detect(b"flyway.url = jdbc:h2:mem\n"));
    }
}
