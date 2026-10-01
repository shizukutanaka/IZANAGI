//! Klipper 3D プリンタ設定 (`printer.cfg`) の検出・カウント。
//!
//! INI 風だがセクション名に空白を含み (`[stepper x]`)、代入は `key: value`。
//! ピン名は `PA0`/`PC14`/`!PF5`/`^PD1`/`ar0`/`exp1` のようなエイリアス形を取る。
//!
//! ```
//! let cfg = b"[stepper x]\nstep_pin: PA0\ndir_pin: !PB5\nenable_pin: !PC2\n\
//!             rotation_distance: 40\n\n[gcode_macro G29]\ngcode:\n  G28\n";
//! assert!(izanagi_kit::klipperconf::detect(cfg));
//! let c = izanagi_kit::klipperconf::parse(cfg).unwrap();
//! assert_eq!(c.sections, 2);
//! assert_eq!(c.section_entries, 5);
//! assert_eq!(c.pin_entries, 3);
//! assert_eq!(c.macro_sections, 1);
//! ```

/// セクション先頭語として知られる Klipper モジュール名。
const KNOWN_SECTIONS: &[&str] = &[
    "stepper",
    "extruder",
    "heater_bed",
    "heater_generic",
    "fan",
    "temperature_fan",
    "temperature_sensor",
    "temperature_probe",
    "probe",
    "bltouch",
    "smart_effector",
    "servo",
    "gcode_macro",
    "delayed_gcode",
    "gcode_arcs",
    "safe_z_home",
    "homing_override",
    "delta_calibrate",
    "bed_mesh",
    "bed_tilt",
    "z_tilt",
    "quad_gantry_level",
    "screws_tilt_adjust",
    "skew_correction",
    "axis_twist_compensation",
    "input_shaper",
    "resonance_tester",
    "adxl345",
    "mpu9250",
    "lis2dw",
    "lis2dh",
    "angle",
    "printer",
    "mcu",
    "board_pins",
    "display",
    "display_data",
    "display_glyph",
    "display_template",
    "hd44780",
    "hd44780_spi",
    "uc1701",
    "ssd1306",
    "st7920",
    "neopixel",
    "dotstar",
    "pca9533",
    "pca9632",
    "led",
    "filament_switch_sensor",
    "filament_motion_sensor",
    "hall_filament_width_sensor",
    "tsl1401cl_filament_width_sensor",
    "pause_resume",
    "virtual_sdcard",
    "sdcard_loop",
    "force_move",
    "manual_probe",
    "manual_stepper",
    "nozzle_clean",
    "output_pin",
    "pwm_tool",
    "pwm_cycle_time",
    "static_digital_output",
    "multi_pin",
    "duplicate_pin_override",
    "verify_heater",
    "verify_pin",
    "respond",
    "idle_timeout",
    "exclude_object",
    "endstop_phase",
    "fan_generic",
    "heater_fan",
    "controller_fan",
    "gcode_button",
    "load_cell",
    "mcp4451",
    "sx1509",
    "z_calibration",
    "firmware_retraction",
    "save_variables",
    "config_status",
    "shaper_calibrate",
];

/// ステッパドライバ系セクションの接頭辞 (`[tmc2209 stepper_x]` 等)。
fn is_driver_section(name: &str) -> bool {
    name.starts_with("tmc")
        || name.starts_with("a4988")
        || name.starts_with("drv8825")
        || name.starts_with("st820")
        || name.starts_with("t51x1")
        || name.starts_with("aht10")
}

/// ピン値らしさ (`PA0`, `PC14`, `!PF5`, `^PD1`, `gpio15`, `mcu:PA0`, `ar0`)。
fn is_pin(v: &str) -> bool {
    let v = v
        .trim()
        .trim_start_matches(['!', '^', '~'])
        .trim_end_matches(['!', '^', '~']);
    let v = v.rsplit(':').next().unwrap_or(v);
    if v.is_empty() {
        return false;
    }
    if v.starts_with("gpio") {
        return true;
    }
    let b = v.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    i > 0 && i < b.len() && b[i].is_ascii_digit() && b[i + 1..].iter().all(|c| c.is_ascii_digit())
}

fn key_of(s: &str) -> Option<(&str, &str)> {
    let i = s.find(':')?;
    let k = s[..i].trim();
    if k.is_empty() || !k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
        return None;
    }
    Some((k, s[i + 1..].trim()))
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Klipper モジュール型を持つセクション数。
    pub sections: usize,
    /// セクション内の `key: value` 行数。
    pub section_entries: usize,
    /// 値がピンエイリアス形の行数。
    pub pin_entries: usize,
    /// ステッパドライバ系セクション数 (`[tmc2209 …]` 等)。
    pub driver_sections: usize,
    /// `[gcode_macro …]` セクション数。
    pub macro_sections: usize,
    /// 真偽値 (`True`/`False`/`true`/`false`) の行数。
    pub bool_entries: usize,
    /// `#`/`;` コメント行数。
    pub comments: usize,
}

/// `b` が Klipper `printer.cfg` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.sections >= 2 && c.section_entries >= 2
}

/// `b` を Klipper `printer.cfg` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        section_entries: 0,
        pin_entries: 0,
        driver_sections: 0,
        macro_sections: 0,
        bool_entries: 0,
        comments: 0,
    };
    let mut in_known = false;
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') || s.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if s.starts_with('[') {
            let inner = s
                .strip_prefix('[')
                .unwrap_or(s)
                .split(']')
                .next()
                .unwrap_or("")
                .trim();
            let head = inner.split(' ').next().unwrap_or("");
            in_known = KNOWN_SECTIONS.contains(&head) || is_driver_section(head);
            if in_known {
                c.sections += 1;
                if is_driver_section(head) {
                    c.driver_sections += 1;
                }
                if head == "gcode_macro" {
                    c.macro_sections += 1;
                }
            }
            continue;
        }
        if !in_known {
            continue;
        }
        if let Some((_k, v)) = key_of(s) {
            c.section_entries += 1;
            if is_pin(v) {
                c.pin_entries += 1;
            }
            if matches!(v, "True" | "False" | "true" | "false") {
                c.bool_entries += 1;
            }
        }
    }
    if c.sections == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_printer_cfg() {
        let cfg = b"[stepper x]\nstep_pin: PC13\ndir_pin: PC0\nenable_pin: !PC1\n\
                    microsteps: 16\nrotation_distance: 8\n\n[extruder]\n\
                    step_pin: PB9\nnozzle_filament_diameter: 1.750\nmax_temp: 270\n\
                    \n[tmc2209 stepper_x]\nuart_pin: PA10\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.sections, 3);
        assert_eq!(c.driver_sections, 1);
        assert_eq!(c.pin_entries, 5);
    }

    #[test]
    fn rejects_plain_ini() {
        assert!(!detect(
            b"[database]\nhost: localhost\nport: 5432\n[cache]\nttl: 60\n"
        ));
        assert!(!detect(b"hello"));
    }
}
