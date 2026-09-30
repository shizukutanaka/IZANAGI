//! Zigbee2MQTT `configuration.yaml` census.
//!
//! Sections: `homeassistant:`, `mqtt:` (server/base_topic/user/password/
//! client_id/keepalive/version/include_device_information/force_disable_retain/
//! ca/key/cert/reject_unauthorized), `serial:` (port/adapter/baudrate/
//! rtscts/disable_led), `permit_join:`, `frontend:` (port/host/auth_token/
//! url), `devices:`/`groups:`/`friend:`/`blocklist:`/`passlist:` (device
//! entries keyed by IEEE addr `0x00124b...`), `advanced:` (network_key/
//! pan_id/channel/log_level/log_output/last_seen/elapsed/timestamp_format/
//! zigbee_herdsman_debug/ikea_ota_use_test_url/ext_pan_id/baudrate/
//! adapter_concurrent/adapter_delay/transmit_power/output/cache_*/log_*/api_*),
//! `external_converters`, `device_options`/`group_options`/`friend`/
//! `availability`, `ota:`/`icon/`/`map_options`/`experimental:`/`new_api:`/
//! `socat:`/`network_map` and `friendly_name:`/`retain:`/`qos:`/`debounce:`/
//! `debounce_ignore:`/`optimistic:`/`filtered_attributes:`/`description:`/
//! `disabled:`/`device_specific_options` per-device keys.
//!
//! ```rust
//! let y = concat!(
//!     "homeassistant: true\n",
//!     "permit_join: false\n",
//!     "mqtt:\n",
//!     "  server: mqtt://localhost\n",
//!     "  base_topic: zigbee2mqtt\n",
//!     "serial:\n",
//!     "  port: /dev/ttyUSB0\n",
//!     "devices:\n",
//!     "  '0x00158d0001234567':\n",
//!     "    friendly_name: lamp\n",
//! );
//! let c = izanagi_kit::zigbee2mqtt::Zigbee2mqtt::parse(y.as_bytes()).unwrap();
//! assert_eq!(c.sections, 5);
//! ```

const KEYS: &[&str] = &[
    "server",
    "base_topic",
    "user",
    "password",
    "client_id",
    "keepalive",
    "version",
    "include_device_information",
    "force_disable_retain",
    "ca",
    "key",
    "cert",
    "reject_unauthorized",
    "port",
    "adapter",
    "baudrate",
    "rtscts",
    "disable_led",
    "host",
    "auth_token",
    "url",
    "fragmentation",
    "network_key",
    "pan_id",
    "ext_pan_id",
    "channel",
    "channels",
    "log_level",
    "log_output",
    "log_directory",
    "log_file",
    "log_rotation",
    "log_symlink_current",
    "log_syslog",
    "last_seen",
    "elapsed",
    "timestamp_format",
    "zigbee_herdsman_debug",
    "ikea_ota_use_test_url",
    "adapter_concurrent",
    "adapter_delay",
    "transmit_power",
    "output",
    "cache_state",
    "cache_state_persistent",
    "cache_state_send_on_startup",
    "device_specific_options",
    "whitelist",
    "ban",
    "soft_reset_timeout",
    "network_key_keep_old",
    "homeassistant_discovery_topic",
    "homeassistant_status_topic",
    "homeassistant_legacy_entity_attributes",
    "homeassistant_legacy_triggers",
    "legacy_api",
    "legacy_availability_payload",
    "mqtt",
    "serial",
    "permit_join",
    "frontend",
    "devices",
    "groups",
    "friend",
    "blocklist",
    "passlist",
    "advanced",
    "external_converters",
    "device_options",
    "group_options",
    "availability",
    "ota",
    "icon",
    "map_options",
    "experimental",
    "new_api",
    "socat",
    "network_map",
    "friendly_name",
    "retain",
    "qos",
    "debounce",
    "debounce_ignore",
    "optimistic",
    "filtered_attributes",
    "description",
    "disabled",
    "simulated_brightness",
    "filtered_optimistic",
    "filtered_cache",
    "icon",
    "vendor",
    "model",
    "manufacturer",
    "converters",
    "report",
    "legacy",
    "retrieve_state",
    "homeassistant",
    "mqtt",
    "ssl",
    "proxy",
    "health_check",
    "watchdog",
    "watchdog_tier",
    "onboarding",
];

/// Zigbee2MQTT configuration census.
#[derive(Debug, Clone)]
pub struct Zigbee2mqtt {
    /// `homeassistant:`/`permit_join:`/`mqtt:`/`serial:`/`frontend:`/`advanced:`/`availability:`/`ota:`/`experimental:`/`new_api:`/`socat:`/`network_map:`/`map_options:`/`external_converters:`/`device_options:`/`group_options:`/`friend:`/`blocklist:`/`passlist:`/`groups:`/`devices:` sections/flags.
    pub sections: usize,
    /// Indented `key:` pairs matched against the key list (mqtt/serial/advanced sub-keys).
    pub keys: usize,
    /// Device/group entries: quoted or bare `0x...` keys or `N:` under `devices:`/`groups:`.
    pub devices: usize,
    /// `friendly_name:`/`retain:`/`qos:`/`debounce`/`optimistic:`/`disabled:`/`description:`/`filtered_*:` per-device keys.
    pub options: usize,
    /// `true`/`false` boolean values.
    pub bools: usize,
}

/// Whether the buffer looks like a zigbee2mqtt configuration.yaml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("permit_join:")
        || t.contains("zigbee2mqtt")
        || t.contains("friendly_name:")
        || (t.contains("base_topic:") && t.contains("serial:"))
        || (t.contains("mqtt:") && t.contains("serial:") && t.contains("port:"))
}

impl Zigbee2mqtt {
    /// Parse a zigbee2mqtt configuration.yaml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            keys: 0,
            devices: 0,
            options: 0,
            bools: 0,
        };
        let mut in_devices: Option<usize> = None;
        let mut entry_indent: Option<usize> = None;
        for l in t.lines() {
            let s = l.trim_start();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let indent = l.len() - s.len();
            if indent == 0 {
                in_devices = None;
                entry_indent = None;
                let head = s.split(':').next().unwrap_or("").trim_end();
                if [
                    "homeassistant",
                    "permit_join",
                    "mqtt",
                    "serial",
                    "frontend",
                    "devices",
                    "groups",
                    "friend",
                    "blocklist",
                    "passlist",
                    "advanced",
                    "external_converters",
                    "device_options",
                    "group_options",
                    "availability",
                    "ota",
                    "icon",
                    "map_options",
                    "experimental",
                    "new_api",
                    "socat",
                    "network_map",
                    "zigbee2mqtt",
                    "version",
                ]
                .contains(&head)
                {
                    c.sections += 1;
                    if s.ends_with(':') && (head == "devices" || head == "groups") {
                        in_devices = Some(indent);
                    }
                }
                if s.ends_with(": true") || s.ends_with(": false") {
                    c.bools += 1;
                }
                continue;
            }
            if in_devices.is_some() {
                match entry_indent {
                    None => {
                        entry_indent = Some(indent);
                        c.devices += 1;
                        continue;
                    }
                    Some(e) if indent == e => {
                        c.devices += 1;
                        continue;
                    }
                    Some(e) if indent > e => {
                        if s.starts_with("friendly_name:")
                            || s.starts_with("retain:")
                            || s.starts_with("qos:")
                            || s.starts_with("debounce")
                            || s.starts_with("optimistic:")
                            || s.starts_with("disabled:")
                            || s.starts_with("description:")
                            || s.starts_with("filtered_")
                            || s.starts_with("icon:")
                            || s.starts_with("simulated_brightness")
                        {
                            c.options += 1;
                        }
                        if s.ends_with(": true") || s.ends_with(": false") {
                            c.bools += 1;
                        }
                        continue;
                    }
                    Some(_) => {
                        in_devices = None;
                        entry_indent = None;
                    }
                }
            }
            let head = s.split(':').next().unwrap_or("");
            if KEYS.contains(&head) {
                c.keys += 1;
            }
            if s.ends_with(": true") || s.ends_with(": false") {
                c.bools += 1;
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
            "homeassistant: true\n",
            "permit_join: false\n",
            "mqtt:\n",
            "  server: mqtt://localhost\n",
            "  base_topic: zigbee2mqtt\n",
            "  user: u\n",
            "  password: p\n",
            "serial:\n",
            "  port: /dev/ttyUSB0\n",
            "  adapter: zstack\n",
            "frontend:\n",
            "  port: 8080\n",
            "advanced:\n",
            "  network_key: GENERATE\n",
            "  pan_id: 6754\n",
            "  channel: 11\n",
            "  log_level: info\n",
            "devices:\n",
            "  '0x00158d0001234567':\n",
            "    friendly_name: lamp\n",
            "    retain: false\n",
            "    optimistic: true\n",
            "  '0x00158d0007654321':\n",
            "    friendly_name: switch\n",
            "    qos: 1\n",
            "groups:\n",
            "  '1':\n",
            "    friendly_name: all\n",
        );
        let c = Zigbee2mqtt::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 8);
        assert_eq!(c.devices, 3);
        assert_eq!(c.options, 6);
        assert_eq!(c.keys, 11);
    }

    #[test]
    fn rejects_other() {
        assert!(Zigbee2mqtt::parse(b"foo = 1").is_none());
    }
}
