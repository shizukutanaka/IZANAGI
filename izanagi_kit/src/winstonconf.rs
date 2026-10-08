//! Winston (Node.js ロガー) 設定の解析。
//!
//! `winston.createLogger({ ... })`/`new winston.Logger({ ... })`/
//! `transports: [ new winston.transports.Console(), new winston.transports.File({...}) ]`/
//! `winston.transports.*` (Console/File/Http/Stream/Couchdb/Mongodb/Redis/
//! Papertrail/Loggly/Logzio/Elasticsearch/DailyRotateFile/CloudWatch/Memory/
//! DistSyslog/Syslog/Sns/SqlServer/WebHook/Fluentd/UDP/Loki/Slack)、
//! `winston.format.*` (json/simple/combine/colorize/timestamp/label/printf/
//! errors/splat/metadata/prettyPrint/align/cli/logstash/ms/padLevels/
//! timestamp/uncolorize)、`winston.config.npm.levels`/`winston.config.syslog.levels`/
//! `winston.config.cli.levels`/`levels:`/`level:`/`silent:`/`format:`/`defaultMeta:`/
//! `exitOnError:`/`exceptionHandlers:`/`rejectionHandlers:`/`handleExceptions:`/
//! `handleRejections:`/`profilers:`/`transports.add`/`winston.add()`/
//! `winston.remove()`/`winston.configure()` 等を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::winstonconf;
//!
//! let text = b"winston.createLogger({\n  level: 'info',\n  format: winston.format.json(),\n  transports: [new winston.transports.Console(), new winston.transports.File({filename: 'app.log'})]\n});\n";
//!
//! assert!(winstonconf::detect(text));
//! let c = winstonconf::parse(text).unwrap();
//! assert_eq!(c.transports, 2);
//! ```

/// `winston.transports.*` の既知トランスポート名。
const TRANSPORT_NAMES: &[&str] = &[
    "Console",
    "File",
    "Http",
    "Stream",
    "Couchdb",
    "Mongodb",
    "Redis",
    "Papertrail",
    "Loggly",
    "Logzio",
    "Elasticsearch",
    "DailyRotateFile",
    "CloudWatch",
    "Memory",
    "DistSyslog",
    "Syslog",
    "Sns",
    "SqlServer",
    "WebHook",
    "Fluentd",
    "UDP",
    "Loki",
    "Slack",
    "Logstash",
];

/// Winston 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `winston.createLogger`/`new winston.Logger`/`winston.configure`/`require('winston')` 呼出数。
    pub constructors: usize,
    /// `winston.transports.*`/`new <Name>Transport(` トランスポート宣言数。
    pub transports: usize,
    /// `winston.format.*`/`format.*(` フォーマット呼出数。
    pub formats: usize,
    /// `levels`/`level`/`silent`/`silent`/`exitOnError`/`handleExceptions`/`exceptionHandlers`/
    /// `rejectionHandlers`/`profilers`/`defaultMeta` オプション宣言数。
    pub options: usize,
    /// `winston.add`/`winston.remove`/`logger.add`/`logger.remove`/`add(`/`remove(` 操作呼出数。
    pub add_remove: usize,
}

/// `b` が winston 設定らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.constructors >= 1 || (c.transports >= 1 && c.formats + c.options >= 1)
}

/// Winston 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        constructors: 0,
        transports: 0,
        formats: 0,
        options: 0,
        add_remove: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        saw_any = true;
        counts.constructors += line.matches("createLogger").count();
        counts.constructors += line.matches("winston.Logger").count();
        counts.constructors += line.matches("winston.configure").count();
        counts.constructors += line.matches("require('winston')").count();
        counts.constructors += line.matches("require(\"winston\")").count();
        counts.transports += line.matches("winston.transports.").count();
        for t in TRANSPORT_NAMES {
            counts.transports += line.matches(&["new ", t, "Transport"].concat()).count();
        }
        counts.formats += line.matches("format.").count();
        counts.options += count_word(line, "level");
        counts.options += count_word(line, "levels");
        counts.options += count_word(line, "silent");
        counts.options += count_word(line, "exitOnError");
        counts.options += count_word(line, "handleExceptions");
        counts.options += count_word(line, "exceptionHandlers");
        counts.options += count_word(line, "handleRejections");
        counts.options += count_word(line, "rejectionHandlers");
        counts.options += count_word(line, "profilers");
        counts.options += count_word(line, "defaultMeta");
        counts.options += line.matches("npm.levels").count();
        counts.options += line.matches("syslog.levels").count();
        counts.options += line.matches("cli.levels").count();
        counts.add_remove += line.matches("winston.add").count();
        counts.add_remove += line.matches("winston.remove").count();
        counts.add_remove += line.matches("logger.add").count();
        counts.add_remove += line.matches("logger.remove").count();
    }
    if !saw_any {
        return None;
    }
    if !s.trim().is_empty()
        && counts.constructors
            + counts.transports
            + counts.formats
            + counts.options
            + counts.add_remove
            == 0
    {
        return None;
    }
    Some(counts)
}

/// `word:` (JS オブジェクトのプロパティ) 宣言数を返す。
fn count_word(line: &str, word: &str) -> usize {
    let mut n = 0usize;
    for m in line.match_indices(&[word, ":"].concat()) {
        let pos = m.0;
        let before_ok = pos == 0
            || !line.as_bytes()[pos - 1].is_ascii_alphanumeric()
                && line.as_bytes()[pos - 1] != b'_'
                && line.as_bytes()[pos - 1] != b'.';
        if before_ok {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"const winston = require('winston');
const logger = winston.createLogger({
  level: 'info',
  levels: winston.config.npm.levels,
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.errors({ stack: true }),
    winston.format.json()
  ),
  defaultMeta: { service: 'user-service' },
  transports: [
    new winston.transports.Console({ level: 'debug', handleExceptions: true }),
    new winston.transports.File({ filename: 'error.log', level: 'error' }),
    new winston.transports.DailyRotateFile({ filename: 'app-%DATE%.log' }),
    new winston.transports.Http({ host: 'logs.local', port: 8080 })
  ],
  exceptionHandlers: [new winston.transports.File({ filename: 'exceptions.log' })],
  exitOnError: false
});
winston.add(logger);
"#;

    #[test]
    fn detects_winston() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.constructors, 2);
        assert_eq!(c.transports, 5);
        assert_eq!(c.formats, 4);
        assert!(c.options >= 5);
        assert_eq!(c.add_remove, 1);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"const x = 1;"));
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
