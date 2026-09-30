//! Home Assistant `configuration.yaml` census.
//!
//! Top-level `key:` sections (`homeassistant:`, `automation:`, `script:`,
//! `scene:`, `sensor:`, `light:`, `switch:`, `mqtt:`, `http:`, `recorder:`,
//! `logger:`, `lovelace:`, `group:`, `zone:`, `person:`, `input_*:`,
//! `template:`, `notify:`, `default_config:`, …), `- alias:`/`trigger:`/
//! `condition:`/`action:`/`service:`/`platform:` items inside lists,
//! `!include*`/`!secret`/`!env_var` tags and `customize:` entries.
//!
//! ```rust
//! let y = concat!(
//!     "default_config:\n",
//!     "homeassistant:\n",
//!     "  name: Home\n",
//!     "automation:\n",
//!     "  - alias: Night\n",
//!     "    trigger:\n",
//!     "      - platform: time\n",
//!     "    action:\n",
//!     "      - service: light.turn_off\n",
//! );
//! let c = izanagi_kit::homeassistant::Homeassistant::parse(y.as_bytes()).unwrap();
//! assert_eq!(c.sections, 3);
//! assert_eq!(c.items, 3);
//! ```

const SECTIONS: &[&str] = &[
    "homeassistant",
    "automation",
    "script",
    "scene",
    "sensor",
    "sensors",
    "binary_sensor",
    "light",
    "switch",
    "mqtt",
    "http",
    "frontend",
    "recorder",
    "logger",
    "lovelace",
    "group",
    "zone",
    "person",
    "input_boolean",
    "input_number",
    "input_text",
    "input_select",
    "input_datetime",
    "input_button",
    "template",
    "notify",
    "device_tracker",
    "camera",
    "media_player",
    "climate",
    "fan",
    "cover",
    "lock",
    "vacuum",
    "alarm_control_panel",
    "weather",
    "sun",
    "system_health",
    "config",
    "default_config",
    "discovery",
    "ssdp",
    "zeroconf",
    "upnp",
    "tts",
    "ios",
    "mobile_app",
    "cloud",
    "alexa",
    "google_assistant",
    "homekit",
    "history",
    "logbook",
    "map",
    "panel_iframe",
    "intent_script",
    "conversation",
    "energy",
    "image",
    "tag",
    "timer",
    "counter",
    "utility_meter",
    "schedule",
    "wake_word",
    "assist_pipeline",
    "shopping_list",
    "my",
    "stream",
    "backup",
    "bluetooth",
    "usb",
    "dhcp",
    "file",
    "folder_watcher",
    "influxdb",
    "prometheus",
    "shell_command",
    "rest",
    "rest_command",
    "command_line",
    "manual",
    "generic",
    "scrape",
    "sql",
    "statistics",
    "min_max",
    "derivative",
    "integration",
    "threshold",
    "trend",
    "bayesian",
    "history_stats",
    "ping",
    "version",
    "local_file",
    "mqtt_statestream",
    "mqtt_eventstream",
    "zwave_js",
    "zha",
    "homeassistant_yellow",
    "radio",
    "otbr",
    "thread",
    "matter",
    "esphome",
    "wled",
    "cast",
    "sonos",
    "denonavr",
    "hue",
    "lifx",
    "tplink",
    "kasa",
    "yeelight",
    "tasmota",
    "shelly",
    "tuya",
    "smartthings",
    "hacs",
    "customize",
    "auth_providers",
    "api",
    "websocket_api",
    "onboarding",
    "webrtc",
    "image_upload",
    "file_upload",
    "diagnostics",
    "analytics",
    "media_source",
    "system_log",
    "demo",
    "owntracks",
    "unifi",
    "flux",
    "adaptive_lighting",
    "pyscript",
    "spook",
    "watchman",
    "browser_mod",
    "mass",
    "music_assistant",
    "nodered",
    "frigate",
    "go2rtc",
    "scheduler",
    "auto_backup",
    "backup_monitor",
    "keymaster",
    "hacs_frontend",
];

/// Home Assistant configuration census.
#[derive(Debug, Clone)]
pub struct Homeassistant {
    /// Top-level `key:` sections matched against the component list.
    pub sections: usize,
    /// `- alias:`/`- name:`/`- platform:`/`- service:` list items.
    pub items: usize,
    /// `trigger:`/`condition:`/`action:`/`choose:` keys.
    pub flows: usize,
    /// `platform:`/`service:`/`entity_id:`/`device_id:`/`area_id:` keys.
    pub platforms: usize,
    /// `!include`/`!include_dir_*`/`!secret`/`!env_var`/`!input` tags.
    pub includes: usize,
    /// `customize:`/`packages:` keys.
    pub customizes: usize,
}

/// Whether the buffer looks like a Home Assistant YAML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| {
        let s = l.trim();
        s == "homeassistant:" || s.starts_with("automation:") || s == "default_config:"
    }) || (t.contains("platform:") && t.contains("entity_id:"))
        || (t.contains("!include") && t.contains("automation"))
}

impl Homeassistant {
    /// Parse a configuration.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            items: 0,
            flows: 0,
            platforms: 0,
            includes: 0,
            customizes: 0,
        };
        for l in t.lines() {
            let s = l.trim_start();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.contains("!include")
                || s.contains("!secret")
                || s.contains("!env_var")
                || s.contains("!input")
            {
                c.includes += 1;
            }
            let col0 = (l.len() - s.len()) == 0;
            if col0 {
                let head = s.trim_end_matches(':').trim_end();
                if SECTIONS.contains(&head) {
                    c.sections += 1;
                    continue;
                }
                if head == "customize" || head == "packages" {
                    c.customizes += 1;
                    continue;
                }
            }
            if s.starts_with("customize:") || s.starts_with("packages:") {
                c.customizes += 1;
                continue;
            }
            if s.starts_with("- alias:")
                || s.starts_with("- name:")
                || s.starts_with("- platform:")
                || s.starts_with("- service:")
            {
                c.items += 1;
                continue;
            }
            if s.starts_with("trigger:")
                || s.starts_with("condition:")
                || s.starts_with("action:")
                || s.starts_with("choose:")
                || s.starts_with("variables:")
                || s.starts_with("mode:")
                || s.starts_with("sequence:")
                || s.starts_with("repeat:")
            {
                c.flows += 1;
                continue;
            }
            if s.starts_with("platform:")
                || s.starts_with("service:")
                || s.starts_with("entity_id:")
                || s.starts_with("device_id:")
                || s.starts_with("area_id:")
                || s.starts_with("target:")
            {
                c.platforms += 1;
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
    fn parses_config() {
        let b = concat!(
            "default_config:\n",
            "homeassistant:\n",
            "  name: Home\n",
            "  customize: !include customize.yaml\n",
            "automation:\n",
            "  - alias: Night\n",
            "    trigger:\n",
            "      - platform: time\n",
            "        at: \"22:00:00\"\n",
            "    condition:\n",
            "      - condition: state\n",
            "        entity_id: input_boolean.guest\n",
            "    action:\n",
            "      - service: light.turn_off\n",
            "        target:\n",
            "          entity_id: light.living\n",
            "mqtt:\n",
            "  broker: 192.168.1.10\n",
            "  sensor: !include_dir_merge_list sensors/\n",
            "logger:\n",
            "  default: warning\n",
        );
        let c = Homeassistant::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 5);
        assert_eq!(c.items, 3);
        assert_eq!(c.flows, 3);
        assert_eq!(c.platforms, 3);
        assert_eq!(c.includes, 2);
        assert_eq!(c.customizes, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Homeassistant::parse(b"foo = 1").is_none());
    }
}
