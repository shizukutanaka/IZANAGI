//! Sequelize `.sequelizerc`/`sequelize.config.js`/config.json の検出・カウント。
//!
//! `.sequelizerc` は `'config'`/`'models-path'`/`'seeders-path'`/`'migrations-path'`
//! 等のパス解決エクスポート、config.js/json は環境名 (development/test/production)
//! × DB 接続キー + `dialect` の JSON/JS オブジェクト。
//!
//! ```
//! let cfg = br#"const path = require('path');\n\
//!         module.exports = {\n\
//!           'config': path.resolve('config', 'config.json'),\n\
//!           'models-path': path.resolve('models'),\n\
//!           'seeders-path': path.resolve('seeders'),\n\
//!           'migrations-path': path.resolve('migrations')\n\
//!         };\n"#;
//! assert!(izanagi_kit::sequelizerc::detect(cfg));
//! let c = izanagi_kit::sequelizerc::parse(cfg).unwrap();
//! assert_eq!(c.path_keys, 4);
//! ```

/// 環境セクション名。
const ENV_NAMES: &[&str] = &[
    "development",
    "test",
    "production",
    "staging",
    "local",
    "ci",
];

/// `.sequelizerc` パス解決キー。
const PATH_KEYS: &[&str] = &[
    "config",
    "models-path",
    "seeders-path",
    "migrations-path",
    "url",
    "path",
    "migration-storage-path",
];

/// DB 接続キー。
const DB_KEYS: &[&str] = &[
    "username",
    "password",
    "database",
    "host",
    "port",
    "dialect",
    "storage",
    "protocol",
    "dialectModule",
    "dialectModulePath",
    "socketPath",
    "use_env_variable",
    "url",
];

/// Sequelize オプションキー。
const OPTION_KEYS: &[&str] = &[
    "logging",
    "pool",
    "dialectOptions",
    "define",
    "timezone",
    "schema",
    "ssl",
    "native",
    "benchmark",
    "retry",
    "seederStorage",
    "seederStorageTableName",
    "seederStorageSchema",
    "migrationStorageTableName",
    "migrationStorageTableSchema",
    "migrationStoragePath",
    "omitNull",
    "keepDefaultTimezone",
    "query",
    "set",
    "operatorsAliases",
    "typeValidation",
    "logQueryParameters",
    "standardConformingStrings",
    "clientMinMessages",
    "replication",
    "isolationLevel",
    "minifyAliases",
    "hooks",
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
        let ok = rest.starts_with("':") || rest.starts_with("\":") || rest.starts_with(':');
        if ok {
            n += 1;
        }
    }
    n
}

/// 行に `'x':`/`"x":`/`x:` のいずれかが現れるか。
fn has_key(line: &str, keys: &[&str]) -> usize {
    keys.iter().map(|k| count_key(line, k)).sum()
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `require(`/`module.exports`/`export default` 出現数。
    pub constructors: usize,
    /// `development:`/`production:` 等環境セクションキー数。
    pub env_sections: usize,
    /// `.sequelizerc` パス解決キー数 (`'models-path'` 等)。
    pub path_keys: usize,
    /// DB 接続キー数。
    pub db_keys: usize,
    /// `dialect` 値別出現数 (mysql|postgres|sqlite|mariadb|mssql|db2|oracle|snowflake)。
    pub dialect_values: usize,
    /// Sequelize オプションキー数。
    pub option_keys: usize,
}

/// `b` が `.sequelizerc`/sequelize config 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        (c.constructors >= 1 && (c.path_keys >= 1 || (c.env_sections >= 1 && c.db_keys >= 2)))
            || (c.env_sections >= 1 && c.db_keys >= 3 && c.dialect_values >= 1)
    })
}

/// `b` を `.sequelizerc`/sequelize config として解析し、キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        constructors: 0,
        env_sections: 0,
        path_keys: 0,
        db_keys: 0,
        dialect_values: 0,
        option_keys: 0,
    };
    let mut saw_any = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
            continue;
        }
        if line.contains("require(") {
            c.constructors += 1;
            saw_any = true;
        }
        if line.contains("module.exports") || line.contains("export default") {
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
        let n = has_key(line, PATH_KEYS);
        if n > 0 {
            c.path_keys += n;
            saw_any = true;
        }
        let n = has_key(line, DB_KEYS);
        if n > 0 {
            c.db_keys += n;
            saw_any = true;
        }
        for d in [
            "mysql",
            "postgres",
            "sqlite",
            "mariadb",
            "mssql",
            "db2",
            "oracle",
            "snowflake",
        ] {
            c.dialect_values += line.matches(&["\"", d, "\""].concat()).count()
                + line.matches(&["'", d, "'"].concat()).count();
        }
        let n = has_key(line, OPTION_KEYS);
        if n > 0 {
            c.option_keys += n;
            saw_any = true;
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

    const SAMPLE_RC: &[u8] = br#"// .sequelizerc
const path = require('path');
module.exports = {
  'config': path.resolve('config', 'config.json'),
  'models-path': path.resolve('models'),
  'seeders-path': path.resolve('seeders'),
  'migrations-path': path.resolve('migrations')
};
"#;

    const SAMPLE_CFG: &[u8] = br#"{
  "development": {
    "username": "root",
    "password": null,
    "database": "app_dev",
    "host": "127.0.0.1",
    "dialect": "postgres",
    "logging": false,
    "pool": {"max": 5, "min": 0}
  },
  "production": {
    "use_env_variable": "DATABASE_URL",
    "dialect": "postgres",
    "dialectOptions": {"ssl": {"require": true}},
    "seederStorage": "sequelize"
  }
}
"#;

    #[test]
    fn detects_sequelizerc() {
        assert!(detect(SAMPLE_RC));
        let c = parse(SAMPLE_RC).unwrap();
        assert_eq!(c.constructors, 2);
        assert_eq!(c.path_keys, 4);
    }

    #[test]
    fn detects_sequelize_config() {
        assert!(detect(SAMPLE_CFG));
        let c = parse(SAMPLE_CFG).unwrap();
        assert_eq!(c.env_sections, 2);
        assert!(c.db_keys >= 5);
        assert!(c.dialect_values >= 2);
        assert!(c.option_keys >= 3);
    }

    #[test]
    fn rejects_other_js() {
        assert!(!detect(b"console.log(1);\n"));
    }
}
