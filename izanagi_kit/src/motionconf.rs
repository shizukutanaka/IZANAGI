//! Motion `motion.conf`/`camera*.conf`/`thread*.conf` census.
//!
//! The Motion daemon's config is whitespace-separated
//! `key value` lines (older releases used `key=value`, both are
//! counted), `#` comments, and `thread <file>` includes for
//! per-camera overrides. Well-known keys include `daemon`/
//! `videodevice`/`netcam_url`/`width`/`height`/`framerate`/
//! `threshold`/`lightswitch`/`minimum_motion_frames`/`event_gap`/
//! `pre_capture`/`post_capture`/`output_pictures`/`picture_type`/
//! `ffmpeg_output_movies`/`ffmpeg_video_codec`/`movie_filename`/
//! `jpeg_filename`/`snapshot_filename`/`target_dir`/
//! `webcontrol_port`/`stream_port`/`stream_authentication`/
//! `text_left`/`text_right`/`text_changes`/`locate_motion_mode`/
//! `mask_file`/`mask_privacy`/`area_detect`/`on_event_start`/
//! `on_picture_save`/`on_movie_end`/`quiet`/`log_level`/`logfile`/
//! `rotate`/`flip_axis`/`despeckle_filter`/`smart_mask_speed`/
//! `database_type`/`sql_*`/`track_*`/`camera_name`/`camera_id`/
//! `emulate_motion`.
//!
//! ```rust
//! let c = izanagi_kit::motionconf::Motionconf::parse(b"daemon off\nvideodevice /dev/video0\nwidth 640\n").unwrap();
//! assert_eq!(c.entries, 3);
//! ```

use crate::textutil::strip_bom;
/// motion.conf census.
#[derive(Debug, Clone)]
pub struct Motionconf {
    /// `key value`/`key=value` option lines.
    pub entries: usize,
    /// Options matching the known Motion option list.
    pub named: usize,
    /// `thread <file>` includes.
    pub threads: usize,
    /// `#`/`;` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "daemon",
    "setup_mode",
    "logfile",
    "log_level",
    "log_type",
    "videodevice",
    "v4l2_device",
    "vid_control_params",
    "power_line_frequency",
    "auto_brightness",
    "brightness",
    "contrast",
    "saturation",
    "hue",
    "width",
    "height",
    "framerate",
    "minimum_frame_time",
    "netcam_url",
    "netcam_high_url",
    "netcam_proxy",
    "netcam_userpass",
    "netcam_keepalive",
    "rtsp_uses_tcp",
    "rotate",
    "flip_axis",
    "locate_motion_mode",
    "locate_motion_style",
    "text_left",
    "text_right",
    "text_changes",
    "text_scale",
    "text_event",
    "text_double",
    "despeckle_filter",
    "area_detect",
    "area_max",
    "mask_file",
    "mask_privacy",
    "smart_mask_speed",
    "lightswitch",
    "minimum_motion_frames",
    "event_gap",
    "pre_capture",
    "post_capture",
    "max_movie_time",
    "output_pictures",
    "output_debug_pictures",
    "quality",
    "picture_type",
    "exif_text",
    "ffmpeg_output_movies",
    "ffmpeg_timelapse",
    "ffmpeg_bps",
    "ffmpeg_variable_bitrate",
    "ffmpeg_video_codec",
    "ffmpeg_passthrough",
    "movie_filename",
    "timelapse_filename",
    "snapshot_filename",
    "jpeg_filename",
    "snapshot_interval",
    "webcontrol_port",
    "webcontrol_localhost",
    "webcontrol_interface",
    "stream_port",
    "stream_localhost",
    "stream_maxrate",
    "stream_auth_method",
    "stream_authentication",
    "target_dir",
    "on_event_start",
    "on_event_end",
    "on_picture_save",
    "on_movie_start",
    "on_movie_end",
    "on_camera_lost",
    "on_motion_detected",
    "quiet",
    "database_type",
    "database_dbname",
    "database_host",
    "database_user",
    "database_password",
    "sql_log_picture",
    "sql_log_snapshot",
    "sql_log_movie",
    "sql_log_timelapse",
    "sql_query_start",
    "sql_query",
    "track_type",
    "track_auto",
    "track_port",
    "track_motorx",
    "track_motory",
    "track_maxx",
    "track_maxy",
    "camera_id",
    "camera_name",
    "camera_dir",
    "emulate_motion",
    "threshold",
    "threshold_maximum",
    "threshold_tune",
    "noise_level",
    "noise_tune",
];

fn key_of(s: &str) -> &str {
    s.split([' ', '=', '\t']).next().unwrap_or("")
}

/// Whether the buffer looks like a motion.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.lines()
        .filter(|l| {
            let s = l.trim_start();
            !s.is_empty() && !s.starts_with('#') && !s.starts_with(';') && KEYS.contains(&key_of(s))
        })
        .count()
        >= 3
}

impl Motionconf {
    /// Parse a motion.conf into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            entries: 0,
            named: 0,
            threads: 0,
            comments: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') || s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            let key = key_of(s);
            if key.is_empty() {
                continue;
            }
            c.entries += 1;
            if key == "thread" {
                c.threads += 1;
            }
            if KEYS.contains(&key) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_motion_conf() {
        let b = concat!(
            "# motion.conf\n",
            "daemon off\n",
            "videodevice /dev/video0\n",
            "width 640\n",
            "height 480\n",
            "framerate 15\n",
            "threshold 1500\n",
            "target_dir /var/lib/motion\n",
            "thread /etc/motion/camera1.conf\n",
            "on_picture_save /home/u/save.sh\n",
        );
        let c = Motionconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.entries, 9);
        assert_eq!(c.named, 8);
        assert_eq!(c.threads, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Motionconf::parse(b"alpha 1\nbeta 2\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
