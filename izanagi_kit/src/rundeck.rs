//! Rundeck job YAML census.
//!
//! A Rundeck job export is YAML: top-level `- name:`/`uuid:`/`group:`/
//! `project:`/`description:` keys plus `schedule:` (with `crontab:` or
//! `month`/`dayofmonth`/`weekday`/`hour`/`minute`/`year`), `sequence:` with
//! `commands:` items (`- exec:`/`script:`/`scriptfile:`/`scripturl:`/
//! `jobref:`), `nodefilters:`/`dispatch:`, `options:`/`option:` and
//! `notification:` (`email`/`webhook`/`plugin`). `parse` counts each class.
//!
//! ```rust
//! let r = concat!(
//!     "- name: job1\n",
//!     "  uuid: u1\n",
//!     "  sequence:\n",
//!     "    commands:\n",
//!     "      - exec: echo hi\n",
//!     "      - scriptfile: /x.sh\n",
//!     "  schedule:\n",
//!     "    month: '*'\n",
//!     "    weekday: '*'\n",
//!     "    hour: '3'\n",
//!     "    minute: '0'\n",
//! );
//! let c = izanagi_kit::rundeck::Rundeck::parse(r.as_bytes()).unwrap();
//! assert_eq!(c.jobs, 1);
//! assert_eq!(c.commands, 2);
//! ```

/// Rundeck job YAML census.
#[derive(Debug, Clone)]
pub struct Rundeck {
    /// Job entries (`- name:` at top level or `uuid:` keys).
    pub jobs: usize,
    /// `commands:` items (`- exec`/`script`/`scriptfile`/`jobref`/`type`).
    pub commands: usize,
    /// `exec:`/`script:`/`scriptfile:`/`scripturl:`/`args:` keys.
    pub execs: usize,
    /// `jobref` entries.
    pub jobrefs: usize,
    /// `schedule:`/`crontab:`/`month:`/`dayofmonth:`/`weekday:`/`hour:`/`minute:`/`year:`/`time:`/`seconds:` keys.
    pub schedules: usize,
    /// `nodefilters:`/`dispatch:`/`filter:`/`node:` keys.
    pub nodefilters: usize,
    /// `notification:`/`onfailure:`/`onsuccess:`/`email:`/`webhook`/`plugin`/`urls:` keys.
    pub notifications: usize,
    /// `options:`/`option:` keys.
    pub options: usize,
    /// `loglevel:`/`timeout`/`retry:`/`multipleExecutions` keys.
    pub extras: usize,
}

/// Whether the buffer looks like a Rundeck job YAML.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("sequence:") && (t.contains("exec:") || t.contains("commands:")))
        || t.contains("nodefilters:")
        || (t.contains("crontab:") && t.contains("- name:"))
}

impl Rundeck {
    /// Parse a Rundeck job YAML into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            jobs: 0,
            commands: 0,
            execs: 0,
            jobrefs: 0,
            schedules: 0,
            nodefilters: 0,
            notifications: 0,
            options: 0,
            extras: 0,
        };
        let mut in_commands = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.starts_with("commands:") {
                in_commands = true;
                continue;
            }
            if s == "sequence:" || s.starts_with("sequence:") {
                in_commands = false;
            }
            if s.starts_with("- exec:")
                || s.starts_with("- script")
                || s.starts_with("- jobref")
                || (in_commands && s.starts_with("- ") && !s.starts_with("- name:"))
            {
                c.commands += 1;
                if s.starts_with("- jobref") {
                    c.jobrefs += 1;
                }
                continue;
            }
            if s.starts_with("- name:") && !l.starts_with(' ') && !l.starts_with('\t') {
                c.jobs += 1;
                continue;
            }
            if s.starts_with("exec:")
                || s.starts_with("script:")
                || s.starts_with("scriptfile:")
                || s.starts_with("scripturl:")
                || s.starts_with("args:")
            {
                c.execs += 1;
                continue;
            }
            if [
                "schedule:",
                "crontab:",
                "month:",
                "dayofmonth:",
                "weekday:",
                "hour:",
                "minute:",
                "year:",
                "time:",
                "seconds:",
                "scheduleEnabled:",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.schedules += 1;
                continue;
            }
            if s.starts_with("nodefilters:")
                || s.starts_with("dispatch:")
                || s.starts_with("filter:")
                || s.starts_with("node ")
            {
                c.nodefilters += 1;
                continue;
            }
            if [
                "notification:",
                "onfailure:",
                "onsuccess:",
                "email:",
                "webhook",
                "plugin",
                "urls:",
                "recipients:",
            ]
            .iter()
            .any(|k| s.starts_with(k))
            {
                c.notifications += 1;
                continue;
            }
            if s.starts_with("options:") || s.starts_with("option:") {
                c.options += 1;
                continue;
            }
            if s.starts_with("loglevel")
                || s.starts_with("timeout")
                || s.starts_with("retry:")
                || s.starts_with("multipleExecutions")
            {
                c.extras += 1;
                continue;
            }
            if s.starts_with("uuid:") {
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_job() {
        let b = concat!(
            "- name: job1\n",
            "  uuid: u1\n",
            "  description: d\n",
            "  schedule:\n",
            "    month: '*'\n",
            "    weekday: '*'\n",
            "    hour: '3'\n",
            "    minute: '0'\n",
            "  sequence:\n",
            "    commands:\n",
            "      - exec: echo hi\n",
            "      - script: run\n",
            "      - jobref: { group: g, name: j }\n",
            "  nodefilters:\n",
            "    dispatch:\n",
            "      threadcount: 1\n",
            "  notification:\n",
            "    onsuccess:\n",
            "      email: { recipients: t }\n",
            "  options:\n",
            "    - name: opt\n",
        );
        let c = Rundeck::parse(b.as_bytes()).unwrap();
        assert_eq!(c.jobs, 1);
        assert_eq!(c.commands, 3);
        assert_eq!(c.jobrefs, 1);
        assert_eq!(c.schedules, 5);
        assert_eq!(c.nodefilters, 2);
        assert_eq!(c.notifications, 3);
        assert_eq!(c.options, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Rundeck::parse(b"foo: bar").is_none());
    }
}
