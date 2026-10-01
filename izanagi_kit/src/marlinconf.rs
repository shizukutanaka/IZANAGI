//! Marlin ファームウェア `Configuration.h`/`Configuration_adv.h` の検出・カウント。
//!
//! C プリプロセッサ形式で、有効な `#define OPTION value` と、
//! `//#define DISABLED_OPTION` のようにコメントアウトされた無効オプションが混在する。
//!
//! ```
//! let cfg = b"// Marlin Config\n#define MOTHERBOARD BOARD_BTT_SKR_V1_4\n#define SERIAL_PORT 0\n\
//!             #define BAUDRATE 250000\n#define TEMP_SENSOR_0 1\n#define X_DRIVER_TYPE A4988\n\
//!             //#define EEPROM_SETTINGS\n#define SDSUPPORT\n";
//! assert!(izanagi_kit::marlinconf::detect(cfg));
//! let c = izanagi_kit::marlinconf::parse(cfg).unwrap();
//! assert_eq!(c.defines, 6);
//! assert_eq!(c.disabled_defines, 1);
//! assert_eq!(c.flag_defines, 1);
//! assert_eq!(c.known_options, 6);
//! ```

/// Marlin で代表的なオプション名。
const KNOWN_OPTIONS: &[&str] = &[
    "MOTHERBOARD",
    "SERIAL_PORT",
    "SERIAL_PORT_2",
    "BAUDRATE",
    "BAUDRATE_2",
    "EXTRUDERS",
    "PRINTRBOARD",
    "DUMMY_THERMISTOR_998_VALUE",
    "SHOW_TEMP_ADC_VALUES",
    "MAX_CONSECUTIVE_LOW_TEMPERATURE_ERROR_ALLOWED",
    "MILLISECONDS_PREHEAT_TIME",
    "PIDTEMP",
    "PIDTEMPBED",
    "PIDTEMPCHAMBER",
    "MPCTEMP",
    "BANG_MAX",
    "PID_MAX",
    "PID_K1",
    "PID_FUNCTIONAL_RANGE",
    "PID_EDIT_MENU",
    "PID_AUTOTUNE_MENU",
    "PID_PARAMS_PER_HOTEND",
    "PREVENT_COLD_EXTRUSION",
    "EXTRUDE_MINTEMP",
    "PREVENT_LENGTHY_EXTRUDE",
    "EXTRUDE_MAXLENGTH",
    "THERMAL_PROTECTION_PERIOD",
    "THERMAL_PROTECTION_HYSTERESIS",
    "ADAPTIVE_FAN_SLOWING",
    "REPORT_FAN_CHANGE",
    "EEPROM_SETTINGS",
    "EEPROM_AUTO_INIT",
    "EEPROM_INIT_NOW",
    "EEPROM_CHITCHAT",
    "DISABLE_M503",
    "SDSUPPORT",
    "SD_PROCEDURE_DEPTH",
    "SDIO_SUPPORT",
    "SD_CHECK_AND_RETRY",
    "USB_FLASH_DRIVE_SUPPORT",
    "USE_OTG_USB_HOST",
    "MEDIA_MENU_AT_START",
    "BINARY_FILE_TRANSFER",
    "CUSTOM_FIRMWARE_UPLOAD",
    "POWER_LOSS_RECOVERY",
    "BACKUP_POWER_SUPPLY",
    "POWER_OFF_DELAY",
    "LONG_FILENAME_HOST_SUPPORT",
    "SCROLL_LONG_FILENAMES",
    "SD_ABORT_ON_ENDSTOP_HIT",
    "SDCARD_READONLY",
    "LCD_LANGUAGE",
    "LCD_INFO_MENU",
    "ENCODER_PULSES_PER_STEP",
    "ENCODER_STEPS_PER_MENU_ITEM",
    "REVERSE_ENCODER_DIRECTION",
    "REVERSE_MENU_DIRECTION",
    "SPEAKER",
    "SOUND_MENU_ITEM",
    "LCD_FEEDBACK_FREQUENCY_DURATION_MS",
    "LCD_FEEDBACK_FREQUENCY_HZ",
    "NEOPIXEL_LED",
    "NEOPIXEL_TYPE",
    "NEOPIXEL_PIN",
    "NEOPIXEL_PIXELS",
    "NEOPIXEL_BRIGHTNESS",
    "RGB_LED",
    "RGBW_LED",
    "BLINKM",
    "PRINTER_EVENT_LEDS",
    "STATUS_MESSAGE_SCROLLING",
    "INDIVIDUAL_AXIS_HOMING_MENU",
    "NOZZLE_PARK_FEATURE",
    "NOZZLE_PARK_POINT",
    "PARK_HEAD_ON_PAUSE",
    "ADVANCED_PAUSE_FEATURE",
    "PAUSE_PARK_RETRACT_LENGTH",
    "PAUSE_PARK_NOZZLE_TIMEOUT",
    "FILAMENT_RUNOUT_SENSOR",
    "FILAMENT_RUNOUT_DISTANCE_MM",
    "FILAMENT_MOTION_SENSOR",
    "FWRETRACT",
    "FWRETRACT_AUTORETRACT",
    "MIN_AUTORETRACT",
    "MAX_AUTORETRACT",
    "RETRACT_LENGTH",
    "RETRACT_FEEDRATE",
    "RETRACT_ZRAISE",
    "RETRACT_RECOVER_LENGTH",
    "RETRACT_MIN_TRAVEL",
    "LIN_ADVANCE",
    "S_CURVE_ACCELERATION",
    "MULTIPLE_PROBING",
    "EXTRA_PROBING",
    "PROBING_FANS_OFF",
    "PROBING_ESTEPPERS_OFF",
    "PROBING_STEPPERS_OFF",
    "DELAY_BEFORE_PROBING",
    "PREHEAT_BEFORE_LEVELING",
    "LEVELING_NOZZLE_TEMP",
    "LEVELING_BED_TEMP",
    "DEBUG_LEVELING_FEATURE",
    "G26_MESH_VALIDATION",
    "Z_MIN_PROBE_REPEATABILITY_TEST",
    "Z_PROBE_LOW_POINT",
    "Z_MIN_PROBE_USES_Z_MIN_ENDSTOP_PIN",
    "TOUCH_MI_PROBE",
    "FIX_MOUNTED_PROBE",
    "NOZZLE_AS_PROBE",
    "BLTOUCH",
    "BLTOUCH_DELAY",
    "BLTOUCH_HS_MODE",
    "MAG_MOUNTED_PROBE",
    "PROBE_ACTIVATION_SWITCH",
    "Z_PROBE_SLED",
    "X_PROBE_OFFSET_FROM_EXTRUDER",
    "Y_PROBE_OFFSET_FROM_EXTRUDER",
    "Z_PROBE_OFFSET_FROM_EXTRUDER",
    "MIN_PROBE_EDGE",
    "XY_PROBE_FEEDRATE",
    "Z_PROBE_FEEDRATE_FAST",
    "Z_PROBE_FEEDRATE_SLOW",
    "Z_CLEARANCE_DEPLOY_PROBE",
    "Z_CLEARANCE_BETWEEN_PROBES",
    "Z_CLEARANCE_MULTI_PROBE",
    "Z_AFTER_PROBING",
    "LEVEL_BED_CORNERS",
    "LEVEL_CENTER_TOO",
    "BED_TRAMMING_INCLUDE_CENTER",
    "BED_TRAMMING_USE_PROBE",
    "BED_TRAMMING_LEVELING_ORDER",
    "BED_TRAMMING_PROBE_TOLERANCE",
    "ASSISTED_TRAMMING",
    "RESTORE_LEVELING_AFTER_G28",
    "ENABLE_LEVELING_AFTER_G28",
    "ENABLE_LEVELING_FADE_HEIGHT",
    "SEGMENT_LEVELED_MOVES",
    "LEVELED_SEGMENT_LENGTH",
    "MESH_BED_LEVELING",
    "MESH_INSET",
    "MESH_EDIT_MENU",
    "LCD_BED_LEVELING",
    "MBL_Z_STEP",
    "LCD_BED_TRAMMING",
    "MESH_FIRST_LAYER",
    "G29_RETRY_AND_RECOVER",
    "G29_MAX_RETRIES",
    "G29_HALT_ON_FAILURE",
    "HOMING_BUMP_MM",
    "HOMING_BUMP_DIVISOR",
    "VALIDATE_HOMING_ENDSTOPS",
    "HOMING_BACKOFF_POST_MM",
    "QUICK_HOME",
    "HOME_Y_BEFORE_X",
    "CODEPENDENT_XY_HOMING",
    "Z_SAFE_HOMING",
    "Z_SAFE_HOMING_X_POINT",
    "Z_SAFE_HOMING_Y_POINT",
    "X_HOME_DIR",
    "Y_HOME_DIR",
    "Z_HOME_DIR",
    "X_ENABLE_ON",
    "Y_ENABLE_ON",
    "Z_ENABLE_ON",
    "E_ENABLE_ON",
    "DISABLE_X",
    "DISABLE_Y",
    "DISABLE_Z",
    "DISABLE_E",
    "DISABLE_INACTIVE_EXTRUDER",
    "X_MIN_POS",
    "Y_MIN_POS",
    "Z_MIN_POS",
    "X_MAX_POS",
    "Y_MAX_POS",
    "Z_MAX_POS",
    "X_BED_SIZE",
    "Y_BED_SIZE",
    "MIN_SOFTWARE_ENDSTOPS",
    "MAX_SOFTWARE_ENDSTOPS",
    "SOFT_ENDSTOPS_MENU_ITEM",
    "ENDSTOP_INTERRUPTS_FEATURE",
    "ENDSTOP_NOISE_THRESHOLD",
    "ENDSTOPPULLUPS",
    "ENDSTOPPULLDOWNS",
    "COREXY",
    "COREXZ",
    "COREYZ",
    "MARKFORGED_XY",
    "DELTA",
    "DELTA_RADIUS",
    "DELTA_DIAGONAL_ROD",
    "DELTA_HEIGHT",
    "DELTA_AUTO_CALIBRATION",
    "DELTA_CALIBRATION_RADIUS",
    "DELTA_PRINTABLE_RADIUS",
    "SCARA",
    "MP_SCARA",
    "AXEL_TPARA",
    "TPARA_ROBOT_ARM",
    "POLARGRAPH",
    "V_PLOTTER",
    "SPINDLE_FEATURE",
    "SPINDLE_LASER_USE_PWM",
    "SPINDLE_LASER_FREQUENCY",
    "LASER_FEATURE",
    "COOLANT_CONTROL",
    "COOLANT_MIST",
    "COOLANT_FLOOD",
    "FILAMENT_WIDTH_SENSOR",
    "FILAMENT_LCD_DISPLAY",
    "DEFAULT_NOMINAL_FILAMENT_DIA",
    "PRUSA_MMU1",
    "PRUSA_MMU2",
    "MMU2_SERIAL_PORT",
    "MMU2_MENUS",
    "GCODE_MACROS",
    "CUSTOM_USER_MENUS",
    "CANCEL_OBJECTS",
    "SAVED_POSITIONS",
    "PRINTCOUNTER",
    "PRINTCOUNTER_SYNC",
    "PASSWORD_FEATURE",
    "PASSWORD_LENGTH",
    "PASSWORD_ON_STARTUP",
    "HOST_ACTION_COMMANDS",
    "HOST_KEEPALIVE_FEATURE",
    "DEFAULT_KEEPALIVE_INTERVAL",
    "KEEPALIVE_INTERVAL",
    "BUSY_WHILE_HEATING",
    "EMERGENCY_PARSER",
    "SERIAL_XON_XOFF",
    "ADVANCED_OK",
    "MAX_CMD_SIZE",
    "BUFSIZE",
    "TX_BUFFER_SIZE",
    "RX_BUFFER_SIZE",
    "SERIAL_STATS_MAX_RX",
    "SERIAL_STATS_DROPPED_RX",
    "SERIAL_STATS_RX_BUFFER_OVERRUNS",
    "SERIAL_STATS_RX_FRAMING_ERRORS",
    "SERIAL_STATS_MAX_TX",
    "CUSTOM_MENDEL_NAME",
    "MACHINE_NAME",
    "CUSTOM_MACHINE_NAME",
    "MACHINE_UUID",
    "SUICIDE_PIN",
    "KILL_PIN",
    "PSU_CONTROL",
    "PSU_NAME",
    "PSU_ACTIVE_HIGH",
    "PSU_DEFAULT_OFF",
    "PSU_ALWAYS_ON",
    "AUTO_POWER_CONTROL",
    "AUTO_POWER_FANS",
    "POWER_TIMEOUT",
    "WATCH_ALL_AXES",
    "USE_WATCHDOG",
    "THERMAL_PROTECTION_HOTENDS",
    "THERMAL_PROTECTION_BED",
    "THERMAL_PROTECTION_CHAMBER",
    "THERMAL_PROTECTION_COOLER",
    "EXTRUDER_RUNOUT_PREVENT",
    "CASE_LIGHT_ENABLE",
    "CASE_LIGHT_PIN",
    "CASE_LIGHT_DEFAULT_ON",
    "CASE_LIGHT_MENU",
    "POWER_SUPPLY",
    "X_DUAL_STEPPER_DRIVERS",
    "Y_DUAL_STEPPER_DRIVERS",
    "Z_DUAL_STEPPER_DRIVERS",
    "Z_MULTI_STEPPER_DRIVERS",
    "Z_STEPPER_AUTO_ALIGN",
    "CLASSIC_JERK",
    "JUNCTION_DEVIATION_MM",
    "HYBRID_THRESHOLD",
    "STEALTHCHOP_XY",
    "STEALTHCHOP_Z",
    "STEALTHCHOP_I",
    "STEALTHCHOP_J",
    "STEALTHCHOP_K",
    "STEALTHCHOP_E",
    "TMC_DEBUG",
    "MONITOR_DRIVER_STATUS",
    "CHOPPER_TIMING",
    "INTERPOLATE",
    "EDGE_STEPPING",
    "HOLD_MULTIPLIER",
    "GCODE_MOTION_MODES",
    "ARC_SUPPORT",
    "BEZIER_CURVE_SUPPORT",
    "CORKSCREW_RETRACTS",
    "NANODLP_Z_SYNC",
    "EXTENDED_CAPABILITIES_REPORT",
    "AUTO_REPORT_TEMPERATURES",
    "AUTO_REPORT_POSITION",
    "M114_DETAIL",
    "M114_REALTIME",
    "REPORT_REAL_E",
    "CAPABILITIES_REPORT",
    "MECHANICAL_GANTRY_CALIBRATION",
    "USER_GCODE_DRIVE_POWER",
    "CONTROLLER_FAN_EDITABLE",
    "CONTROLLER_FAN",
    "CONTROLLERFAN_IDLE_TIME",
    "FAST_PWM_FAN",
    "FAN_SOFT_PWM",
    "FAN_KICKSTART_TIME",
    "FAN_MIN_PWM",
    "FAN_MAX_PWM",
    "EXTRA_FAN_SPEED",
    "USE_CONTROLLER_FAN",
    "IDLE_OOZING_PREVENT",
    "XY_FREQUENCY_LIMIT",
    "XY_FREQUENCY_MIN_PERCENT",
    "MAX7219_DEBUG",
    "DWIN_CREALITY_LCD",
    "DWIN_LCD_PROUI",
    "CR10_STOCKDISPLAY",
    "RET6_12864_LCD",
    "ANET_FULL_GRAPHICS_LCD",
    "ANYCUBIC_LCD_CHIRON",
    "ANYCUBIC_LCD_I3MEGA",
    "ANYCUBIC_TFT_MODEL",
    "REPRAP_DISCOUNT_SMART_CONTROLLER",
    "REPRAP_DISCOUNT_FULL_GRAPHIC_SMART_CONTROLLER",
    "ULTIPANEL",
    "PANEL_ONE",
    "MAKRPANEL",
    "ELB_FULL_GRAPHIC_CONTROLLER",
    "MINIPANEL",
    "ORIGIN_FULL_GRAPHIC_SMART_CONTROLLER",
    "BQ_LCD_SMART_CONTROLLER",
    "CARTESIO_UI",
    "DWIN_MARLINUI_PORTRAIT",
    "DWIN_MARLINUI_LANDSCAPE",
    "TOUCH_SCREEN",
    "TFT_GENERIC",
    "TFT_CLASSIC_UI",
    "TFT_COLOR_UI",
    "FSMC_GRAPHICAL_TFT",
    "SPI_GRAPHICAL_TFT",
    "MKS_TS35_V2_0",
    "MKS_ROBIN_TFT24",
    "MKS_ROBIN_TFT28",
    "MKS_ROBIN_TFT32",
    "MKS_ROBIN_TFT35",
    "MKS_ROBIN_TFT43",
    "MKS_ROBIN_TFT_V1_1R",
    "DGUS_LCD_UI_ORIGIN",
    "DGUS_LCD_UI_FYSETC",
    "DGUS_LCD_UI_HIPRECY",
    "DGUS_LCD_UI_MKS",
    "DGUS_LCD_UI_RELOADED",
    "TFT_TRONXY_X5SA",
    "ANYCUBIC_TFT35",
    "LONGER_LK_TFT28",
    "BIQU_BX_TFT70",
    "BTT_MINI_12864",
    "FYSETC_MINI_12864_2_1",
    "MKS_MINI_12864",
    "CR10_12864",
    "ENDER2_STOCKDISPLAY",
    "U8GLIB_SH1106",
    "U8GLIB_SSD1306",
    "U8GLIB_ST7920",
    "U8GLIB_SH1106_EWYHLP",
    "ZONESTAR_12864LCD",
    "ZONESTAR_12864OLED",
    "SILVER_GATE_GLCD_CONTROLLER",
    "OVERLORD_OLED",
    "KS0713",
    "EXTENSIBLE_UI",
    "DGUS_LCD_UI",
    "TOUCH_UI",
    "GENERIC_THERMISTOR_0",
    "MAX31865_SENSOR_OHMS_0",
    "MAX31865_CALIBRATION_OHMS_0",
    "DHT_SENSOR",
    "HEATER_0_MINTEMP",
    "HEATER_0_MAXTEMP",
    "HEATER_1_MAXTEMP",
    "HEATER_2_MAXTEMP",
    "HEATER_3_MAXTEMP",
    "HEATER_4_MAXTEMP",
    "HEATER_5_MAXTEMP",
    "HEATER_6_MAXTEMP",
    "HEATER_7_MAXTEMP",
    "BED_MINTEMP",
    "BED_MAXTEMP",
    "CHAMBER_MINTEMP",
    "CHAMBER_MAXTEMP",
    "HOTEND_OVERSHOOT",
    "BED_OVERSHOOT",
    "CHAMBER_OVERSHOOT",
    "COOLER_OVERSHOOT",
];

/// サフィックス/プレフィックス規則による既知オプション判定。
fn is_known_option(name: &str) -> bool {
    KNOWN_OPTIONS.contains(&name)
        || name.ends_with("_ENDSTOP_INVERTING")
        || name.ends_with("_DRIVER_TYPE")
        || name.ends_with("_AUTO_FAN_PIN")
        || name.ends_with("_FIL_RUNOUT_PIN")
        || name.ends_with("_FIL_RUNOUT_STATE")
        || name.ends_with("_FAN_PIN")
        || name.ends_with("_PROBE_PIN")
        || name.ends_with("_SERVO_PIN")
        || name.ends_with("_DIR_PIN")
        || name.ends_with("_STEP_PIN")
        || name.ends_with("_ENABLE_PIN")
        || name.ends_with("_CS_PIN")
        || name.ends_with("_MAX_CURRENT")
        || name.ends_with("_HOME_BUMP_MM")
        || name.ends_with("_AXIS_STEPS_PER_UNIT")
        || (name.starts_with("USE_") && name.ends_with("_PLUG"))
        || name.starts_with("TEMP_SENSOR_")
        || name.starts_with("MIN_SOFTWARE_ENDSTOP_")
        || name.starts_with("MAX_SOFTWARE_ENDSTOP_")
        || name.starts_with("DEFAULT_MAX_")
        || name.starts_with("DEFAULT_AXIS_")
        || name.starts_with("DEFAULT_") && name.ends_with("JERK")
        || name.starts_with("DEFAULT_") && name.ends_with("ACCELERATION")
        || name.starts_with("PREHEAT_")
        || name.starts_with("AUTO_BED_LEVELING_")
        || name.starts_with("THERMAL_PROTECTION_")
        || name.starts_with("PROBE_PT_")
        || name.starts_with("INVERT_") && name.ends_with("_DIR")
        || name.starts_with("FIL_RUNOUT")
        || name.starts_with("X_DUAL_")
        || name.starts_with("Y_DUAL_")
        || name.starts_with("Z_DUAL_")
        || name.starts_with("E0_")
        || name.starts_with("E1_")
        || name.starts_with("E2_")
        || name.starts_with("TMC_")
        || name.starts_with("HAVE_TMC")
        || name.starts_with("HAVE_L64")
        || name.starts_with("MKS_")
        || name.starts_with("DWIN_")
        || name.starts_with("DGUS_")
}

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 有効な `#define` 行数。
    pub defines: usize,
    /// `//#define` のようにコメントアウトされたオプション数。
    pub disabled_defines: usize,
    /// 値を伴わないフラグ型 `#define` 数。
    pub flag_defines: usize,
    /// Marlin の既知オプション名を持つ define 数。
    pub known_options: usize,
    /// 数値値を持つ define 数。
    pub numeric_values: usize,
    /// `"..."` 文字列値を持つ define 数。
    pub string_values: usize,
    /// その他の `//` コメント行数 (`//#define` 除く)。
    pub comments: usize,
}

/// `b` が Marlin `Configuration*.h` 形式かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.defines + c.disabled_defines >= 4 && c.known_options >= 3
}

/// `b` を Marlin `Configuration.h` として解析し、行種別をカウントする。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        defines: 0,
        disabled_defines: 0,
        flag_defines: 0,
        known_options: 0,
        numeric_values: 0,
        string_values: 0,
        comments: 0,
    };
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        if s.starts_with("//") {
            let rest = s.strip_prefix("//").unwrap_or(s).trim_start();
            if rest.starts_with("#define") {
                c.disabled_defines += 1;
            } else {
                c.comments += 1;
            }
            continue;
        }
        let Some(rest) = s.strip_prefix("#define") else {
            if s.starts_with('#') || s.starts_with("/*") || s.starts_with('*') {
                c.comments += 1;
            }
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        let mut it = rest.splitn(2, [' ', '\t']);
        let name = it.next().unwrap_or("");
        let val = it.next().unwrap_or("").trim();
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            continue;
        }
        c.defines += 1;
        if is_known_option(name) {
            c.known_options += 1;
        }
        if val.is_empty() {
            c.flag_defines += 1;
        } else {
            if val.starts_with('"') {
                c.string_values += 1;
            } else {
                let v = val.trim_start_matches(['-', '+']);
                if !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit() || c == b'.') {
                    c.numeric_values += 1;
                }
            }
        }
    }
    if c.defines == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_configuration() {
        let cfg = b"#define MOTHERBOARD BOARD_CREALITY_V4\n#define SERIAL_PORT 1\n\
                    #define BAUDRATE 115200\n#define EXTRUDERS 1\n\
                    #define X_MIN_ENDSTOP_INVERTING false\n#define E0_DRIVER_TYPE TMC2209\n\
                    //#define POWER_LOSS_RECOVERY\n#define NOZZLE_PARK_FEATURE\n";
        assert!(detect(cfg));
        let c = parse(cfg).unwrap();
        assert_eq!(c.defines, 7);
        assert_eq!(c.disabled_defines, 1);
        assert_eq!(c.known_options, 7);
        assert_eq!(c.numeric_values, 3);
        assert_eq!(c.flag_defines, 1);
    }

    #[test]
    fn rejects_generic_c_header() {
        assert!(!detect(
            b"#define MAX 100\n#define MIN 1\n#define DEBUG\n#define SIZE 10\n"
        ));
    }
}
