//! TypeORM `data-source.ts`/`ormconfig.json`/`ormconfig.env` の検出・カウント。
//!
//! JS/TS (`new DataSource({…})`/`module.exports`)・JSON・env (`TYPEORM_*`) の
//! 3形態を扱う。`type`/`synchronize`/`entities`/`migrations`/`subscribers`/
//! `migrationsTableName`/`cli.*` 等のキーを集計する。
//!
//! ```
//! let cfg = br#"{
//!           "type": "postgres",
//!           "host": "db",
//!           "synchronize": false,
//!           "entities": ["src/entity/*.ts"],
//!           "migrations": ["src/migration/*.ts"]
//!           }"#;
//! assert!(izanagi_kit::typeormconf::detect(cfg));
//! let c = izanagi_kit::typeormconf::parse(cfg).unwrap();
//! assert!(c.db_keys >= 2);
//! assert!(c.orm_keys >= 1);
//! ```

/// DB 接続キー。
const DB_KEYS: &[&str] = &[
    "type",
    "host",
    "port",
    "username",
    "password",
    "database",
    "url",
    "sid",
    "serviceName",
    "schema",
    "charset",
    "extra",
    "driver",
    "socketPath",
    "instanceName",
    "domain",
];

/// ORM 振る舞いキー。
const ORM_KEYS: &[&str] = &[
    "synchronize",
    "dropSchema",
    "migrationsRun",
    "migrationsTransactionMode",
    "metadataTableName",
    "logging",
    "logger",
    "maxQueryExecutionTime",
    "namingStrategy",
    "entityPrefix",
    "relationLoadStrategy",
    "entitySkipConstructor",
    "cache",
    "isolateWhereStatements",
    "poolSize",
    "poolErrorHandler",
    "supportBigNumbers",
    "bigNumberStrings",
    "installExtensions",
    "applicationName",
    "connectionTimeout",
    "acquireTimeout",
    "statementCacheSize",
    "transactionType",
];

/// パス/ディレクトリ系キー。
const PATH_KEYS: &[&str] = &[
    "entities",
    "migrations",
    "subscribers",
    "factories",
    "seeds",
    "entitiesDir",
    "migrationsDir",
    "subscribersDir",
    "cli",
    "migrationsTableName",
];

/// TypeORM 既知の `type`/`TYPEORM_CONNECTION` 値。
const DIALECT_VALUES: &[&str] = &[
    "mysql",
    "mariadb",
    "postgres",
    "cockroachdb",
    "sqlite",
    "better-sqlite3",
    "cordova",
    "nativescript",
    "oracle",
    "mssql",
    "mongodb",
    "sqljs",
    "react-native",
    "expo",
    "capacitor",
    "spanner",
    "aurora-mysql",
    "aurora-postgres",
    "sap",
];

/// `'name':`/`"name":`/`name:` 形式のキー出現数。
fn count_key(line: &str, key: &str) -> usize {
    let mut n = 0usize;
    for m in line.match_indices(key) {
        let pos = m.0;
        let before_ok = pos == 0
            || line.as_bytes()[pos - 1] == b'\''
            || line.as_bytes()[pos - 1] == b'"'
            || line.as_bytes()[pos - 1] == b' '
            || line.as_bytes()[pos - 1] == b'\t'
            || line.as_bytes()[pos - 1] == b'{'
            || line.as_bytes()[pos - 1] == b',';
        if !before_ok {
            continue;
        }
        let after = pos + key.len();
        let rest = &line[after..];
        if rest.starts_with("':") || rest.starts_with("\":") || rest.starts_with(':') {
            n += 1;
        }
    }
    n
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `DataSource`/`module.exports`/`typeorm:`/`createConnection` 出現数。
    pub constructors: usize,
    /// `TYPEORM_*` 環境変数エントリ数。
    pub env_vars: usize,
    /// DB 接続キー数。
    pub db_keys: usize,
    /// ORM 振る舞いキー数。
    pub orm_keys: usize,
    /// パス/ディレクトリ系キー数。
    pub path_keys: usize,
    /// 既知 dialect 値出現数。
    pub dialect_values: usize,
}

/// `b` が TypeORM 設定形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        c.env_vars >= 3
            || (c.constructors >= 1 && c.db_keys >= 2 && c.orm_keys + c.path_keys >= 1)
            || (c.db_keys >= 2 && c.orm_keys >= 1 && c.path_keys >= 1)
            || (c.dialect_values >= 1 && c.db_keys + c.orm_keys + c.path_keys >= 4)
    })
}

/// `b` を TypeORM 設定として解析し、キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        constructors: 0,
        env_vars: 0,
        db_keys: 0,
        orm_keys: 0,
        path_keys: 0,
        dialect_values: 0,
    };
    let mut saw_any = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with("/*")
            || line.starts_with('#')
        {
            continue;
        }
        if line.starts_with("TYPEORM_") {
            c.env_vars += 1;
            saw_any = true;
            continue;
        }
        if line.contains("DataSource")
            || line.contains("module.exports")
            || line.contains("export default")
            || line.contains("createConnection")
        {
            c.constructors += 1;
            saw_any = true;
        }
        let n: usize = DB_KEYS.iter().map(|k| count_key(line, k)).sum();
        if n > 0 {
            c.db_keys += n;
            saw_any = true;
        }
        let n: usize = ORM_KEYS.iter().map(|k| count_key(line, k)).sum();
        if n > 0 {
            c.orm_keys += n;
            saw_any = true;
        }
        let n: usize = PATH_KEYS.iter().map(|k| count_key(line, k)).sum();
        if n > 0 {
            c.path_keys += n;
            saw_any = true;
        }
        for d in DIALECT_VALUES {
            let n = line.matches(&["\"", d, "\""].concat()).count()
                + line.matches(&["'", d, "'"].concat()).count();
            if n > 0 {
                c.dialect_values += n;
                saw_any = true;
            }
        }
    }
    if saw_any {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &[u8] = br#"{
  "type": "postgres",
  "host": "db.internal",
  "port": 5432,
  "username": "app",
  "password": "s3cret",
  "database": "appdb",
  "synchronize": false,
  "logging": ["error"],
  "entities": ["src/entity/**/*.ts"],
  "migrations": ["src/migration/**/*.ts"],
  "subscribers": ["src/subscriber/**/*.ts"],
  "migrationsTableName": "migrations",
  "cli": {
    "entitiesDir": "src/entity",
    "migrationsDir": "src/migration",
    "subscribersDir": "src/subscriber"
  }
}
"#;

    const SAMPLE_ENV: &[u8] = b"TYPEORM_CONNECTION = postgres\n\
        TYPEORM_HOST = db.internal\n\
        TYPEORM_PORT = 5432\n\
        TYPEORM_USERNAME = app\n\
        TYPEORM_DATABASE = appdb\n\
        TYPEORM_SYNCHRONIZE = false\n\
        TYPEORM_ENTITIES = src/entity/*.ts\n";

    #[test]
    fn detects_json() {
        assert!(detect(SAMPLE_JSON));
        let c = parse(SAMPLE_JSON).unwrap();
        assert!(c.db_keys >= 6);
        assert!(c.orm_keys >= 2);
        assert!(c.path_keys >= 7);
        assert_eq!(c.dialect_values, 1);
    }

    #[test]
    fn detects_env() {
        assert!(detect(SAMPLE_ENV));
        let c = parse(SAMPLE_ENV).unwrap();
        assert_eq!(c.env_vars, 7);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"foo: bar\n"));
        assert!(!detect(b"TYPEORM_HOST = x\n"));
    }
}
