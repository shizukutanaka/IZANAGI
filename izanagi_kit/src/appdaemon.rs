//! AppDaemon `appdaemon.yaml`/`apps.yaml` census.
//!
//! `appdaemon.yaml`: `appdaemon:` block (latitude/longitude/elevation/
//! time_zone/app_dir/total_threads/threads/pin_apps/pin_threads/
//! exclude_dirs/missing_app_warnings/production_mode/keywords/
//! cert_verify/cert_path/disable_apps/http://https://api:/admin://
//! namespaces://plugins://logs://access_log/error_log/
//! diag_output/parse_debug/process_debug/utility_delay/
//! plugin_performance_update/admin_delay/accurate_timestamps/
//! timewarp/speed/starttime/endtime), `hadashboard:` (dash_url/
//! dash_dir/dashboard/compiled_dir/dashboard_compile_directives/
//! module_parameters/rss_feeds/rss_update), `plugins:` HASS/HASS2/
//! mqtt plugin sub-keys (type/host/port/token/ha_key/cert_verify/
//! cert_path/daemon_sleep_time/birth_msg/will_msg/topic_payload/
//! client_id/event_name/namespace).
//!
//! `apps.yaml`: per-app blocks `app:`, `module:`, `class:`,
//! `Global:`/`global:`/`global_dependencies:`/`dependencies:`,
//! `constrain_*:`/`constrain_start_time`/`constrain_end_time`/
//! `constrain_days`/`constrain_presence`/`constrain_input_boolean`/
//! `constrain_input_select`/`constrain_state`/`constrain_now`,
//! `listen_log`/`log:`/`level:`/`alias:`/`loglevel:`/`log_messages:`/
//! `dependency:`/`plugin:`/`namespace:`/`pin:`/`pin_thread:`/`disable:`.
//!
//! ```rust
//! let y = concat!(
//!     "appdaemon:\n",
//!     "  latitude: 35\n",
//!     "  longitude: 139\n",
//!     "  elevation: 10\n",
//!     "  time_zone: Asia/Tokyo\n",
//!     "  plugins:\n",
//!     "    HASS:\n",
//!     "      type: hass\n",
//!     "      ha_url: http://localhost:8123\n",
//! );
//! let c = izanagi_kit::appdaemon::Appdaemon::parse(y.as_bytes()).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

const KEYS: &[&str] = &[
    "latitude",
    "longitude",
    "elevation",
    "time_zone",
    "app_dir",
    "apps",
    "total_threads",
    "threads",
    "pin_apps",
    "pin_threads",
    "exclude_dirs",
    "missing_app_warnings",
    "production_mode",
    "keywords",
    "cert_verify",
    "cert_path",
    "disable_apps",
    "http",
    "https",
    "api",
    "admin",
    "namespaces",
    "logs",
    "access_log",
    "error_log",
    "diag_output",
    "parse_debug",
    "process_debug",
    "utility_delay",
    "plugin_performance_update",
    "admin_delay",
    "accurate_timestamps",
    "timewarp",
    "speed",
    "starttime",
    "endtime",
    "dash_url",
    "dash_dir",
    "dashboard",
    "compiled_dir",
    "dashboard_compile_directives",
    "module_parameters",
    "rss_feeds",
    "rss_update",
    "type",
    "host",
    "port",
    "token",
    "ha_key",
    "ha_url",
    "cert_verify",
    "daemon_sleep_time",
    "birth_msg",
    "will_msg",
    "topic_payload",
    "client_id",
    "event_name",
    "namespace",
    "namespace_key",
    "persistent",
    "load_namespaces",
    "app",
    "module",
    "class",
    "Global",
    "global",
    "global_dependencies",
    "dependencies",
    "constrain_start_time",
    "constrain_end_time",
    "constrain_days",
    "constrain_presence",
    "constrain_input_boolean",
    "constrain_input_select",
    "constrain_state",
    "constrain_now",
    "constrain_lat",
    "constrain_lon",
    "listen_log",
    "log",
    "level",
    "alias",
    "loglevel",
    "log_messages",
    "dependency",
    "plugin",
    "pin",
    "pin_thread",
    "disable",
    "silence_switch",
    "switch",
    "entities",
    "entity",
    "state",
    "devices",
    "device",
    "events",
    "event",
    "conditions",
    "condition",
    "custom_constraints",
    "initialize",
    "sched_delay",
    "run_every",
    "run_daily",
    "run_hourly",
    "run_minutely",
    "sunrise",
    "sunset",
    "sun_offset",
    "topic",
    "topics",
    "qos",
    "payload",
    "on_message",
    "args",
    "kwargs",
];

/// AppDaemon configuration census.
#[derive(Debug, Clone)]
pub struct Appdaemon {
    /// `appdaemon:`/`hadashboard:`/`plugins:`/`HASS:`/`mqtt:`/`logs:`/`namespaces:`/`admin:`/`http:`/`api:`/`apps:`/`Global:`/`global:`/`dependencies:` blocks.
    pub sections: usize,
    /// App entries (a col-0 `<name>:` block under apps.yaml semantics) — counted as indented blocks when `module:`/`class:` present.
    pub apps: usize,
    /// `module:`/`class:`/`app:` keys.
    pub modules: usize,
    /// `constrain_*:`/`listen_log`/`run_*`/`sunrise`/`sunset` constraint/scheduling keys.
    pub constraints: usize,
    /// `key:` pairs matched against the key list (plugin/namespace/log/topic/entity/level/payload/etc).
    pub keys: usize,
    /// `type:`/`host:`/`port:`/`token:`/`ha_url:`/`ha_key:`/`cert_*:`/`client_id:`/`event_name:`/`namespace:`/`persistent:`/`daemon_sleep_time:`/`birth_msg:`/`will_msg:`/`topic_payload:` plugin-specific keys.
    pub plugins: usize,
}

fn is_key(s: &str, key: &str) -> bool {
    // `key :` (コロン前の空白)も YAML では合法。
    s.strip_prefix(key)
        .is_some_and(|r| r.trim_start().starts_with(':'))
}

fn has_key(t: &str, key: &str) -> bool {
    t.lines().any(|l| is_key(l.trim(), key))
}

/// Whether the buffer looks like an AppDaemon config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let s = l.trim();
        is_key(s, "appdaemon") || is_key(s, "hadashboard") || is_key(s, "plugins")
    }) || (has_key(t, "module") && has_key(t, "class"))
}

impl Appdaemon {
    /// Parse an AppDaemon config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            apps: 0,
            modules: 0,
            constraints: 0,
            keys: 0,
            plugins: 0,
        };
        for l in t.lines() {
            let s = l.trim_start();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let head = s.split(':').next().unwrap_or("").trim_end();
            if head == "appdaemon"
                || head == "hadashboard"
                || head == "plugins"
                || head == "HASS"
                || head == "HASS2"
                || head == "mqtt"
                || head == "MQTT"
                || head == "logs"
                || head == "namespaces"
                || head == "admin"
                || head == "http"
                || head == "api"
                || head == "apps"
                || head == "Global"
                || head == "global"
                || head == "dependencies"
                || head == "app"
                || head == "log"
                || head == "module_parameters"
            {
                c.sections += 1;
                continue;
            }
            if head == "module" || head == "class" || head == "type" {
                c.modules += 1;
                continue;
            }
            if head.starts_with("constrain_")
                || head == "listen_log"
                || head == "run_daily"
                || head == "run_hourly"
                || head == "run_minutely"
                || head == "sunrise"
                || head == "sunset"
                || head == "sun_offset"
                || head == "sched_delay"
            {
                c.constraints += 1;
                continue;
            }
            if head == "host"
                || head == "port"
                || head == "token"
                || head == "ha_url"
                || head == "ha_key"
                || head == "cert_verify"
                || head == "cert_path"
                || head == "client_id"
                || head == "event_name"
                || head == "namespace"
                || head == "namespace_key"
                || head == "persistent"
                || head == "daemon_sleep_time"
                || head == "birth_msg"
                || head == "will_msg"
                || head == "topic_payload"
                || head == "load_namespaces"
                || head == "broker"
                || head == "tls_version"
            {
                c.plugins += 1;
                continue;
            }
            if KEYS.contains(&head) {
                c.keys += 1;
                continue;
            }
            // indented arbitrary app keys (entity/args etc) — counted as apps-side
            if s.contains(':') {
                c.apps += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_space_before_colon() {
        // YAML では `key :` も合法。
        assert!(detect(b"appdaemon :\n  x: 1\n"));
        assert!(detect(b"module : a\nclass : b\n"));
    }

    #[test]
    fn parses_config() {
        let b = concat!(
            "appdaemon:\n",
            "  latitude: 35\n",
            "  longitude: 139\n",
            "  elevation: 10\n",
            "  time_zone: Asia/Tokyo\n",
            "  plugins:\n",
            "    HASS:\n",
            "      type: hass\n",
            "      ha_url: http://localhost:8123\n",
            "      token: tok\n",
            "    MQTT:\n",
            "      type: mqtt\n",
            "      host: 127.0.0.1\n",
            "      port: 1883\n",
            "hadashboard:\n",
            "  dash_url: http://localhost:5050\n",
        );
        let c = Appdaemon::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.modules, 2);
        assert_eq!(c.plugins, 4);
        assert_eq!(c.keys, 5);
    }

    #[test]
    fn parses_apps() {
        let b = concat!(
            "living:\n",
            "  module: lights\n",
            "  class: Light\n",
            "  entity: light.living\n",
            "  constrain_input_boolean: input_boolean.auto\n",
            "hall:\n",
            "  module: motion\n",
            "  class: Motion\n",
            "  sensor: binary_sensor.hall\n",
            "  log: my.log\n",
        );
        let c = Appdaemon::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 1);
        assert_eq!(c.modules, 4);
        assert_eq!(c.constraints, 1);
        assert_eq!(c.apps, 3);
        assert_eq!(c.keys, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Appdaemon::parse(b"foo = 1").is_none());
    }
}
