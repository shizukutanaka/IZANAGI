//! Apache log4net `log4net.config` / `*.log4net` XML census.
//!
//! `<log4net>` root + `<appender name= type=>`/`<appender-ref`/
//! `<root>`/`<logger`/`<level`/`param name=`/`<layout`/
//! `<conversionPattern`/`filter`/`<threshold`/`<evaluator`/
//! `<lockingModel`/`<rollingStyle`/`<maximumFileSize`/
//! `<maxSizeRollBackups`/`datePattern`/`staticLogFileName`/
//! `<bufferSize`/`<lossy`/`<smtpHost` elements.
//!
//! ```rust
//! let l = br#"<log4net><appender name="c" type="log4net.Appender.ConsoleAppender"><layout type="log4net.Layout.PatternLayout"><conversionPattern value="%m%n"/></layout></appender><root><level value="INFO"/><appender-ref ref="c"/></root></log4net>"#;
//! assert!(izanagi_kit::log4net::detect(l));
//! ```

/// log4net config census.
#[derive(Debug, Clone)]
pub struct Log4net {
    /// `<log4net` root seen.
    pub has_root: bool,
    /// Element lines matching known log4net elements.
    pub elements: usize,
    /// `<!-- -->` comment lines.
    pub comments: usize,
}

fn element(tr: &str) -> bool {
    tr.starts_with("<appender")
        || tr.starts_with("<appender-ref")
        || tr.starts_with("<root")
        || tr.starts_with("<logger")
        || tr.starts_with("<level")
        || tr.starts_with("<param ")
        || tr.starts_with("<layout")
        || tr.starts_with("<conversionPattern")
        || tr.starts_with("<patternLayout")
        || tr.starts_with("<filter")
        || tr.starts_with("<threshold")
        || tr.starts_with("<evaluator")
        || tr.starts_with("<lockingModel")
        || tr.starts_with("<rollingStyle")
        || tr.starts_with("<maximumFileSize")
        || tr.starts_with("<maxSizeRollBackups")
        || tr.starts_with("<datePattern")
        || tr.starts_with("<staticLogFileName")
        || tr.starts_with("<countDirection")
        || tr.starts_with("<dateTimeStrategy")
        || tr.starts_with("<preserveLogFileNameExtension")
        || tr.starts_with("<bufferSize")
        || tr.starts_with("<lossy")
        || tr.starts_with("<immediateFlush")
        || tr.starts_with("<onlyFixPartialEventData")
        || tr.starts_with("<renderer")
        || tr.starts_with("<renderedClass")
        || tr.starts_with("<forwardingAppender")
        || tr.starts_with("<eventLog")
        || tr.starts_with("<smtpHost")
        || tr.starts_with("<to>")
        || tr.starts_with("<from>")
        || tr.starts_with("<subject")
        || tr.starts_with("<connectionStringName")
        || tr.starts_with("<connectionString")
        || tr.starts_with("<commandText")
        || tr.starts_with("<parameter")
        || tr.starts_with("<dataType")
        || tr.starts_with("<dbType")
        || tr.starts_with("<size")
        || tr.starts_with("<conversionPattern")
        || tr.starts_with("<key")
        || tr.starts_with("<global")
        || tr.starts_with("<log4net")
}

/// Detect a log4net config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut root = false;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.starts_with("<log4net") {
            root = true;
            continue;
        }
        if element(tr) {
            elems += 1;
        }
    }
    root || (elems >= 4 && t.contains("</log4net>"))
}

impl Log4net {
    /// Count elements. Returns `None` when the input does not look like
    /// a log4net config.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            has_root: false,
            elements: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("<!--") {
                c.comments += 1;
                continue;
            }
            if tr.starts_with("<log4net") {
                c.has_root = true;
                continue;
            }
            if element(tr) {
                c.elements += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<?xml version="1.0"?>
<log4net>
    <appender name="ConsoleAppender" type="log4net.Appender.ConsoleAppender">
        <layout type="log4net.Layout.PatternLayout">
            <conversionPattern value="%date %-5level %logger - %message%newline"/>
        </layout>
    </appender>
    <appender name="RollingFile" type="log4net.Appender.RollingFileAppender">
        <param name="File" value="app.log"/>
        <rollingStyle value="Size"/>
        <maximumFileSize value="10MB"/>
        <maxSizeRollBackups value="10"/>
        <staticLogFileName value="true"/>
        <lockingModel type="log4net.Appender.FileAppender+MinimalLock"/>
    </appender>
    <root>
        <level value="INFO"/>
        <appender-ref ref="ConsoleAppender"/>
        <appender-ref ref="RollingFile"/>
    </root>
    <logger name="MyApp.Data">
        <level value="WARN"/>
    </logger>
</log4net>
"#;
        assert!(detect(b));
        let c = Log4net::parse(b).unwrap();
        assert!(c.has_root);
        assert!(c.elements >= 15);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"<configuration></configuration>"));
        assert!(!detect(b"key=value\n"));
        assert!(Log4net::parse(b"").is_none());
    }
}
