//! Liquibase `liquibase.properties`/`liquibase.checks-settings.conf` の検出・カウント。
//!
//! `key: value` / `key=value` フラットプロパティ形式。`liquibase.` 接頭辞あり/なしの
//! 両形を扱い、`liquibase.command.<cmd>.<opt>` はコマンド別オプションとして分類する。
//!
//! ```
//! let cfg = b"changeLogFile: db/changelog/db.changelog-master.xml\n\
//!             url: jdbc:postgresql://db:5432/app\n\
//!             username: liquibase\n\
//!             logLevel: info\n\
//!             hub.mode: all\n\
//!             parameter.tenant: core\n";
//! assert!(izanagi_kit::liquibase::detect(cfg));
//! let c = izanagi_kit::liquibase::parse(cfg).unwrap();
//! assert_eq!(c.entries, 6);
//! assert_eq!(c.database, 2);
//! assert_eq!(c.hub, 1);
//! ```

/// 変更ログ系キー。
const CHANGELOG_KEYS: &[&str] = &[
    "changeLogFile",
    "changelogFile",
    "changeSetDirectory",
    "contexts",
    "contextFilter",
    "labels",
    "labelFilter",
    "runOnChange",
    "includeAllFilesWithId",
    "includeMatchingTagInRollBackDate",
    "propertySubstitutionEnabled",
    "searchPath",
    "resourceLoader",
    "databaseChangelogTableName",
    "databaseChangelogLockTableName",
    "databaseChangeLogTableName",
    "databaseChangeLogLockTableName",
];

/// データベース接続系キー。
const DATABASE_KEYS: &[&str] = &[
    "url",
    "username",
    "password",
    "driver",
    "driverPropertiesFile",
    "defaultSchemaName",
    "defaultCatalogName",
    "liquibaseCatalogName",
    "liquibaseSchemaName",
    "referenceUrl",
    "referenceUsername",
    "referencePassword",
    "referenceDriver",
    "referenceDefaultSchemaName",
    "referenceDefaultCatalogName",
];

/// ログ・出力系キー。
const OUTPUT_KEYS: &[&str] = &[
    "logLevel",
    "logFile",
    "logChannels",
    "outputFile",
    "outputFileEncoding",
    "outputLineSeparator",
    "showBanner",
    "monitorPerformance",
    "verbose",
    "promptForNonLocalDatabase",
    "includeSystemClasspath",
    "strict",
    "autoInstallClj",
    "secureParsing",
    "liquibaseProLicenseKey",
    "proLicenseKey",
    "outputDefaultSchema",
    "outputDefaultCatalog",
    "classicFidelity",
    "defaultsFile",
    "classpath",
    "headless",
    "showHiddenArgs",
    "uiService",
];

/// Hub 系キー。
const HUB_KEYS: &[&str] = &[
    "hub.apiKey",
    "hubApiKey",
    "hubLogLevel",
    "hubConnectionId",
    "hubProjectId",
    "hubProjectName",
    "hubOrganizationId",
    "hubLabels",
    "hub.mode",
    "hubMode",
    "liquibaseHubApiKey",
    "liquibaseHubOrganizationId",
];

/// スナップショット・高度キー。
const ADVANCED_KEYS: &[&str] = &[
    "snapshotReferences",
    "generateSnapshotsOnUpdate",
    "snapshotFormat",
    "preserveSchemaCase",
    "outputObjectChangeFilter",
    "diffChangeFilter",
    "databaseClass",
    "propertyProviderClass",
    "parameter.dialect",
    "dataDir",
    "changeLogLockPollRate",
    "changeLogLockWaitTimeInMinutes",
    "changelogLockPollRate",
    "changelogLockWaitTimeInMinutes",
    "duplicateFileMode",
];

/// 行からプロパティキーを取り出す (`:`/`=` の先出側を区切りとする)。
fn prop_key(line: &str) -> Option<&str> {
    let eq = line.find('=');
    let co = line.find(':');
    let idx = match (eq, co) {
        (Some(e), Some(c)) => e.min(c),
        (Some(e), None) => e,
        (None, Some(c)) => c,
        _ => return None,
    };
    let key = line[..idx].trim();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// `liquibase.` 接頭辞を剥がす。
fn strip_prefix(key: &str) -> &str {
    key.strip_prefix("liquibase.").unwrap_or(key)
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// プロパティエントリ総数。
    pub entries: usize,
    /// 変更ログ系エントリ数。
    pub changelog: usize,
    /// DB 接続系エントリ数。
    pub database: usize,
    /// ログ・出力・雑多システム系エントリ数。
    pub output: usize,
    /// `hub.*`/`hubApiKey` 等 Hub 系エントリ数。
    pub hub: usize,
    /// `parameter.*`/`variable.*` 定義エントリ数。
    pub parameters: usize,
    /// `liquibase.command.<cmd>.*` コマンド別オプションエントリ数。
    pub command_opts: usize,
    /// スナップショット/ロック高度系エントリ数。
    pub advanced: usize,
    /// 既知以外のエントリ数。
    pub misc: usize,
}

/// `b` が `liquibase.properties` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を `liquibase.properties` として解析し、キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        entries: 0,
        changelog: 0,
        database: 0,
        output: 0,
        hub: 0,
        parameters: 0,
        command_opts: 0,
        advanced: 0,
        misc: 0,
    };
    let mut known = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some(key) = prop_key(line) else {
            continue;
        };
        let key = strip_prefix(key);
        c.entries += 1;
        if key.starts_with("command.") {
            c.command_opts += 1;
            known += 1;
        } else if key.starts_with("parameter.") || key.starts_with("variable.") {
            c.parameters += 1;
            known += 1;
        } else if HUB_KEYS.contains(&key) || key.starts_with("hub.") {
            c.hub += 1;
            known += 1;
        } else if CHANGELOG_KEYS.contains(&key) {
            c.changelog += 1;
            known += 1;
        } else if DATABASE_KEYS.contains(&key) {
            c.database += 1;
            known += 1;
        } else if OUTPUT_KEYS.contains(&key) {
            c.output += 1;
            known += 1;
        } else if ADVANCED_KEYS.contains(&key) {
            c.advanced += 1;
            known += 1;
        } else {
            c.misc += 1;
        }
    }
    if known >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# liquibase.properties\n\
        searchPath: /opt/liquibase/lib\n\
        changeLogFile: db/changelog/db.changelog-master.xml\n\
        url: jdbc:postgresql://db.internal:5432/app\n\
        username: liquibase\n\
        password: changeMe\n\
        driver: org.postgresql.Driver\n\
        defaultSchemaName: public\n\
        contexts: prod\n\
        labels: release-2024\n\
        logLevel: info\n\
        logFile: /var/log/liquibase.log\n\
        monitorPerformance: false\n\
        hub.mode: all\n\
        hub.apiKey: xxxx\n\
        parameter.tenant: core\n\
        liquibase.command.update.runOnChange: true\n";

    #[test]
    fn detects_liquibase() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 16);
        assert_eq!(c.changelog, 4);
        assert_eq!(c.database, 5);
        assert_eq!(c.output, 3);
        assert_eq!(c.hub, 2);
        assert_eq!(c.parameters, 1);
        assert_eq!(c.command_opts, 1);
    }

    #[test]
    fn rejects_other_props() {
        assert!(!detect(b"foo: bar\nbaz: qux\n"));
        assert!(!detect(b"key=value\n"));
    }
}
