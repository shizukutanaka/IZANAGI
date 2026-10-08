//! Logback (SLF4J バックエンド) `logback.xml`/`logback-spring.xml` の解析。
//!
//! `<configuration>` ルート、`<appender class="ch.qos.logback.*">` (ConsoleAppender/
//! FileAppender/RollingFileAppender/AsyncAppender/SiftingAppender/SMTPAppender/
//! DBAppender/SyslogAppender/SocketAppender/GelfAppender)、`<encoder>`/`<pattern>`/
//! `<rollingPolicy class="*TimeBasedRollingPolicy|*SizeAndTimeBasedRollingPolicy|
//! *FixedWindowRollingPolicy">`/`<fileNamePattern>`/`<triggeringPolicy>`/
//! `<maxFileSize>`/`<maxHistory>`/`<totalSizeCap>`/`<cleanHistoryOnStart>`/
//! `<logger name level additivity>`/`<root level>`/`<appender-ref ref>`/
//! `<property name value>`/`<include resource>`/`<jmxConfigurator>`/
//! `<turboFilter>`/`<statusListener>`/`scan`/`scanPeriod`/`packagingData`/
//! `<shutdownHook>`/`<contextName>` 等を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::logback;
//!
//! let text = br#"<configuration><appender name="STDOUT" class="ch.qos.logback.core.ConsoleAppender"><encoder><pattern>%d %msg%n</pattern></encoder></appender><root level="info"><appender-ref ref="STDOUT"/></root></configuration>"#;
//!
//! assert!(logback::detect(text));
//! let c = logback::parse(text).unwrap();
//! assert_eq!(c.appenders, 1);
//! ```

/// 計数対象の Logback 要素タグ (出現位置・構造判定用)。
const STRUCTURE_TAGS: &[&str] = &[
    "<contextName",
    "<jmxConfigurator",
    "<statusListener",
    "<shutdownHook",
    "<turboFilter",
    "<if ",
    "<if>",
    "<then",
    "<else",
    "<springProfile",
    "<springProperty",
];

/// ローリング・履歴系タグ。
const ROLLING_TAGS: &[&str] = &[
    "<rollingPolicy",
    "<triggeringPolicy",
    "<fileNamePattern",
    "<maxFileSize",
    "<maxHistory",
    "<totalSizeCap",
    "<cleanHistoryOnStart",
    "<minIndex",
    "<maxIndex",
    "<timeBasedFileNamingAndTriggeringPolicy",
];

/// Logback 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<configuration` ルート宣言数。
    pub roots: usize,
    /// `<appender` 宣言数。
    pub appenders: usize,
    /// `class="ch.qos.logback.*"` 型指定数。
    pub logback_classes: usize,
    /// `<logger`/`<root` ロガー宣言数。
    pub loggers: usize,
    /// `<encoder>`/`<pattern>`/`<charset>` エンコーダ宣言数。
    pub encoders: usize,
    /// ローリング・履歴要素数。
    pub rolling: usize,
    /// `<appender-ref`/`<appenderRef`/`ref="` 参照数。
    pub references: usize,
    /// `<property`/`<variable`/`<define`/`<include`/`<substitutionProperty` 等構造要素数。
    pub properties: usize,
}

/// `line` 中の `tag` 出現数を返す。
fn count_tag(line: &str, tag: &str) -> usize {
    line.matches(tag).count()
}

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `b` が Logback 設定らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.logback_classes >= 1 || (c.roots >= 1 && c.appenders >= 1)
}

/// Logback 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = strip_comments(std::str::from_utf8(b).ok()?);
    let mut counts = Counts {
        roots: 0,
        appenders: 0,
        logback_classes: 0,
        loggers: 0,
        encoders: 0,
        rolling: 0,
        references: 0,
        properties: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("<!--") {
            continue;
        }
        saw_any = true;
        counts.roots += count_tag(line, "<configuration");
        counts.appenders += count_tag(line, "<appender ");
        counts.appenders += count_tag(line, "<appender>");
        counts.logback_classes += line.matches("ch.qos.logback").count();
        counts.loggers += count_tag(line, "<logger ");
        counts.loggers += count_tag(line, "<logger>");
        counts.loggers += count_tag(line, "<root ");
        counts.loggers += count_tag(line, "<root>");
        counts.encoders += count_tag(line, "<encoder");
        counts.encoders += count_tag(line, "<pattern>");
        counts.encoders += count_tag(line, "<charset");
        counts.encoders += count_tag(line, "<immediateFlush");
        for tag in ROLLING_TAGS {
            counts.rolling += count_tag(line, tag);
        }
        counts.references += count_tag(line, "<appender-ref");
        counts.references += count_tag(line, "<appenderRef");
        counts.references += line.matches(" ref=\"").count();
        counts.properties += count_tag(line, "<property ");
        counts.properties += count_tag(line, "<property>");
        counts.properties += count_tag(line, "<variable");
        counts.properties += count_tag(line, "<define");
        counts.properties += count_tag(line, "<include");
        counts.properties += count_tag(line, "<substitutionProperty");
        for tag in STRUCTURE_TAGS {
            counts.properties += count_tag(line, tag);
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

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<configuration scan="true" scanPeriod="30 seconds" packagingData="true">
  <property name="LOG_DIR" value="/var/log/app"/>
  <contextName>myapp</contextName>
  <appender name="STDOUT" class="ch.qos.logback.core.ConsoleAppender">
    <encoder>
      <pattern>%d{HH:mm:ss.SSS} %-5level %logger{36} - %msg%n</pattern>
    </encoder>
  </appender>
  <appender name="FILE" class="ch.qos.logback.core.rolling.RollingFileAppender">
    <file>${LOG_DIR}/app.log</file>
    <rollingPolicy class="ch.qos.logback.core.rolling.TimeBasedRollingPolicy">
      <fileNamePattern>app-%d{yyyy-MM-dd}.log</fileNamePattern>
      <maxHistory>30</maxHistory>
      <totalSizeCap>1GB</totalSizeCap>
    </rollingPolicy>
    <encoder>
      <pattern>%d %msg%n</pattern>
    </encoder>
  </appender>
  <logger name="com.example" level="DEBUG" additivity="false">
    <appender-ref ref="STDOUT"/>
  </logger>
  <root level="INFO">
    <appender-ref ref="FILE"/>
  </root>
</configuration>
"#;

    #[test]
    fn detects_logback() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.roots, 1);
        assert_eq!(c.appenders, 2);
        assert_eq!(c.logback_classes, 3);
        assert_eq!(c.loggers, 2);
        assert_eq!(c.encoders, 4);
        assert_eq!(c.rolling, 4);
        assert_eq!(c.references, 4);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"<html><body>x</body></html>"));
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
