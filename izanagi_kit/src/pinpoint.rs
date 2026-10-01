//! Parser for Naver Pinpoint APM agent configuration (`pinpoint.config`,
//! `pinpoint-root.config`, `profiles/<env>/pinpoint.config`).
//!
//! Java-properties `key=value` lines grouped by dotted prefix:
//! `profiler.*` (agentId/applicationName/agentName/collector.ip/
//! applicationservertype/sampling/enabled plugins/transport/grpc),
//! `profiler.collector.*`, `profiler.transport.*`,
//! `profiler.sampling.*`, `profiler.instrument.*`,
//! `profiler.plugin.*`, `profiler.uri.stat.*`,
//! `profiler.support.int.*`/`profiler.security.*`,
//! `profiler.apilog.*`, `pinpoint.*`/`web.*`/`collector.*` legacy keys,
//! plus `#` comments.
//!
//! ```
//! let b = b"profiler.agentId=a1\nprofiler.applicationName=svc\nprofiler.collector.ip=127.0.0.1\n";
//! assert!(izanagi_kit::pinpoint::detect(b));
//! let c = izanagi_kit::pinpoint::Pinpoint::parse(b).unwrap();
//! assert_eq!(c.settings, 3);
//! assert!(c.profiler_keys >= 3);
//! ```

/// Parsed pinpoint.config summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pinpoint {
    /// Total `key=value`/`key: value` settings.
    pub settings: usize,
    /// `profiler.*` keys.
    pub profiler_keys: usize,
    /// Distinct second-level groups under `profiler.*` (`collector`, `transport`, `sampling`, `instrument`, `plugin`, `uri`, `jdbc`, `log`, `grpc`, `thrift`, `stat`, `security`, `cpu`, `memory`, `os`, `server`, `agent`, `application`, `enable`, `trace`, `include`, `exclude`).
    pub groups: usize,
    /// `profiler.*.enabled=false`-style plugin toggles (value `true`/`false`).
    pub toggles: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Detects pinpoint-style config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let mut hits = 0usize;
    for l in t.lines() {
        let s = l.trim();
        if s.starts_with("profiler.") {
            hits += 1;
        }
        if s.contains("pinpoint") || s.contains("Pinpoint") {
            hits += 2;
        }
    }
    hits >= 3
}

impl Pinpoint {
    /// Parse pinpoint config; `None` when no settings found.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let Ok(t) = core::str::from_utf8(b) else {
            return None;
        };
        let mut c = Self {
            settings: 0,
            profiler_keys: 0,
            groups: 0,
            toggles: 0,
            comments: 0,
        };
        let mut seen = std::vec::Vec::<u64>::new();
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with('!') {
                c.comments += 1;
                continue;
            }
            let Some(sep) = s.find('=').or_else(|| s.find(": ")) else {
                continue;
            };
            let key = s[..sep].trim_end();
            if key.is_empty()
                || !key.bytes().all(|x| {
                    x.is_ascii_alphanumeric() || x == b'.' || x == b'_' || x == b'-' || x == b'$'
                })
            {
                continue;
            }
            c.settings += 1;
            if let Some(rest) = key.strip_prefix("profiler.") {
                c.profiler_keys += 1;
                let g = rest.split('.').next().unwrap_or("");
                let mut h = 0u64;
                for x in g.bytes() {
                    h = h.wrapping_mul(257).wrapping_add(u64::from(x));
                }
                if !seen.contains(&h) {
                    seen.push(h);
                    c.groups += 1;
                }
                let v = s[sep + 1..].trim_start_matches(':').trim();
                if (v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false"))
                    && (key.ends_with(".enabled") || key.contains("plugin"))
                {
                    c.toggles += 1;
                }
            }
        }
        (c.settings > 0).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pinpoint() {
        assert!(detect(
            b"profiler.agentId=a\nprofiler.applicationName=s\nprofiler.collector.ip=h\n"
        ));
        assert!(!detect(b"foo=bar\n"));
    }

    #[test]
    fn counts_groups() {
        let c = Pinpoint::parse(
            b"profiler.agentId=a\nprofiler.collector.ip=h\nprofiler.collector.tcpPort=1\nprofiler.sampling.enable=true\nprofiler.plugin.tomcat.enabled=false\n",
        )
        .unwrap();
        assert_eq!(c.settings, 5);
        assert_eq!(c.profiler_keys, 5);
        assert_eq!(c.groups, 4);
        assert_eq!(c.toggles, 1);
    }

    #[test]
    fn rejects() {
        assert!(Pinpoint::parse(b"nothing\n").is_none());
    }
}
