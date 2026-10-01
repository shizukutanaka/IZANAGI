//! Apache Log4j 2 `log4j2.xml` 設定ファイルの解析。
//!
//! `<Configuration>` ルート要素と `<Appenders>`/`<Loggers>`/`<Properties>`/
//! `<CustomLevels>`/`<Filters>`/`packages`/`status`/`monitorInterval`/
//! `<Console`/`<File`/`<RollingFile`/`<RollingRandomAccessFile`/`<Async`/
//! `<SMTP`/`<JDBC`/`<Kafka`/`<Syslog`/`<Socket`/`<NoSQL`/`<Rewrite`/
//! `<Failover`/`<RandomAccessFile`/`<Logger`/`<Root`/`<AppenderRef`/
//! `<PatternLayout`/`<JSONLayout`/`<XmlLayout`/`<CsvLayout`/`<Level`/
//! `<ThresholdFilter`/`<RegexFilter`/`<TimeFilter`/`Pattern`/`FileName`/
//! `filePattern`/`Policies`/`SizeBasedTriggeringPolicy`/`<CronTriggeringPolicy`/
//! `<TimeBasedTriggeringPolicy`/`DefaultRolloverStrategy` 等要素・属性を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::log4j;
//!
//! let text = br#"<Configuration status="WARN"><Appenders><Console name="Std"><PatternLayout pattern="%m%n"/></Console></Appenders><Loggers><Root level="info"><AppenderRef ref="Std"/></Root></Loggers></Configuration>"#;
//!
//! assert!(log4j::detect(text));
//! let c = log4j::parse(text).unwrap();
//! assert_eq!(c.appenders, 1);
//! ```

/// Log4j2 XML 内のアペンダー型タグ。
const APPENDER_TAGS: &[&str] = &[
    "<Console ",
    "<Console>",
    "<File ",
    "<File>",
    "<RollingFile ",
    "<RollingFile>",
    "<RollingRandomAccessFile ",
    "<RandomAccessFile ",
    "<RandomAccessFile>",
    "<Async ",
    "<Async>",
    "<AsyncAppender",
    "<SMTP ",
    "<SMTP>",
    "<JDBC ",
    "<JDBC>",
    "<Kafka ",
    "<Kafka>",
    "<Syslog ",
    "<Syslog>",
    "<Socket ",
    "<Socket>",
    "<NoSQL ",
    "<NoSQL>",
    "<Rewrite ",
    "<Rewrite>",
    "<Failover ",
    "<Failover>",
    "<Routing ",
    "<Routing>",
    "<MemoryMappedFile ",
    "<ZeroMQ ",
    "<JeroMq ",
    "<FlumeAppender",
    "<Appender ",
    "<Appender>",
];

/// レイアウト系タグ。
const LAYOUT_TAGS: &[&str] = &[
    "<PatternLayout",
    "<JSONLayout",
    "<XmlLayout",
    "<CsvLayout",
    "<HtmlLayout",
    "<GelfLayout",
    "<SerializedLayout",
    "<MessageLayout",
    "<RFC5424Layout",
    "<SyslogLayout",
    "<YamlLayout",
    "<Pattern ",
    "<Pattern>",
];

/// フィルタ系タグ。
const FILTER_TAGS: &[&str] = &[
    "<ThresholdFilter",
    "<RegexFilter",
    "<TimeFilter",
    "<LevelRangeFilter",
    "<BurstFilter",
    "<DynamicThresholdFilter",
    "<MapFilter",
    "<MarkerFilter",
    "<StructuredDataFilter",
    "<ThreadContextMapFilter",
    "<NoMarkerFilter",
    "<CompositeFilter",
    "<ScriptFilter",
    "<filters>",
];

/// ロールオーバー・ポリシー系タグ。
const POLICY_TAGS: &[&str] = &[
    "<SizeBasedTriggeringPolicy",
    "<TimeBasedTriggeringPolicy",
    "<CronTriggeringPolicy",
    "<OnStartupTriggeringPolicy",
    "<DefaultRolloverStrategy",
    "<DirectWriteRolloverStrategy",
    "<Policies>",
];

/// Log4j2 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<Configuration` ルート宣言数。
    pub roots: usize,
    /// アペンダー宣言数。
    pub appenders: usize,
    /// `<Logger `/`<Logger>`/`<Root ` ロガー宣言数。
    pub loggers: usize,
    /// レイアウト宣言数。
    pub layouts: usize,
    /// フィルタ宣言数。
    pub filters: usize,
    /// ロールオーバー・ポリシー宣言数。
    pub policies: usize,
    /// `<AppenderRef`/`<FilterRef`/`ref=`/`appender-ref` 参照数。
    pub references: usize,
    /// `<Property`/`<property`/`name=`/`level=`/`status=`/`monitorInterval` 属性宣言数。
    pub attributes: usize,
}

/// `line` 中の `tag` 出現数を返す。
fn count_tag(line: &str, tag: &str) -> usize {
    line.matches(tag).count()
}

/// `b` が Log4j2 設定らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.roots >= 1 || (c.appenders >= 1 && c.loggers >= 1)
}

/// Log4j2 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = std::str::from_utf8(b).ok()?;
    let mut counts = Counts {
        roots: 0,
        appenders: 0,
        loggers: 0,
        layouts: 0,
        filters: 0,
        policies: 0,
        references: 0,
        attributes: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("<!--") {
            continue;
        }
        if line.contains("<!--") {
            let Some((head, _)) = line.split_once("<!--") else {
                continue;
            };
            if head.trim().is_empty() {
                continue;
            }
        }
        saw_any = true;
        counts.roots += count_tag(line, "<Configuration");
        for tag in APPENDER_TAGS {
            counts.appenders += count_tag(line, tag);
        }
        counts.loggers += count_tag(line, "<Logger ");
        counts.loggers += count_tag(line, "<Logger>");
        counts.loggers += count_tag(line, "<Root ");
        counts.loggers += count_tag(line, "<Root>");
        for tag in LAYOUT_TAGS {
            counts.layouts += count_tag(line, tag);
        }
        for tag in FILTER_TAGS {
            counts.filters += count_tag(line, tag);
        }
        for tag in POLICY_TAGS {
            counts.policies += count_tag(line, tag);
        }
        counts.references += count_tag(line, "<AppenderRef");
        counts.references += count_tag(line, "<appender-ref");
        counts.references += line.matches("ref=\"").count();
        counts.attributes += count_tag(line, "<Property ");
        counts.attributes += count_tag(line, "<Property>");
        counts.attributes += line.matches(" name=\"").count();
        counts.attributes += line.matches(" level=\"").count();
        counts.attributes += line.matches(" status=\"").count();
        counts.attributes += line.matches("monitorInterval").count();
        counts.attributes += count_tag(line, "<CustomLevel");
        counts.attributes += count_tag(line, "<KeyValuePair");
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Configuration status="WARN" monitorInterval="30" packages="com.example">
  <Appenders>
    <Console name="Console" target="SYSTEM_OUT">
      <PatternLayout pattern="%d %p %c{1.} [%t] %m%n"/>
      <ThresholdFilter level="debug" onMatch="ACCEPT"/>
    </Console>
    <RollingFile name="File" fileName="app.log" filePattern="app-%d{yyyy-MM-dd}.log">
      <PatternLayout pattern="%d %p %m%n"/>
      <Policies>
        <TimeBasedTriggeringPolicy interval="1"/>
        <SizeBasedTriggeringPolicy size="10 MB"/>
      </Policies>
      <DefaultRolloverStrategy max="10"/>
    </RollingFile>
    <Async name="Async">
      <AppenderRef ref="File"/>
    </Async>
  </Appenders>
  <Loggers>
    <Logger name="com.example" level="debug" additivity="false">
      <AppenderRef ref="Console"/>
    </Logger>
    <Root level="info">
      <AppenderRef ref="Async"/>
    </Root>
  </Loggers>
</Configuration>
"#;

    #[test]
    fn detects_log4j() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.roots, 1);
        assert_eq!(c.appenders, 3);
        assert_eq!(c.loggers, 2);
        assert_eq!(c.layouts, 2);
        assert_eq!(c.policies, 4);
        assert_eq!(c.references, 6);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"<html><body>x</body></html>"));
    }
}
