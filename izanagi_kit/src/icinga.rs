//! Parser for Icinga 2 configuration files (`icinga2.conf`, `zones.conf`,
//! `services.conf`, `hosts.conf`, `*.conf` in `/etc/icinga2/`).
//!
//! Counts `object Host|Service|CheckCommand|NotificationCommand|Endpoint|Zone|ApiUser|TimePeriod|User|UserGroup|HostGroup|ServiceGroup|Dependency|Downtime|ScheduledDowntime|Comment|Acknowledgement|Notification|EventCommand|EventHandler|PerfdataWriter|ApiListener|FileLogger|SyslogLogger|IcingaApplication|CheckerComponent|MonitoringComponent|CompatLogger|ExternalCommandListener|GelfWriter|GraphiteWriter|IdoMysqlConnection|IdoPgsqlConnection|InfluxdbWriter|Influxdb2Writer|LivestatusListener|OpenTsdbWriter|PerfdataWriter|StatusDataWriter|SyslogLogger|ElasticsearchWriter|InfluxWriter` blocks, `template`/`import`/`apply`/`assign where`/`ignore where`, `var`/`vars`/`const`/`include`/`include_recursive`, `check_command`/`check_interval`/`retry_interval`/`enable_*`/`max_check_attempts`/`command_endpoint`, `vars.*`/`display_name`/`address`/`icon_image`/`notes`/`action_url`/`groups`/`zone`/`parent`/`host_name`/`service_name`/`where`/`when`/`command`/`arguments`/`env`/`timeout`/`worker`/`interval`/`is_volatile`/`volatile`/`enable_notifications`/`enable_active_checks`/`enable_passive_checks`/`enable_event_handler`/`enable_flapping`/`enable_perfdata`/`send_notifications`/`users`/`user_groups`/`times`/`states`/`types`/`period`/`fixed`/`flexible`/`author`/`comment`/`entry_time`/`duration`/`scheduled_by`/`triggered_by`/`package`, and `//`/`#` comments.
//!
//! ```
//! let b = b"object Host \"web1\" {\n  check_command = \"hostalive\"\n  address = \"1.1.1.1\"\n}\n";
//! assert!(izanagi_kit::icinga::detect(b));
//! let c = izanagi_kit::icinga::Icinga::parse(b).unwrap();
//! assert_eq!(c.objects, 1);
//! assert_eq!(c.hosts, 1);
//! ```

/// Parsed Icinga 2 config summary.
#[derive(Debug, Clone)]
pub struct Icinga {
    /// `object <Type> "<name>" { }` blocks.
    pub objects: usize,
    /// `object Host` blocks.
    pub hosts: usize,
    /// `object Service`/`apply Service`/`apply Notification`/`object Notification` blocks.
    pub services: usize,
    /// `template <Type> "<name>" { }` blocks.
    pub templates: usize,
    /// `import "<name>"` lines.
    pub imports: usize,
    /// `include`/`include_recursive`/`include_zones` lines.
    pub includes: usize,
    /// `apply`/`assign where`/`ignore where`/`for`/`if`/`else` control statements.
    pub applies: usize,
    /// `var`/`vars`/`const`/`library`/`function`/`lambda`/`set_if`/`get_template`/`globals`/`run_once`/`this`/`parent`/`prototype` attribute assignments.
    pub vars: usize,
    /// `key = value` fields inside objects (`check_command`/`check_interval`/`retry_interval`/`max_check_attempts`/`zone`/`parent`/`host_name`/`service_name`/`display_name`/`address*`/`groups`/`users`/`times`/`states`/`types`/`period`/`enable_*`/`bind_*`/`socket_*`/`agent_*`/`ca_path`/`cert_path`/`key_path`/`crl_path`/`ticket_salt`/`accept_*`/`notification_*`/`check_period`/`flapping_*`/`max_attempts`/`filter`/`source`/`log_dir`/`severity`/`priority`/`command`/`arguments`/`env`/`timeout`/`worker`/`volatile`/`fixed`/`flexible`/`author`/`comment`/`entry_time`/`duration`/`scheduled_by`/`triggered_by`/`package`/`start_time`/`end_time`).
    pub fields: usize,
    /// `//`/`#` comment lines.
    pub comments: usize,
}

const OBJ_TYPES: &[&str] = &[
    "Host",
    "Service",
    "CheckCommand",
    "NotificationCommand",
    "EventCommand",
    "Endpoint",
    "Zone",
    "ApiUser",
    "TimePeriod",
    "User",
    "UserGroup",
    "HostGroup",
    "ServiceGroup",
    "Dependency",
    "Downtime",
    "ScheduledDowntime",
    "Comment",
    "Acknowledgement",
    "Notification",
    "EventHandler",
    "ApiListener",
    "FileLogger",
    "SyslogLogger",
    "IcingaApplication",
    "CheckerComponent",
    "MonitoringComponent",
    "CompatLogger",
    "ExternalCommandListener",
    "GelfWriter",
    "GraphiteWriter",
    "IdoMysqlConnection",
    "IdoPgsqlConnection",
    "InfluxdbWriter",
    "Influxdb2Writer",
    "LivestatusListener",
    "OpenTsdbWriter",
    "PerfdataWriter",
    "StatusDataWriter",
    "ElasticsearchWriter",
    "InfluxWriter",
];

const FIELDS: &[&str] = &[
    "check_command",
    "check_interval",
    "retry_interval",
    "max_check_attempts",
    "command_endpoint",
    "zone",
    "parent",
    "host_name",
    "service_name",
    "display_name",
    "address",
    "address6",
    "icon_image",
    "notes",
    "action_url",
    "groups",
    "users",
    "user_groups",
    "times",
    "states",
    "types",
    "period",
    "fixed",
    "flexible",
    "author",
    "comment",
    "entry_time",
    "duration",
    "command",
    "arguments",
    "env",
    "timeout",
    "worker",
    "is_volatile",
    "volatile",
    "enable_notifications",
    "enable_active_checks",
    "enable_passive_checks",
    "enable_event_handler",
    "enable_flapping",
    "enable_perfdata",
    "send_notifications",
    "scheduled_by",
    "triggered_by",
    "package",
    "start_time",
    "end_time",
    "severity",
    "priority",
    "filter",
    "source",
    "log_dir",
    "notification_interval",
    "notification_period",
    "check_period",
    "flapping_threshold",
    "flapping_threshold_low",
    "flapping_threshold_high",
    "max_attempts",
    "agent_endpoint",
    "agent_port",
    "ca_path",
    "cert_path",
    "key_path",
    "crl_path",
    "ticket_salt",
    "bind_host",
    "bind_port",
    "socket_path",
    "socket_type",
    "accept_config",
    "accept_commands",
];

/// Returns `true` when the bytes look like an Icinga 2 config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("object Host")
        || t.contains("object Service")
        || t.contains("apply Service")
        || t.contains("object Zone")
        || t.contains("object Endpoint"))
        && t.contains('{')
}

impl Icinga {
    /// Parses an Icinga 2 config, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            objects: 0,
            hosts: 0,
            services: 0,
            templates: 0,
            imports: 0,
            includes: 0,
            applies: 0,
            vars: 0,
            fields: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with("//") || tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(rest) = tr.strip_prefix("object ") {
                let ty = rest.split_whitespace().next().unwrap_or("");
                if OBJ_TYPES.contains(&ty) {
                    c.objects += 1;
                    match ty {
                        "Host" => c.hosts += 1,
                        "Service" | "Notification" => c.services += 1,
                        _ => {}
                    }
                }
                continue;
            }
            if tr.starts_with("template ") {
                c.templates += 1;
                continue;
            }
            if tr.starts_with("import ") {
                c.imports += 1;
                continue;
            }
            if tr.starts_with("include") {
                c.includes += 1;
                continue;
            }
            if tr.starts_with("apply ")
                || tr.starts_with("assign where")
                || tr.starts_with("ignore where")
                || tr.starts_with("for ")
                || tr.starts_with("if ")
                || tr.starts_with("else")
            {
                c.applies += 1;
                continue;
            }
            if tr.starts_with("var ")
                || tr.starts_with("const ")
                || tr.starts_with("library ")
                || tr.starts_with("function ")
                || tr.starts_with("lambda ")
            {
                c.vars += 1;
                continue;
            }
            if let Some((k, _)) = tr.split_once('=') {
                let key = k.trim().trim_start_matches("vars.");
                if FIELDS.contains(&key) {
                    c.fields += 1;
                } else {
                    c.vars += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"// icinga2\nobject Host \"web1\" {\n  check_command = \"hostalive\"\n  address = \"1.1.1.1\"\n  vars.os = \"linux\"\n}\nobject Service \"http\" {\n  host_name = \"web1\"\n  check_command = \"http\"\n  check_interval = 5m\n}\ntemplate Host \"generic\" {\n  max_check_attempts = 3\n}\napply Service \"ping\" {\n  import \"generic-service\"\n}\n";

    #[test]
    fn parses_icinga() {
        let c = Icinga::parse(CONF).unwrap();
        assert_eq!(c.objects, 2);
        assert_eq!(c.hosts, 1);
        assert_eq!(c.services, 1);
        assert_eq!(c.templates, 1);
        assert_eq!(c.applies, 1);
        assert_eq!(c.vars, 1);
        assert_eq!(c.fields, 6);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_icinga() {
        assert!(!detect(b"server { listen 80; }"));
        assert!(Icinga::parse(b"x").is_none());
    }
}
