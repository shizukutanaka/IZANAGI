//! Serilog `appsettings.json` 内 `"Serilog"` セクション設定の解析。
//!
//! `"Serilog": {` ルート、 `"Using": ["Serilog.Sinks.*"]` シンクアセンブリ、
//! `"MinimumLevel": {"Default"|"Override"|"Microsoft*"|"System*"|"Else"`/`"Verbose"`
//! `|"Debug"|"Information"|"Warning"|"Error"|"Fatal"` レベル宣言、
//! `"WriteTo": [{"Name": "*", "Args": {...}}]` シンク宣言 (Console/File/RollingFile/
//! FileSizeLimit/Seq/Elasticsearch/ApplicationInsights/Email/EventLog/MSSqlServer/
//! Async/PeriodicBatching/...)、`"Enrich": ["FromLogContext"|...]`、`"Destructure"`/
//! `"Filter"`/`"Properties"`/`"AuditTo"`/`"LevelSwitch"`/`"restrictedToMinimumLevel"`/
//! `"theme"`/`"formatter"`/`"outputTemplate"`/`"path"`/`"rollingInterval"`/
//! `"retainedFileCountLimit"`/`"fileSizeLimitBytes"`/`"rollOnFileSizeLimit"`/
//! `"shared"`/`"period"`/`"buffered"`/`"formatter"` 等を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::serilog;
//!
//! let text = br#"{"Serilog":{"Using":["Serilog.Sinks.Console"],"MinimumLevel":{"Default":"Information"},"WriteTo":[{"Name":"Console"}]}}"#;
//!
//! assert!(serilog::detect(text));
//! let c = serilog::parse(text).unwrap();
//! assert_eq!(c.sinks, 1);
//! ```

/// Serilog セクション内の既知構造キー。
const STRUCT_KEYS: &[&str] = &[
    "Using",
    "MinimumLevel",
    "WriteTo",
    "AuditTo",
    "Enrich",
    "Destructure",
    "Filter",
    "Properties",
    "LevelSwitch",
    "Conditional",
];

/// Serilog レベル値。
const LEVELS: &[&str] = &[
    "Verbose",
    "Debug",
    "Information",
    "Warning",
    "Error",
    "Fatal",
];

/// `Args` 内の既知引数キー。
const ARG_KEYS: &[&str] = &[
    "path",
    "rollingInterval",
    "retainedFileCountLimit",
    "fileSizeLimitBytes",
    "rollOnFileSizeLimit",
    "shared",
    "period",
    "buffered",
    "formatter",
    "theme",
    "outputTemplate",
    "restrictedToMinimumLevel",
    "levelSwitch",
    "serverUrl",
    "apiKey",
    "connectionString",
    "tableName",
    "autoCreateSqlTable",
    "batchPostingLimit",
    "formatProvider",
    "renderTemplate",
    "standardErrorFromLevel",
    "internalEncodings",
    "memoryBuffer",
];

/// Serilog 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `"Serilog"` ルート宣言数。
    pub roots: usize,
    /// 構造キー宣言数 (Using/MinimumLevel/WriteTo/Enrich/…)。
    pub struct_keys: usize,
    /// `"Using"` 値数 (`Serilog.*` アセンブリ)。
    pub usings: usize,
    /// `WriteTo`/`AuditTo` シンクエントリ数 (`"Name": "` 宣言)。
    pub sinks: usize,
    /// `Enrich`/`Destructure`/`Filter` 配列要素数 (推定)。
    pub enrichers: usize,
    /// レベル値宣言数 (`"Verbose"`/`"Information"`/… + `Default`/`Override`/`Microsoft`/`System` スコープ)。
    pub level_decls: usize,
    /// `Args` 既知引数キー数。
    pub arg_keys: usize,
}

/// `line` 中の `"key"` (JSON キー、クォート付き) 出現数を返す。
fn count_qkey(line: &str, key: &str) -> usize {
    line.matches(&["\"", key, "\""].concat()).count()
}

/// `b` が Serilog 設定らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.roots >= 1 || (c.usings >= 1 && c.sinks >= 1)
}

/// Serilog 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        roots: 0,
        struct_keys: 0,
        usings: 0,
        sinks: 0,
        enrichers: 0,
        level_decls: 0,
        arg_keys: 0,
    };
    let mut saw_any = false;
    let mut in_write_or_audit = 0i32;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        saw_any = true;
        counts.roots += count_qkey(line, "Serilog");
        for key in STRUCT_KEYS {
            let n = count_qkey(line, key);
            counts.struct_keys += n;
            if n > 0 && (*key == "WriteTo" || *key == "AuditTo") {
                in_write_or_audit += 1;
            }
        }
        counts.usings += line.matches("Serilog.Sinks").count();
        counts.usings += line.matches("Serilog.Enrichers").count();
        counts.usings += line.matches("Serilog.Filters").count();
        counts.usings += line.matches("Serilog.Formatting").count();
        counts.usings += line.matches("Serilog.Settings").count();
        if in_write_or_audit > 0 {
            counts.sinks += count_qkey(line, "Name");
        }
        if in_write_or_audit > 0 {
            counts.arg_keys += count_qkey(line, "configure");
        }
        for key in ARG_KEYS {
            counts.arg_keys += count_qkey(line, key);
        }
        for lv in LEVELS {
            counts.level_decls += line.matches(&["\"", lv, "\""].concat()).count();
        }
        counts.level_decls += count_qkey(line, "Default");
        counts.level_decls += count_qkey(line, "Override");
        counts.level_decls += count_qkey(line, "Else");
        counts.level_decls += line.matches("Microsoft.").count();
        counts.level_decls += line.matches("System.").count();
        counts.enrichers += line.matches("FromLogContext").count();
        counts.enrichers += line.matches("WithMachineName").count();
        counts.enrichers += line.matches("WithThreadId").count();
        counts.enrichers += line.matches("WithProcessId").count();
        counts.enrichers += line.matches("WithEnvironmentUserName").count();
        counts.enrichers += line.matches("WithExceptionDetails").count();
        counts.enrichers += line.matches("ByExcluding").count();
        counts.enrichers += line.matches("Matching.").count();
    }
    if !saw_any {
        return None;
    }
    if !s.trim().is_empty()
        && counts.roots
            + counts.struct_keys
            + counts.usings
            + counts.sinks
            + counts.enrichers
            + counts.level_decls
            + counts.arg_keys
            == 0
    {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"{
  "Serilog": {
    "Using": [
      "Serilog.Sinks.Console",
      "Serilog.Sinks.File",
      "Serilog.Sinks.Seq"
    ],
    "MinimumLevel": {
      "Default": "Information",
      "Override": {
        "Microsoft": "Warning",
        "Microsoft.AspNetCore": "Warning",
        "System": "Error"
      }
    },
    "WriteTo": [
      {
        "Name": "Console",
        "Args": {
          "theme": "Serilog.Sinks.SystemConsole.Themes.AnsiConsoleTheme::Code, Serilog.Sinks.Console",
          "outputTemplate": "[{Timestamp:HH:mm:ss} {Level:u3}] {Message:lj}{NewLine}{Exception}"
        }
      },
      {
        "Name": "File",
        "Args": {
          "path": "logs/app.log",
          "rollingInterval": "Day",
          "retainedFileCountLimit": 14,
          "restrictedToMinimumLevel": "Information"
        }
      },
      { "Name": "Seq", "Args": { "serverUrl": "http://localhost:5341", "apiKey": "k" } }
    ],
    "Enrich": ["FromLogContext", "WithMachineName", "WithThreadId"],
    "Properties": {
      "Application": "Sample"
    }
  }
}
"#;

    #[test]
    fn detects_serilog() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.roots, 1);
        assert_eq!(c.usings, 5);
        assert_eq!(c.sinks, 3);
        assert!(c.arg_keys >= 6);
        assert!(c.enrichers >= 3);
        assert!(c.level_decls >= 6);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"{\"name\":\"value\"}"));
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
