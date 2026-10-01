//! Parser for Alertmanager configuration files (`alertmanager.yml`).
//!
//! Counts `global:`/`route:`/`receivers:`/`inhibit_rules:`/`mute_time_intervals:`
//! sections, receivers (`- name:` + `*_configs:` types), `match`/`matchers`/
//! `match_re`/`group_by`/`group_wait`/`group_interval`/`repeat_interval`,
//! `inhibit_rules` `source_match`/`target_match`, `time_intervals`,
//! `smtp_*`/`slack_*`/`webhook_config`/`pagerduty_*`/`opsgenie_*`/`victorops_*`/`pushover_*`/`wechat_*`/`webex_*`/`discord_*`/`msteams*`/`sns_*`/`telegram_*`/`rocketchat_*`/`jira_*` integrations, and `templates`/`resolve_timeout`.
//!
//! ```
//! let b = b"route:\n  receiver: default\nreceivers:\n  - name: default\n    webhook_configs:\n      - url: http://x\n";
//! assert!(izanagi_kit::alertmanager::detect(b));
//! let c = izanagi_kit::alertmanager::Alertmanager::parse(b).unwrap();
//! assert_eq!(c.routes, 1);
//! assert_eq!(c.receivers, 1);
//! ```

/// Parsed alertmanager.yml summary.
#[derive(Debug, Clone)]
pub struct Alertmanager {
    /// `global:` section + `resolve_timeout`/`smtp_*`/`slack_api_url*`/`http_config`/`victorops_api_*`/`opsgenie_api_*`/`wechat_api_*`/`telegram_api_url`/`webex_api_url` keys.
    pub global: usize,
    /// `route:` top-level + nested `- match:`/`routes:` sub-routes.
    pub routes: usize,
    /// `receivers:` `- name:` entries.
    pub receivers: usize,
    /// `*_configs:` receiver integration blocks.
    pub integrations: usize,
    /// `match`/`matchers`/`match_re`/`group_by`/`group_wait`/`group_interval`/`repeat_interval`/`continue` routing keys.
    pub routing_keys: usize,
    /// `inhibit_rules:` entries + `source_match`/`target_match`/`equal` keys.
    pub inhibit_rules: usize,
    /// `mute_time_intervals:`/`time_intervals:`/`active_time_intervals:` entries + `weekdays`/`days_of_month`/`months`/`years`/`times`/`location`.
    pub time_intervals: usize,
    /// `templates:` entries.
    pub templates: usize,
    /// `send_resolved`/`html`/`headers`/`title`/`text`/`api_url`/`url`/`channel`/`token`/`bot_token`/`chat_id`/`service_key`/`routing_key`/`webhook_url`/`tls_config`/`proxy_url`/`email_config` to/common fields.
    pub fields: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const INTEG: &[&str] = &[
    "email_configs",
    "pagerduty_configs",
    "pushover_configs",
    "slack_configs",
    "opsgenie_configs",
    "victorops_configs",
    "webhook_configs",
    "wechat_configs",
    "webex_configs",
    "discord_configs",
    "msteams_configs",
    "msteamsv2_configs",
    "sns_configs",
    "telegram_configs",
    "rocketchat_configs",
    "jira_configs",
    "incidentio_configs",
    "mattermost_configs",
];

const ROUTE_KEYS: &[&str] = &[
    "match",
    "matchers",
    "match_re",
    "group_by",
    "group_wait",
    "group_interval",
    "repeat_interval",
    "continue",
    "routes",
    "receiver",
];

const TIME_KEYS: &[&str] = &[
    "weekdays",
    "days_of_month",
    "months",
    "years",
    "times",
    "location",
];

const FIELD_KEYS: &[&str] = &[
    "send_resolved",
    "html",
    "headers",
    "title",
    "text",
    "api_url",
    "url",
    "channel",
    "token",
    "bot_token",
    "chat_id",
    "service_key",
    "routing_key",
    "severity",
    "webhook_url",
    "tls_config",
    "proxy_url",
    "username",
    "password",
    "to",
    "from",
    "require_tls",
    "smarthost",
];

/// Returns `true` when the bytes look like an alertmanager.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("receivers:") || t.contains("route:"))
        && (t.contains("_configs:") || t.contains("inhibit_rules:") || t.contains("group_by:"))
}

impl Alertmanager {
    /// Parses an alertmanager.yml, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            global: 0,
            routes: 0,
            receivers: 0,
            integrations: 0,
            routing_keys: 0,
            inhibit_rules: 0,
            time_intervals: 0,
            templates: 0,
            fields: 0,
            comments: 0,
        };
        let mut ctx = "";
        let mut ctx_i = 0usize;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let i = l.len() - l.trim_start().len();
            let key = tr
                .trim_start_matches('-')
                .trim()
                .split(':')
                .next()
                .unwrap_or("");
            if i == 0 {
                ctx = key;
                ctx_i = i;
                match key {
                    "global" => c.global += 1,
                    "route" => c.routes += 1,
                    "inhibit_rules" => c.inhibit_rules += 1,
                    "mute_time_intervals" | "time_intervals" => c.time_intervals += 1,
                    "templates" => c.templates += 1,
                    _ => {}
                }
                continue;
            }
            if i <= ctx_i {
                ctx = "";
            }
            if ctx == "route" && ROUTE_KEYS.contains(&key) {
                c.routing_keys += 1;
                if key == "routes" {
                    c.routes += 1;
                }
                continue;
            }
            if tr.starts_with("- ") && key == "name" && ctx == "receivers" {
                c.receivers += 1;
                continue;
            }
            if INTEG.contains(&key) {
                c.integrations += 1;
            }
            if ctx == "inhibit_rules" && ["source_match", "target_match", "equal"].contains(&key) {
                c.inhibit_rules += 1;
            }
            if (ctx == "mute_time_intervals" || ctx == "time_intervals")
                && (TIME_KEYS.contains(&key) || key == "name")
            {
                c.time_intervals += 1;
            }
            if FIELD_KEYS.contains(&key) {
                c.fields += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"global:\n  smtp_smarthost: m:587\nroute:\n  receiver: team-x\n  group_by: ['alertname']\n  group_wait: 30s\n  repeat_interval: 4h\n  routes:\n    - match:\n        severity: page\nreceivers:\n  - name: team-x\n    slack_configs:\n      - channel: '#alerts'\n        api_url: https://hooks.slack.com/x\n  - name: page\n    pagerduty_configs:\n      - service_key: KEY\ninhibit_rules:\n  - source_match:\n      severity: critical\n    target_match:\n      severity: warning\n    equal: ['alertname']\nmute_time_intervals:\n  - name: offhours\n    time_intervals:\n      - weekdays: ['saturday', 'sunday']\n";

    #[test]
    fn parses_alertmanager() {
        let c = Alertmanager::parse(CONF).unwrap();
        assert_eq!(c.global, 1);
        assert_eq!(c.routes, 2);
        assert_eq!(c.receivers, 2);
        assert_eq!(c.integrations, 2);
        assert_eq!(c.routing_keys, 6);
        assert_eq!(c.inhibit_rules, 4);
        assert_eq!(c.time_intervals, 3);
        assert!(c.fields >= 3);
        assert_eq!(c.comments, 0);
    }

    #[test]
    fn rejects_non_alertmanager() {
        assert!(!detect(b"scrape_configs:\n  - job_name: x"));
        assert!(Alertmanager::parse(b"x").is_none());
    }
}
