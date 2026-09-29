//! Parser for Nagios configuration files (`nagios.cfg`, `objects/*.cfg`).
//!
//! Counts `cfg_file=`/`cfg_dir=`/`resource_file=`/`log_file=`/`command_file=`/
//! `status_file=`/`object_cache_file=`/`precached_object_file=`/`temp_file=`/
//! `p1_file=`/`lock_file=`/`log_archive_path=` directives, `define host/service/
//! command/contact/timeperiod/hostgroup/servicegroup/contactgroup/dependency/
//! escalation/hostescalation/serviceescalation` object blocks, `use`/`name`/
//! `host_name`/`service_description`/`check_command`/`contact_groups`/
//! `max_check_attempts`/`check_interval`/`retry_interval`/`notification_interval`/
//! `notification_period`/`notifications_enabled`/`check_period`/`active_checks_enabled`/
//! `passive_checks_enabled`/`register`/`alias`/`address`/`email`/`members`/`parents`/
//! `hostgroups`/`servicegroups`/`icon_image`/`notes`/`notes_url`/`action_url` fields,
//! `host_dependencies`/`service_dependencies`/`host_notifications_enabled`/
//! `service_notifications_enabled`/`flap_detection_*`/`is_volatile`/`low_flap_threshold`/
//! `high_flap_threshold`/`process_perf_data`/`retain_*`/`retention_*`/`stalking_options`/
//! `event_handler`/`obsess_*`/`perfdata_*`/`check_*`/`debug_*`/`use_*`/`enable_*`
//! boolean keys, `interval_length`/`service_check_timeout`/`host_check_timeout`/
//! `notification_timeout`/`ocsp_timeout`/`ochp_timeout`/`perfdata_timeout`/
//! `sleep_time`/`service_inter_check_delay_method`/`service_interleave_factor`/
//! `max_concurrent_checks`/`max_host_check_spread`/`max_service_check_spread`/
//! `interval_length`/`timing_*` values, and `#`/`;` comments.
//!
//! ```
//! let b = b"cfg_file=/etc/nagios/objects/commands.cfg\nlog_file=/var/log/nagios/nagios.log\ndefine host {\n  host_name web1\n  check_command check_ping\n}\n";
//! assert!(izanagi_kit::nagios::detect(b));
//! let c = izanagi_kit::nagios::Nagios::parse(b).unwrap();
//! assert_eq!(c.defines, 1);
//! ```

/// Parsed Nagios config summary.
#[derive(Debug, Clone)]
pub struct Nagios {
    /// `cfg_file=`/`cfg_dir=`/`resource_file=`/`log_file=`/`command_file=`/
    /// `status_file=`/`object_cache_file=`/`precached_object_file=`/`temp_file=`/
    /// `p1_file=`/`lock_file=`/`log_archive_path=` file/path directives.
    pub path_directives: usize,
    /// `define host|service|command|contact|*group|timeperiod|dependency|escalation` blocks.
    pub defines: usize,
    /// Object field lines (`use`/`name`/`host_name`/`check_command`/`contact_groups`/`alias`/`address`/`email`/`members`/`parents`/`notes*`/`event_handler`/`obsess_*`/`stalking_*`/`flap_*`/`process_perf_data`/`retain_*`).
    pub object_fields: usize,
    /// `key=value` directives in nagios.cfg (`command_name`/`command_line`/`debug_*`/`log_*`/`nagios_*`/`broker_*`/`timeout`/`interval_length`/`max_*`/`admin_*`/`date_format`/`illegal_*`/`enable_*`/`allow_*`/`use_*`/`check_for_*`).
    pub nagios_cfg_keys: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const DEFS: &[&str] = &[
    "host",
    "service",
    "command",
    "contact",
    "contactgroup",
    "hostgroup",
    "servicegroup",
    "timeperiod",
    "dependency",
    "escalation",
    "hostdependency",
    "servicedependency",
    "hostescalation",
    "serviceescalation",
    "hostextinfo",
    "serviceextinfo",
];

const OBJ_FIELDS: &[&str] = &[
    "use",
    "name",
    "host_name",
    "service_description",
    "check_command",
    "contact_groups",
    "max_check_attempts",
    "check_interval",
    "retry_interval",
    "notification_interval",
    "notification_period",
    "notifications_enabled",
    "check_period",
    "active_checks_enabled",
    "passive_checks_enabled",
    "register",
    "alias",
    "address",
    "email",
    "members",
    "parents",
    "hostgroups",
    "servicegroups",
    "icon_image",
    "notes",
    "notes_url",
    "action_url",
    "event_handler",
    "obsess_over_host",
    "obsess_over_service",
    "stalking_options",
    "low_flap_threshold",
    "high_flap_threshold",
    "is_volatile",
    "flap_detection_enabled",
    "process_perf_data",
    "retain_status_information",
    "retain_nonstatus_information",
];

const PATH_DIRS: &[&str] = &[
    "cfg_file",
    "cfg_dir",
    "resource_file",
    "log_file",
    "command_file",
    "status_file",
    "object_cache_file",
    "precached_object_file",
    "temp_file",
    "p1_file",
    "lock_file",
    "log_archive_path",
];

const CFG_KEYS: &[&str] = &[
    "host_dependencies",
    "service_dependencies",
    "host_notifications_enabled",
    "service_notifications_enabled",
    "initial_state",
    "first_notification_delay",
    "notification_options",
    "freshness_threshold",
    "check_freshness",
    "obsess",
    "passive",
    "parallelize_check",
    "exclude_from_check",
    "normal_check_interval",
    "retry_check_interval",
    "max_attempts",
    "command_name",
    "command_line",
    "contact_name",
    "contactgroup_name",
    "hostgroup_name",
    "servicegroup_name",
    "timeperiod_name",
    "admin_email",
    "admin_pager",
    "date_format",
    "illegal_macro_output_chars",
    "illegal_object_name_chars",
    "use_aggressive_host_checking",
    "check_for_updates",
    "bare_update_check",
    "daemon_dumps_core",
    "use_ssl",
    "enable_embedded_perl",
    "use_embedded_perl_implicitly",
    "enable_environment_macros",
    "allow_empty_hostgroup_assignment",
    "allow_circular_dependencies",
    "nagios_user",
    "nagios_group",
    "debug_level",
    "debug_verbosity",
    "debug_file",
    "max_debug_file_size",
    "event_broker_options",
    "broker_module",
    "log_rotation_method",
    "log_event_handlers",
    "log_notifications",
    "log_service_retries",
    "log_host_retries",
    "log_external_commands",
    "log_passive_checks",
    "log_initial_states",
    "log_current_states",
    "service_check_timeout",
    "host_check_timeout",
    "notification_timeout",
    "ocsp_timeout",
    "ochp_timeout",
    "perfdata_timeout",
    "sleep_time",
    "interval_length",
    "service_inter_check_delay_method",
    "host_inter_check_delay_method",
    "service_interleave_factor",
    "max_concurrent_checks",
    "max_service_check_spread",
    "max_host_check_spread",
];

/// Returns `true` when the bytes look like a Nagios config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let sig = t.lines().any(|l| {
        l.trim_start().starts_with("define ")
            || l.trim_start().starts_with("cfg_file=")
            || l.trim_start().starts_with("cfg_dir=")
    });
    sig && (t.contains('=') || t.contains("define"))
}

impl Nagios {
    /// Parses a Nagios config, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            path_directives: 0,
            defines: 0,
            object_fields: 0,
            nagios_cfg_keys: 0,
            comments: 0,
        };
        let mut in_def = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if tr == "}" {
                in_def = false;
                continue;
            }
            if let Some(rest) = tr.strip_prefix("define ") {
                let ty = rest.split_whitespace().next().unwrap_or("");
                if DEFS.contains(&ty) {
                    c.defines += 1;
                }
                in_def = true;
                continue;
            }
            if let Some((k, _)) = tr.split_once('=') {
                let key = k.trim();
                if PATH_DIRS.contains(&key) {
                    c.path_directives += 1;
                } else if OBJ_FIELDS.contains(&key) {
                    c.object_fields += 1;
                } else if CFG_KEYS.contains(&key) {
                    c.nagios_cfg_keys += 1;
                }
            } else if in_def {
                let key = tr.split_whitespace().next().unwrap_or("");
                if OBJ_FIELDS.contains(&key) || CFG_KEYS.contains(&key) {
                    c.object_fields += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONF: &[u8] = b"# nagios\ncfg_file=/etc/nagios/objects/commands.cfg\ncfg_dir=/etc/nagios/objects\nlog_file=/var/log/nagios/nagios.log\ndebug_level=0\n\ndefine command {\n  command_name check_ping\n  command_line /usr/lib/nagios/plugins/check_ping -H $HOSTADDRESS$\n}\ndefine host {\n  use linux-server\n  host_name web1\n  address 192.168.1.10\n  max_check_attempts 5\n  check_command check_ping\n}\ndefine service {\n  use generic-service\n  host_name web1\n  service_description HTTP\n  check_command check_http\n}\n";

    #[test]
    fn parses_nagios() {
        let c = Nagios::parse(CONF).unwrap();
        assert_eq!(c.path_directives, 3);
        assert_eq!(c.defines, 3);
        assert_eq!(c.object_fields, 11);
        assert_eq!(c.nagios_cfg_keys, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_nagios() {
        assert!(!detect(b"key=value\nother=thing"));
        assert!(Nagios::parse(b"x").is_none());
    }
}
