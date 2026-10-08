//! NLog `NLog.config` 設定ファイル (XML) の解析。
//!
//! `<nlog>` ルート、`<targets>`/`<target xsi:type="File|Console|ColoredConsole|
//! Mail|Database|Memory|Network|Null|AsyncWrapper|AutoLoadWrapper|
//! BufferingWrapper|FallbackGroup|FilteringWrapper|LimitingWrapper|MethodCall|
//! RetryingWrapper|SplitGroup|WebService|Chainsaw|DebugSystem|EventLog|
//! NLogViewer|OutputDebugString|PerfCounter|WebSocket|Trace|LogReceiverService">`
//! ターゲット、`<rules>`/`<logger name minlevel|level writeTo final>`、
//! `<extensions>`/`<add assembly>`、`<variable name value>`、`<include file>`、
//! `throwExceptions`/`internalLogFile`/`internalLogLevel`/`globalThreshold`/
//! `autoReload`/`keepVariablesOnReload`/`throwConfigExceptions`/
//! `parseMessageTemplates`/`optimizeBufferReuse`/`internalLogToConsole`/
//! `internalLogToFile`/`internalLogToTrace`/`internalLogIncludeTimestamp`/
//! `useInvariantCulture` 等を計数する。
//!
//! # Examples
//!
//! ```
//! use izanagi_kit::nlogconf;
//!
//! let text = br#"<nlog><targets><target name="file" xsi:type="File" fileName="app.log"/></targets><rules><logger name="*" minlevel="Info" writeTo="file"/></rules></nlog>"#;
//!
//! assert!(nlogconf::detect(text));
//! let c = nlogconf::parse(text).unwrap();
//! assert_eq!(c.targets, 1);
//! ```

use crate::textutil::strip_xml_comments;
/// ターゲット `xsi:type` の既知値。
const TARGET_TYPES: &[&str] = &[
    "File",
    "Console",
    "ColoredConsole",
    "Mail",
    "Database",
    "Memory",
    "Network",
    "Null",
    "AsyncWrapper",
    "AutoLoadWrapper",
    "BufferingWrapper",
    "FallbackGroup",
    "FilteringWrapper",
    "LimitingWrapper",
    "MethodCall",
    "RetryingWrapper",
    "SplitGroup",
    "WebService",
    "Chainsaw",
    "DebugSystem",
    "EventLog",
    "NLogViewer",
    "OutputDebugString",
    "PerfCounter",
    "WebSocket",
    "Trace",
    "LogReceiverService",
    "Debugger",
    "Debug",
    "Csv",
    "Group",
    "RefCounting",
    "RoundRobinGroup",
    "Repeatable",
    "Random",
];

/// NLog 設定の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `<nlog` ルート宣言数。
    pub roots: usize,
    /// `<target ` 宣言数。
    pub targets: usize,
    /// `xsi:type="*"` ターゲット型指定数。
    pub xsi_types: usize,
    /// `<logger ` ルール宣言数。
    pub rules: usize,
    /// `<extensions>`/`<add ` 拡張宣言数。
    pub extensions: usize,
    /// `<variable`/`<include`/`<time`/`globalThreshold`/`internalLog*`/`throwExceptions` 等その他要数数。
    pub misc: usize,
    /// `writeTo`/`name`/`minlevel`/`level`/`fileName`/`layout` 属性宣言数。
    pub attributes: usize,
}

/// `b` が NLog 設定らしいかを返す。
pub fn detect(b: &[u8]) -> bool {
    let c = match parse(b) {
        Some(c) => c,
        None => return false,
    };
    c.roots >= 1 && c.targets + c.rules >= 1
}

/// NLog 設定を解析して `Counts` を返す。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let s = strip_xml_comments(std::str::from_utf8(b).ok()?);
    let mut counts = Counts {
        roots: 0,
        targets: 0,
        xsi_types: 0,
        rules: 0,
        extensions: 0,
        misc: 0,
        attributes: 0,
    };
    let mut saw_any = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("<!--") {
            continue;
        }
        saw_any = true;
        counts.roots += line.matches("<nlog").count();
        counts.targets += line.matches("<target ").count();
        counts.targets += line.matches("<target>").count();
        counts.xsi_types += line.matches("xsi:type=\"").count();
        for t in TARGET_TYPES {
            counts.misc += line.matches(&["xsi:type=\"", t, "\""].concat()).count();
        }
        counts.rules += line.matches("<logger ").count();
        counts.rules += line.matches("<logger>").count();
        counts.extensions += line.matches("<extensions").count();
        counts.extensions += line.matches("<add ").count();
        counts.extensions += line.matches("<add>").count();
        counts.misc += line.matches("<variable").count();
        counts.misc += line.matches("<include").count();
        counts.misc += line.matches("<time ").count();
        counts.misc += line.matches("throwExceptions").count();
        counts.misc += line.matches("internalLog").count();
        counts.misc += line.matches("globalThreshold").count();
        counts.misc += line.matches("autoReload").count();
        counts.misc += line.matches("throwConfigExceptions").count();
        counts.misc += line.matches("parseMessageTemplates").count();
        counts.misc += line.matches("optimizeBufferReuse").count();
        counts.misc += line.matches("useInvariantCulture").count();
        counts.attributes += line.matches(" writeTo=\"").count();
        counts.attributes += line.matches(" name=\"").count();
        counts.attributes += line.matches(" minlevel=\"").count();
        counts.attributes += line.matches(" level=\"").count();
        counts.attributes += line.matches(" fileName=\"").count();
        counts.attributes += line.matches(" layout=\"").count();
        counts.attributes += line.matches("<layout ").count();
        counts.attributes += line.matches("<layout>").count();
        counts.attributes += line.matches("${").count();
        counts.attributes += line.matches(" xsi:type=\"").count();
    }
    if !saw_any {
        return None;
    }
    Some(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<nlog xmlns="http://www.nlog-project.org/schemas/NLog.xsd"
      xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
      autoReload="true" throwExceptions="false" internalLogFile="nlog-internal.log">
  <extensions>
    <add assembly="NLog.Web"/>
  </extensions>
  <variable name="logdir" value="/var/log"/>
  <targets>
    <target name="file" xsi:type="File" fileName="${logdir}/app.log"
            layout="${longdate}|${level}|${message}"/>
    <target name="console" xsi:type="ColoredConsole"/>
    <target name="async" xsi:type="AsyncWrapper" overflowAction="Block">
      <target xsi:type="Database" connectionString="Server=."/>
    </target>
  </targets>
  <rules>
    <logger name="Microsoft.*" maxlevel="Info" final="true"/>
    <logger name="*" minlevel="Debug" writeTo="file,console"/>
    <logger name="*" minlevel="Info" writeTo="async"/>
  </rules>
</nlog>
"#;

    #[test]
    fn detects_nlog() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.roots, 1);
        assert_eq!(c.targets, 4);
        assert_eq!(c.xsi_types, 4);
        assert_eq!(c.rules, 3);
        assert_eq!(c.extensions, 2);
        assert!(c.attributes >= 10);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"hello world"));
        assert!(!detect(b"<html><body>x</body></html>"));
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_xml_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_xml_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
