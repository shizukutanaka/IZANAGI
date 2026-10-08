//! Zap (Go 構造化ロガー) 設定 `zap.config`/`zap.yaml`/`config.json` の解析。
//!
//! トップレベル既知キー `level`/`development`/`disableCaller`/`disableStacktrace`/
//! `sampling`/`encoding`/`encoderConfig`/`outputPaths`/`errorOutputPaths`/
//! `initialFields`/`epoch`/`DPanicKey` と、`encoderConfig` 配下の
//! `messageKey`/`levelKey`/`timeKey`/`nameKey`/`callerKey`/`functionKey`/
//! `stacktraceKey`/`skipLineEnding`/`lineEnding`/`levelEncoder`/`timeEncoder`/
//! `durationEncoder`/`callerEncoder`/`nameEncoder`/`consoleSeparator`、
//! `sampling` 配下の `initial`/`thereafter`/`samplingFactor`、
//! `encoding` 値 (`json`/`console`)、エンコーダ値 (`iso8601`/`rfc3339`/
//! `rfc3339nano`/`epoch`/`millis`/`nanos`/`capital`/`capitalColor`/`lowercase`/
//! `full`/`short`) 等を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::zapconf;
//!
//! let text = br#"{"level":"info","encoding":"json","outputPaths":["stdout"],"encoderConfig":{"messageKey":"msg","levelKey":"level","timeKey":"ts","levelEncoder":"lowercase","timeEncoder":"iso8601"}}"#;
//!
//! assert!(zapconf::detect(text));
//! let c = zapconf::parse(text).unwrap();
//! assert_eq!(c.encoder_keys, 5);
//! ```

/// トップレベル既知キー。
const TOP_KEYS: &[&str] = &[
    "level",
    "development",
    "disableCaller",
    "disableStacktrace",
    "sampling",
    "encoding",
    "encoderConfig",
    "outputPaths",
    "errorOutputPaths",
    "initialFields",
    "epoch",
    "DPanicKey",
];

/// `encoderConfig` 配下の既知キー。
const ENCODER_KEYS: &[&str] = &[
    "messageKey",
    "levelKey",
    "timeKey",
    "nameKey",
    "callerKey",
    "functionKey",
    "stacktraceKey",
    "skipLineEnding",
    "lineEnding",
    "levelEncoder",
    "timeEncoder",
    "durationEncoder",
    "callerEncoder",
    "nameEncoder",
    "consoleSeparator",
];

/// `sampling` 配下の既知キー。
const SAMPLING_KEYS: &[&str] = &["initial", "thereafter", "samplingFactor"];

/// 既知エンコーダ・エンコーディング値。
const KNOWN_VALUES: &[&str] = &[
    "json",
    "console",
    "iso8601",
    "rfc3339",
    "rfc3339nano",
    "epoch",
    "millis",
    "nanos",
    "capital",
    "capitalColor",
    "lowercase",
    "full",
    "short",
    "color",
    "syslog",
    "journald",
    "stderr",
    "stdout",
    "debug",
    "info",
    "warn",
    "error",
    "dpanic",
    "panic",
    "fatal",
];

/// Zap 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// トップレベル既知キー出現数。
    pub top_keys: usize,
    /// `encoderConfig` 配下の既知キー数。
    pub encoder_keys: usize,
    /// `sampling` 配下の既知キー数。
    pub sampling_keys: usize,
    /// 既知値 (`iso8601`/`lowercase`/`json` 等) 数。
    pub known_values: usize,
    /// `outputPaths`/`errorOutputPaths` パス配列要素推定数。
    pub output_paths: usize,
}

/// 行から `key` (JSON `"key":` / YAML `key:`) の宣言数を返す。
fn count_key(line: &str, key: &str) -> usize {
    let mut n = 0usize;
    for m in line.match_indices(key) {
        let pos = m.0;
        let before_ok = pos == 0
            || line.as_bytes()[pos - 1] == b' '
            || line.as_bytes()[pos - 1] == b'\t'
            || line.as_bytes()[pos - 1] == b'"'
            || line.as_bytes()[pos - 1] == b','
            || line.as_bytes()[pos - 1] == b'{';
        if !before_ok {
            continue;
        }
        let after = pos + key.len();
        let ok = match line.as_bytes().get(after) {
            Some(b'"') => line.as_bytes().get(after + 1) == Some(&b':'),
            Some(b':') => true,
            _ => false,
        };
        if ok {
            n += 1;
        }
    }
    n
}

/// `b` が zap 設定らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.top_keys >= 3 && c.encoder_keys + c.known_values >= 2
}

/// Zap 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        top_keys: 0,
        encoder_keys: 0,
        sampling_keys: 0,
        known_values: 0,
        output_paths: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        saw_any = true;
        for key in TOP_KEYS {
            counts.top_keys += count_key(line, key);
        }
        for key in ENCODER_KEYS {
            counts.encoder_keys += count_key(line, key);
        }
        for key in SAMPLING_KEYS {
            counts.sampling_keys += count_key(line, key);
        }
        for v in KNOWN_VALUES {
            let quoted = ["\"", v, "\""].concat();
            counts.known_values += line.matches(&quoted).count();
        }
        if line.contains("outputPaths") || line.contains("errorOutputPaths") {
            counts.output_paths += line.matches("stdout").count();
            counts.output_paths += line.matches("stderr").count();
            counts.output_paths += line.matches(".log").count();
            counts.output_paths += line.matches("file://").count();
            counts.output_paths += line.matches("http").count();
        }
    }
    if !saw_any {
        return None;
    }
    if !s.trim().is_empty()
        && counts.top_keys
            + counts.encoder_keys
            + counts.sampling_keys
            + counts.known_values
            + counts.output_paths
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
  "level": "info",
  "development": false,
  "disableCaller": false,
  "disableStacktrace": false,
  "sampling": {
    "initial": 100,
    "thereafter": 100
  },
  "encoding": "json",
  "encoderConfig": {
    "messageKey": "msg",
    "levelKey": "level",
    "timeKey": "ts",
    "nameKey": "logger",
    "callerKey": "caller",
    "stacktraceKey": "stacktrace",
    "lineEnding": "",
    "levelEncoder": "lowercase",
    "timeEncoder": "iso8601",
    "durationEncoder": "string",
    "callerEncoder": "short"
  },
  "outputPaths": ["stdout", "/var/log/app.log"],
  "errorOutputPaths": ["stderr"]
}
"#;

    #[test]
    fn detects_zap() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.encoder_keys, 11);
        assert_eq!(c.sampling_keys, 2);
        assert!(c.known_values >= 3);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"level: info"));
    }

    #[test]
    fn rejects_unrecognized_garbage() {
        assert!(parse(b"the quick brown fox jumps over the lazy dog\n").is_none());
        assert!(parse(b"hello world this is not a config file at all\n").is_none());
    }
}
