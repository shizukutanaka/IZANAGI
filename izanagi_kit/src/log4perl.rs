//! Log::Log4perl `log4perl.conf` 設定ファイルの解析。
//!
//! `log4perl.category.<名> = <LEVEL>, <アペンダー…>`、`log4perl.logger.<名>`/
//! `log4perl.additivity.<名>`/`log4perl.appender.<名> = Log::Log4perl::Appender::*`/
//! `log4perl.appender.<名>.<param>`/`log4perl.appender.<名>.layout = Log::Log4perl::Layout::*`/
//! `log4perl.appender.<名>.layout.<param>`/`log4perl.filter.<名>`/
//! `log4perl.rootLogger = <LEVEL>, <アペンダー…>`/`log4perl.threshold`/
//! `log4perl.oneMessagePerAppender`/`log4perl.PatternLayout.cspec.*`/
//! `log4perl.wrapper.register`/`log4j.category`/`log4j.logger`/`log4j.appender`/
//! `log4j.rootLogger` (log4j 互換プレフィックス) 等を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::log4perl;
//!
//! let text = b"log4perl.rootLogger=INFO, Screen\nlog4perl.appender.Screen=Log::Log4perl::Appender::Screen\nlog4perl.appender.Screen.layout=Log::Log4perl::Layout::PatternLayout\nlog4perl.appender.Screen.layout.ConversionPattern=%d %m%n\n";
//!
//! assert!(log4perl::detect(text));
//! let c = log4perl::parse(text).unwrap();
//! assert_eq!(c.appenders, 1);
//! ```

/// `log4perl.*`/`log4j.*` プレフィックスを除いた既知第2キー。
const PARAM_TAIL_KEYS: &[&str] = &[
    "threshold",
    "oneMessagePerAppender",
    "factory",
    "utf8",
    "PatternLayout.cspec",
    "wrapper.register",
];

/// Log4perl 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `log4perl.*`/`log4j.*` 設定行数。
    pub entries: usize,
    /// `category.`/`logger.` カテゴリ宣言数。
    pub categories: usize,
    /// `appender.<名> = Log::Log4perl::Appender::*` アペンダー宣言数。
    pub appenders: usize,
    /// `.layout = Log::Log4perl::Layout::*` レイアウト宣言数。
    pub layouts: usize,
    /// `filter.<名>`/`Filter` フィルタ宣言数。
    pub filters: usize,
    /// `rootLogger`/`rootCategory` ルートロガー宣言数。
    pub roots: usize,
    /// `additivity.`/`threshold`/`oneMessagePerAppender`/`cspec`/`utf8` 等その他宣言数。
    pub misc: usize,
}

/// `b` が log4perl.conf らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.entries >= 2 && c.appenders + c.categories + c.roots >= 1
}

/// `key` が `log4perl.`/`log4j.` プレフィックスを持つ場合残りを返す。
fn tail(key: &str) -> Option<&str> {
    if let Some(t) = key.strip_prefix("log4perl.") {
        Some(t)
    } else {
        key.strip_prefix("log4j.")
    }
}

/// Log4perl 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        entries: 0,
        categories: 0,
        appenders: 0,
        layouts: 0,
        filters: 0,
        roots: 0,
        misc: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let Some(t) = tail(key) else {
            continue;
        };
        saw_any = true;
        counts.entries += 1;
        let mut t_parts = t.splitn(2, '.');
        let head = t_parts.next().unwrap_or("");
        match head {
            "category" | "logger" => {
                if t == "rootLogger" || t == "rootCategory" {
                    counts.roots += 1;
                } else {
                    counts.categories += 1;
                }
            }
            "rootLogger" | "rootCategory" => counts.roots += 1,
            "additivity" | "appender" => {
                if head == "additivity" {
                    counts.misc += 1;
                    continue;
                }
                // appender.<名>[.<param>[.<param>…]]
                let rest = t_parts.next().unwrap_or("");
                let mut rp = rest.split('.');
                let _name = rp.next().unwrap_or("");
                match (rp.next(), rp.next()) {
                    (None, _) => {
                        if value.contains("Log::Log4perl::Appender")
                            || value.contains("Log::Dispatch")
                            || value.contains("Log::Log4perl::JavaMap")
                        {
                            counts.appenders += 1;
                        } else {
                            counts.misc += 1;
                        }
                    }
                    (Some("layout"), None) => {
                        if value.contains("Layout") {
                            counts.layouts += 1;
                        } else {
                            counts.misc += 1;
                        }
                    }
                    (Some("layout"), Some(_)) => counts.misc += 1,
                    (Some("filter"), _) => counts.filters += 1,
                    (Some(_), _) => counts.misc += 1,
                }
            }
            "filter" => counts.filters += 1,
            "PatternLayout" | "level" | "java_class" => counts.misc += 1,
            _ => {
                if PARAM_TAIL_KEYS.iter().any(|k| t.starts_with(k)) || t.contains('.') {
                    counts.misc += 1;
                }
            }
        }
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"# log4perl.conf
log4perl.rootLogger=INFO, Screen, Logfile
log4perl.category.Foo.Bar=DEBUG, Logfile
log4perl.additivity.Foo.Bar=0

log4perl.appender.Screen=Log::Log4perl::Appender::Screen
log4perl.appender.Screen.stderr=0
log4perl.appender.Screen.layout=Log::Log4perl::Layout::PatternLayout
log4perl.appender.Screen.layout.ConversionPattern=%d %p %m%n

log4perl.appender.Logfile=Log::Log4perl::Appender::File
log4perl.appender.Logfile.filename=/var/log/app.log
log4perl.appender.Logfile.mode=append
log4perl.appender.Logfile.layout=Log::Log4perl::Layout::SimpleLayout
log4perl.appender.Logfile.filter.1=Log::Log4perl::Filter::LevelMatch
log4perl.threshold=WARN
"#;

    #[test]
    fn detects_log4perl() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 13);
        assert_eq!(c.categories, 1);
        assert_eq!(c.roots, 1);
        assert_eq!(c.appenders, 2);
        assert_eq!(c.layouts, 2);
        assert_eq!(c.filters, 1);
        assert_eq!(c.misc, 6);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"key=value\nother=1"));
    }
}
