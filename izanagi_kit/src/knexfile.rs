//! Knex `knexfile.js`/`knexfile.ts`/`knex.js` の検出・カウント。
//!
//! `module.exports = { <env>: { client: '<dialect>', connection: {…},
//! pool: {min,max}, migrations: {tableName,directory}, seeds: {…},
//! useNullAsDefault, searchPath, acquireConnectionTimeout } }` 形式。
//!
//! ```
//! let cfg = br#"module.exports = {\n\
//!         development: { client: 'sqlite3', connection: { filename: './dev.db' },\n\
//!         useNullAsDefault: true, migrations: { tableName: 'knex_migrations' } }\n\
//!         };\n"#;
//! assert!(izanagi_kit::knexfile::detect(cfg));
//! let c = izanagi_kit::knexfile::parse(cfg).unwrap();
//! assert_eq!(c.env_sections, 1);
//! assert!(c.knex_keys >= 1);
//! ```

/// 環境セクション名。
const ENV_NAMES: &[&str] = &[
    "development",
    "test",
    "production",
    "staging",
    "local",
    "ci",
    "testing",
];

/// Knex 接続系キー。
const CONNECTION_KEYS: &[&str] = &[
    "filename",
    "host",
    "port",
    "user",
    "password",
    "database",
    "charset",
    "ssl",
    "connectionString",
    "instanceName",
    "domain",
    "appName",
    "connectionStringParams",
    "connection",
];

/// pool 系キー。
const POOL_KEYS: &[&str] = &[
    "min",
    "max",
    "acquireTimeoutMillis",
    "createTimeoutMillis",
    "destroyTimeoutMillis",
    "idleTimeoutMillis",
    "reapIntervalMillis",
    "createRetryIntervalMillis",
    "propagateCreateError",
    "afterCreate",
    "beforeDestroy",
    "pool",
];

/// マイグレーション/シード系キー。
const MIGRATION_KEYS: &[&str] = &[
    "migrations",
    "seeds",
    "tableName",
    "schemaName",
    "directory",
    "extension",
    "stub",
    "loadExtensions",
    "disableTransactions",
    "disableMigrationsListValidation",
    "validateChecksums",
    "sortDirsSeparately",
];

/// Knex 固有情報キー (sequelize 系との区別に使う)。
const KNEX_KEYS: &[&str] = &[
    "client",
    "useNullAsDefault",
    "searchPath",
    "acquireConnectionTimeout",
    "wrapIdentifier",
    "postProcessResponse",
    "asyncStackTraces",
    "compileSqlOnError",
    "queryBuilder",
    "raw",
    "debug",
    "log",
    "knex",
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
    /// `module.exports`/`export default`/`require('knex')`/`knex(` 出現数。
    pub constructors: usize,
    /// 環境セクションキー数。
    pub env_sections: usize,
    /// `client:` 宣言数。
    pub clients: usize,
    /// 接続系キー数。
    pub connection_keys: usize,
    /// pool 系キー数。
    pub pool_keys: usize,
    /// マイグレーション/シード系キー数。
    pub migration_keys: usize,
    /// Knex 固有情報キー数 (`useNullAsDefault`/`searchPath`/`acquireConnectionTimeout` 等、`client`/`debug`/`log`/`knex` 除く)。
    pub knex_keys: usize,
}

/// `b` が `knexfile.js` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        c.clients >= 1 && (c.knex_keys >= 1 || (c.connection_keys >= 1 && c.migration_keys >= 1))
            || (c.env_sections >= 1 && c.clients >= 1 && c.connection_keys + c.migration_keys >= 2)
    })
}

/// `b` を `knexfile.js` として解析し、キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        constructors: 0,
        env_sections: 0,
        clients: 0,
        connection_keys: 0,
        pool_keys: 0,
        migration_keys: 0,
        knex_keys: 0,
    };
    let mut saw_any = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
            continue;
        }
        if line.contains("module.exports") || line.contains("export default") {
            c.constructors += 1;
            saw_any = true;
        }
        if line.contains("require('knex')")
            || line.contains("require(\"knex\")")
            || line.contains("knex(")
        {
            c.constructors += 1;
            saw_any = true;
        }
        for e in ENV_NAMES {
            let n = count_key(line, e);
            if n > 0 {
                c.env_sections += n;
                saw_any = true;
            }
        }
        let n = count_key(line, "client");
        if n > 0 {
            c.clients += n;
            saw_any = true;
        }
        let n: usize = CONNECTION_KEYS.iter().map(|k| count_key(line, k)).sum();
        if n > 0 {
            c.connection_keys += n;
            saw_any = true;
        }
        let n: usize = POOL_KEYS.iter().map(|k| count_key(line, k)).sum();
        if n > 0 {
            c.pool_keys += n;
            saw_any = true;
        }
        let n: usize = MIGRATION_KEYS.iter().map(|k| count_key(line, k)).sum();
        if n > 0 {
            c.migration_keys += n;
            saw_any = true;
        }
        for k in KNEX_KEYS {
            if *k == "client" || *k == "debug" || *k == "log" || *k == "knex" {
                continue;
            }
            let n = count_key(line, k);
            if n > 0 {
                c.knex_keys += n;
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

    const SAMPLE: &[u8] = br#"// knexfile.js
module.exports = {
  development: {
    client: 'sqlite3',
    connection: {
      filename: './dev.sqlite3'
    },
    useNullAsDefault: true,
    pool: {
      min: 2,
      max: 10
    },
    migrations: {
      tableName: 'knex_migrations',
      directory: './migrations'
    },
    seeds: {
      directory: './seeds'
    }
  },
  production: {
    client: 'postgresql',
    connection: process.env.DATABASE_URL,
    searchPath: ['knex', 'public'],
    acquireConnectionTimeout: 60000,
    pool: { min: 2, max: 30 },
    migrations: { tableName: 'knex_migrations' }
  }
};
"#;

    #[test]
    fn detects_knexfile() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.constructors, 1);
        assert_eq!(c.env_sections, 2);
        assert_eq!(c.clients, 2);
        assert!(c.connection_keys >= 2);
        assert!(c.pool_keys >= 5);
        assert!(c.migration_keys >= 4);
        assert!(c.knex_keys >= 3);
    }

    #[test]
    fn rejects_other_js() {
        assert!(!detect(b"console.log(1);\n"));
        assert!(!detect(b"module.exports = { foo: 1 };\n"));
    }
}
