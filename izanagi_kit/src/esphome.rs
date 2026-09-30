//! ESPHome device YAML configuration census.
//!
//! An ESPHome config is headed by `esphome:` with `name:`/`friendly_name:`,
//! then `esp32:`/`esp8266:`/`rp2040:`/`bk72xx:`/`rtl87xx:`/`host:` platform
//! blocks, buses (`i2c:`/`spi:`/`uart:`/`one_wire:`/`canbus:`/`usb:`),
//! domains (`sensor:`/`binary_sensor:`/`switch:`/`light:`/`fan:`/`cover:`/
//! `climate:`/`display:`/`number:`/`select:`/`text_sensor:`/`button:`/`lock:`/
//! `media_player:`/`speaker:`/`microphone:`/`voice_assistant:`/`lvgl:`/`web_server:`/
//! `captive_portal:`/`api:`/`ota:`/`logger:`/`wifi:`/`mqtt:`/`http_request:`/
//! `script:`/`globals:`/`substitutions:`/`packages:`/`external_components:`/
//! `time:`/`font:`/`image:`/`color:`/`graph:`/`qr_code:`/`psram:`/`debug:`/
//! `safe_mode:`/`power_supply:`/`output:`/`interval:`/`improv_serial:`/
//! `bluetooth_proxy:`/`esp32_ble_tracker:`/`esp32_ble_beacon:`/`dashboard_import:`/
//! `sound_level:`/`sprinkler:`/`sun:`/`valve:`/`datetime:`/`alarm_control_panel:`/
//! `event:`/`text:`/`update:`), `- platform:`/`on_*:` automation lambdas and
//! `!secret` tags.
//!
//! ```rust
//! let y = concat!(
//!     "esphome:\n",
//!     "  name: node1\n",
//!     "esp32:\n",
//!     "  board: esp32dev\n",
//!     "wifi:\n",
//!     "  ssid: !secret ssid\n",
//!     "sensor:\n",
//!     "  - platform: dht\n",
//!     "    pin: GPIO4\n",
//! );
//! let c = izanagi_kit::esphome::Esphome::parse(y.as_bytes()).unwrap();
//! assert_eq!(c.sections, 4);
//! ```

const SECTIONS: &[&str] = &[
    "esphome",
    "esp32",
    "esp8266",
    "rp2040",
    "bk72xx",
    "rtl87xx",
    "host",
    "i2c",
    "spi",
    "uart",
    "one_wire",
    "canbus",
    "usb",
    "sensor",
    "binary_sensor",
    "switch",
    "light",
    "fan",
    "cover",
    "climate",
    "display",
    "number",
    "select",
    "text_sensor",
    "button",
    "lock",
    "media_player",
    "speaker",
    "microphone",
    "micro_wake_word",
    "voice_assistant",
    "lvgl",
    "web_server",
    "captive_portal",
    "api",
    "ota",
    "logger",
    "wifi",
    "mqtt",
    "ethernet",
    "modem",
    "http_request",
    "script",
    "globals",
    "substitutions",
    "packages",
    "external_components",
    "time",
    "font",
    "image",
    "color",
    "graph",
    "qr_code",
    "psram",
    "debug",
    "safe_mode",
    "power_supply",
    "output",
    "interval",
    "improv_serial",
    "improv_ble",
    "bluetooth_proxy",
    "esp32_ble_tracker",
    "esp32_ble_beacon",
    "ble_client",
    "dashboard_import",
    "sound_level",
    "sprinkler",
    "sun",
    "valve",
    "datetime",
    "alarm_control_panel",
    "event",
    "text",
    "update",
    "factory_reset",
    "shutdown",
    "status_led",
    "status",
    "restart",
    "deep_sleep",
    "rotary_encoder",
    "remote_receiver",
    "remote_transmitter",
    "rf_bridge",
    "pca9685",
    "mcp23xxx",
    "sx1509",
    "adc",
    "pulse_counter",
    "pulse_meter",
    "pwm",
    "ledc",
    "servo",
    "stepper",
    "hlw8012",
    "cse7766",
    "pzemac",
    "pzemdc",
    "scd30",
    "sht4x",
    "bme280",
    "bmp280",
    "bmp085",
    "aht10",
    "dht",
    "dallas",
    "ds18b20",
    "max31855",
    "max6675",
    "max7219",
    "max7219digit",
    "max6956",
    "tm1637",
    "tm1638",
    "ht16k33",
    "lcd_pcf8574",
    "lcd_gpio",
    "ssd1306_i2c",
    "ssd1306_spi",
    "ssd1331",
    "ssd1322",
    "ssd1325",
    "ssd1327",
    "ssd1351",
    "st7735",
    "st7789v",
    "ili9341",
    "ili9342",
    "ili9481",
    "ili9486",
    "ili9488",
    "waveshare_epaper",
    "nextion",
    "tft",
    "touchscreen",
    "keypad",
    "matrix_keypad",
    "matrix",
    "mpr121",
    "tt21100",
    "axp192",
    "axp2101",
    "sm2135",
    "sm2235",
    "sm2335",
    "bp1658cj",
    "bp5758d",
    "my9231",
    "p9713",
    "sm16716",
    "ws2812",
    "addressable",
    "neopixelbus",
    "lightwaverf",
    "rc_switch",
    "hbridge",
    "ac_dimmer",
    "tuya",
    "dfplayer",
    "max98543a",
    "pn532",
    "rc522",
    "rdm6300",
    "wiegand",
    "fingerprint_grow",
    "teleinfo",
    "modbus",
    "modbus_controller",
    "selec_meter",
    "pipsolar",
    "growatt_solar",
    "sml",
    "demod",
    "tolink",
    "zoem8q",
    "gps",
    "time_based",
    "combination",
    "template",
    "custom",
    "fastled_clockless",
    "fastled_spi",
    "chase",
    "partition",
    "rgb",
    "rgbw",
    "rgbww",
    "rgbct",
    "cwww",
    "cold_white",
    "monochrome",
    "binary",
    "beep",
    "esp32_rmt_led_strip",
    "rp2040_pio_led_strip",
    "midea",
    "heatpumpir",
    "analog_threshold",
    "endstop",
    "haier",
    "airwell",
    "am43",
    "cover",
    "time_cover",
    "endstop_cover",
    "feedback",
    "tormatic",
    "teleinfo",
    "esp32_camera",
    "esp32_camera_web_server",
    "esp32_touch",
];

/// ESPHome configuration census.
#[derive(Debug, Clone)]
pub struct Esphome {
    /// Top-level section keys matched against the component list.
    pub sections: usize,
    /// `- platform:` list items.
    pub platforms: usize,
    /// `platform:` keys (non-list form).
    pub platform_keys: usize,
    /// `name:`/`id:` keys.
    pub names: usize,
    /// `pin:`/`GPIO*`/address/`update_interval:`/`i2c_id:`/`spi_id:`/`uart_id:` references.
    pub refs: usize,
    /// `on_*:`/`then:` automation keys + `lambda:`/`script.execute` calls.
    pub automations: usize,
    /// `!secret`/`!include`/`!lambda` tags + `${subst}` substitutions.
    pub secrets: usize,
}

/// Whether the buffer looks like an ESPHome YAML config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| l.trim() == "esphome:")
        || (t.contains("- platform:") && (t.contains("esphome") || t.contains("wifi:")))
}

impl Esphome {
    /// Parse an ESPHome config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            platforms: 0,
            platform_keys: 0,
            names: 0,
            refs: 0,
            automations: 0,
            secrets: 0,
        };
        for l in t.lines() {
            let s = l.trim_start();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            if s.contains("!secret") || s.contains("!include") || s.contains("!lambda") {
                c.secrets += 1;
            }
            if s.contains("${") {
                c.secrets += 1;
            }
            let col0 = (l.len() - s.len()) == 0;
            if col0 {
                let head = s.trim_end_matches(':').trim_end();
                if SECTIONS.contains(&head) {
                    c.sections += 1;
                    continue;
                }
            }
            if s.starts_with("- platform:") {
                c.platforms += 1;
                continue;
            }
            if s.starts_with("platform:") {
                c.platform_keys += 1;
                continue;
            }
            if s.starts_with("name:") || s.starts_with("id:") {
                c.names += 1;
                continue;
            }
            if s.starts_with("pin:")
                || s.starts_with("address:")
                || s.starts_with("update_interval:")
                || s.starts_with("i2c_id:")
                || s.starts_with("spi_id:")
                || s.starts_with("uart_id:")
                || s.starts_with("GPIO")
            {
                c.refs += 1;
                continue;
            }
            if s.starts_with("on_")
                || s.starts_with("then:")
                || s.starts_with("lambda:")
                || s.contains("script.execute")
                || s.starts_with("automation:")
            {
                c.automations += 1;
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
            "esphome:\n",
            "  name: node1\n",
            "  friendly_name: Node One\n",
            "esp32:\n",
            "  board: esp32dev\n",
            "wifi:\n",
            "  ssid: !secret wifi_ssid\n",
            "  password: !secret wifi_pass\n",
            "api:\n",
            "ota:\n",
            "logger:\n",
            "i2c:\n",
            "  sda: GPIO21\n",
            "  scl: GPIO22\n",
            "sensor:\n",
            "  - platform: dht\n",
            "    pin: GPIO4\n",
            "    update_interval: 60s\n",
            "    name: Temp\n",
            "  - platform: bme280\n",
            "    address: 0x76\n",
            "binary_sensor:\n",
            "  - platform: gpio\n",
            "    pin: GPIO5\n",
            "    name: Motion\n",
            "    on_press:\n",
            "      then:\n",
            "        - light.toggle: l1\n",
            "light:\n",
            "  - platform: binary\n",
            "    id: l1\n",
            "    output: out1\n",
            "output:\n",
            "  - platform: gpio\n",
            "    pin: GPIO12\n",
            "    id: out1\n",
            "web_server:\n",
            "  port: 80\n",
        );
        let c = Esphome::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 12);
        assert_eq!(c.platforms, 5);
        assert_eq!(c.names, 5);
        assert_eq!(c.refs, 5);
        assert_eq!(c.automations, 2);
        assert_eq!(c.secrets, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Esphome::parse(b"foo = 1").is_none());
    }
}
