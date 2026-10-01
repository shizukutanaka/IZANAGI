//! Alembic `alembic.ini` の検出・カウント。
//!
//! `[alembic]` セクション (`script_location`/`sqlalchemy.url`/`version_locations` 等) と、
//! logging.configparser 連携セクション (`[loggers]`/`[handlers]`/`[formatters]`/
//! `[logger_*]`/`[handler_*]`/`[formatter_*]`) からなる INI ファイル。
//!
//! ```
//! let cfg = b"[alembic]\n\
//!             script_location = migrations\n\
//!             sqlalchemy.url = postgresql:///app\n\
//!             version_locations = %(here)s/versions\n\
//!             \n\
//!             [loggers]\n\
//!             keys = root,sqlalchemy,alembic\n";
//! assert!(izanagi_kit::alembic::detect(cfg));
//! let c = izanagi_kit::alembic::parse(cfg).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.script_keys, 2);
//! assert_eq!(c.migration_keys, 1);
//! ```

/// `[alembic]` セクションのスクリプト/レイアウト系キー。
const SCRIPT_KEYS: &[&str] = &[
    "script_location",
    "prepend_sys_path",
    "path_separator",
    "file_template",
    "date_format",
    "templates",
    "version_locations",
    "recursive_version_locations",
    "sourceless",
    "template_args",
    "version_table",
    "version_table_schema",
    "version_table_pk",
];

/// `[alembic]` セクションのマイグレーション実行系キー。
const MIGRATION_KEYS: &[&str] = &[
    "sqlalchemy.url",
    "transaction_per_migration",
    "transaction_dialect_name",
    "output_buffer",
    "output_encoding",
    "compare_type",
    "compare_server_default",
    "render_as_batch",
    "revision_environment",
    "timezone",
    "truncate_slug_length",
    "black_out",
    "dialect_name",
    "version_path_separator",
    "literal_binds",
    "include_schemas",
    "include_name",
    "include_object",
    "process_revision_directives",
    "render_item",
];

/// logging.configparser の既知セクション名接頭辞。
const LOGGER_SECTION_PREFIXES: &[&str] = &["logger_", "handler_", "formatter_"];

/// logging.configparser の固定セクション名。
const LOGGER_PLAIN_SECTIONS: &[&str] = &["loggers", "handlers", "formatters"];

/// セクション名を取り出す (`[name]` 行のみ)。
fn section_name(line: &str) -> Option<&str> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.trim())
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// セクション総数。
    pub sections: usize,
    /// `[alembic]` セクション数 (0/1)。
    pub alembic_sections: usize,
    /// logging 関連セクション数 (`[loggers]`/`[handler_*]` 等)。
    pub logging_sections: usize,
    /// その他セクション数。
    pub other_sections: usize,
    /// `key = value` エントリ総数。
    pub entries: usize,
    /// `[alembic]` 内スクリプト/レイアウト系キー数。
    pub script_keys: usize,
    /// `[alembic]` 内マイグレーション実行系キー数。
    pub migration_keys: usize,
    /// `[alembic]` 内その他キー数。
    pub other_alembic_keys: usize,
    /// logging セクション内エントリ数。
    pub logging_entries: usize,
    /// その他セクション内エントリ数。
    pub other_entries: usize,
}

/// `b` が `alembic.ini` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| {
        c.alembic_sections >= 1 && c.script_keys + c.migration_keys + c.other_alembic_keys >= 1
    })
}

/// `b` を `alembic.ini` として解析し、セクション・キー種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        alembic_sections: 0,
        logging_sections: 0,
        other_sections: 0,
        entries: 0,
        script_keys: 0,
        migration_keys: 0,
        other_alembic_keys: 0,
        logging_entries: 0,
        other_entries: 0,
    };
    #[derive(Clone, Copy, PartialEq)]
    enum Sec {
        Alembic,
        Logging,
        Other,
    }
    let mut cur = Sec::Other;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(name) = section_name(line) {
            c.sections += 1;
            if name == concat!("alembi", 'c') {
                c.alembic_sections += 1;
                cur = Sec::Alembic;
            } else if LOGGER_PLAIN_SECTIONS.contains(&name)
                || LOGGER_SECTION_PREFIXES.iter().any(|p| name.starts_with(p))
            {
                c.logging_sections += 1;
                cur = Sec::Logging;
            } else {
                c.other_sections += 1;
                cur = Sec::Other;
            }
            continue;
        }
        let eq = line.find('=');
        let co = line.find(':');
        let idx = match (eq, co) {
            (Some(e), Some(c2)) => e.min(c2),
            (Some(e), None) => e,
            (None, Some(c2)) => c2,
            _ => continue,
        };
        let key = line[..idx].trim();
        if key.is_empty() {
            continue;
        }
        c.entries += 1;
        match cur {
            Sec::Alembic => {
                if SCRIPT_KEYS.contains(&key) {
                    c.script_keys += 1;
                } else if MIGRATION_KEYS.contains(&key) {
                    c.migration_keys += 1;
                } else {
                    c.other_alembic_keys += 1;
                }
            }
            Sec::Logging => c.logging_entries += 1,
            Sec::Other => c.other_entries += 1,
        }
    }
    if c.alembic_sections >= 1 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"# Alembic Config\n\
        [alembic]\n\
        script_location = %(here)s/migrations\n\
        prepend_sys_path = .\n\
        version_path_separator = os\n\
        sqlalchemy.url = driver://user:pass@localhost/dbname\n\
        revision_environment = false\n\
        file_template = %%(rev)s_%%(slug)s\n\
        truncate_slug_length = 40\n\
        version_locations = %(here)s/versions\n\
        \n\
        [loggers]\n\
        keys = root,sqlalchemy,alembic,flask_migrate\n\
        \n\
        [handlers]\n\
        keys = console\n\
        \n\
        [formatters]\n\
        keys = generic\n\
        \n\
        [logger_root]\n\
        level = WARN\n\
        handlers = console\n\
        qualname =\n\
        \n\
        [handler_console]\n\
        class = StreamHandler\n\
        args = (sys.stderr,)\n\
        level = NOTSET\n\
        formatter = generic\n\
        \n\
        [formatter_generic]\n\
        format = %%(levelname)-5.5s [%%(name)s] %%(message)s\n\
        datefmt = %%H:%%M:%%S\n";

    #[test]
    fn detects_alembic() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 7);
        assert_eq!(c.alembic_sections, 1);
        assert_eq!(c.logging_sections, 6);
        assert_eq!(c.script_keys, 4);
        assert_eq!(c.migration_keys, 4);
        assert_eq!(c.entries, 20);
        assert_eq!(c.logging_entries, 12);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[section]\nkey = value\n"));
        assert!(!detect(b"[alembic]\n"));
        assert!(detect(b"[alembic]\nscript_location = m\n"));
    }
}
