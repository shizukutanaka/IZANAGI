//! Frigate NVR `config.yml` census.
//!
//! Sections: `mqtt:` (host/user/password/topic_prefix/client_id/stats_interval),
//! `cameras:` with per-camera `ffmpeg:` `inputs:` `- path:`/`roles:`,
//! `detect:`/`record:`/`snapshots:`/`objects:`/`zones:`/`review:`/`motion:`/
//! `birdseye:`/`go2rtc:`/`live:`/`audio:`/`timestamp_style:`/`onvif:`/`ptz:`,
//! `detectors:`/`model:`/`database:`/`environment_vars:`/`ui:`/`proxy:`/
//! `auth:`/`tls:`/`telemetry:`/`version:`/`check_for_updates:`/`logger:`/
//! `snapshots:`/`birdseye:`/`genai:`/`semantic_search:`/`face_recognition:`/
//! `lpr:`/`classification:`/`ffmpeg:`/`detect:`/`record:`/`audio:`/`notifications:`.
//!
//! ```rust
//! let y = concat!(
//!     "mqtt:\n",
//!     "  host: localhost\n",
//!     "cameras:\n",
//!     "  porch:\n",
//!     "    ffmpeg:\n",
//!     "      inputs:\n",
//!     "        - path: rtsp://cam/stream\n",
//!     "          roles:\n",
//!     "            - detect\n",
//! );
//! let c = izanagi_kit::frigate::Frigate::parse(y.as_bytes()).unwrap();
//! assert_eq!(c.cameras, 1);
//! ```

const SECTIONS: &[&str] = &[
    "mqtt",
    "cameras",
    "detectors",
    "model",
    "database",
    "environment_vars",
    "ui",
    "proxy",
    "auth",
    "tls",
    "telemetry",
    "version",
    "check_for_updates",
    "logger",
    "go2rtc",
    "birdseye",
    "live",
    "detect",
    "record",
    "snapshots",
    "objects",
    "zones",
    "review",
    "motion",
    "audio",
    "timestamp_style",
    "onvif",
    "ptz",
    "genai",
    "semantic_search",
    "face_recognition",
    "lpr",
    "classification",
    "ffmpeg",
    "notifications",
    "camera_groups",
    "layouts",
    "snapshots",
    "snapshots_dir",
    "snapshots_expunge",
    "non_docker",
    "safe_mode",
    "remote_stream",
    "hwaccel_args",
    "input_args",
    "output_args",
    "global_args",
    "hwaccel",
    "retry_interval",
    "width",
    "height",
    "fps",
    "queue",
    "paths",
    "url",
    "api",
    "rtmp",
    "restream",
    "listen",
    "webrtc",
    "record",
    "snapshots",
    "exports",
    "previews",
    "events",
    "alerts",
    "detections",
    "cameras_ui",
    "timelapse",
    "profile",
    "in_memory",
];

/// Frigate config census.
#[derive(Debug, Clone)]
pub struct Frigate {
    /// Top-level `key:`/`key value` sections matched against the section list.
    pub sections: usize,
    /// Camera entries (`<name>:` under `cameras:`).
    pub cameras: usize,
    /// `- path:`/`- url:`/`rtsp://`/`rtmp://`/`http` stream inputs.
    pub inputs: usize,
    /// `roles:` values (`detect`/`record`/`audio`/`restream`).
    pub roles: usize,
    /// `detect:`/`record:`/`snapshots:`/`objects:`/`zones:`/`motion:`/`review:`/`audio:` blocks (any depth).
    pub features: usize,
    /// `detectors:` entries + `type:`/`device:`/`model:`/`num_threads` keys.
    pub detectors: usize,
    /// `hwaccel_args`/`hwaccel`/`input_args`/`output_args`/`global_args` keys + `- ` ffmpeg args.
    pub ffmpeg_args: usize,
}

/// Whether the buffer looks like a Frigate config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.lines().any(|l| l.trim() == "cameras:")
        || t.contains("go2rtc:")
        || (t.contains("detectors:") && t.contains("mqtt:"))
        || (t.contains("ffmpeg:") && t.contains("roles:"))
}

impl Frigate {
    /// Parse a Frigate config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            cameras: 0,
            inputs: 0,
            roles: 0,
            features: 0,
            detectors: 0,
            ffmpeg_args: 0,
        };
        let mut cam_depth: Option<usize> = None;
        let mut cam_entry: Option<usize> = None;
        for l in t.lines() {
            let s = l.trim_start();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let indent = l.len() - s.len();
            if indent == 0 {
                cam_depth = None;
                cam_entry = None;
                let head = s.split(':').next().unwrap_or("").trim_end();
                if SECTIONS.contains(&head) {
                    c.sections += 1;
                    if head == "cameras" && s.ends_with(':') {
                        cam_depth = Some(0);
                    }
                }
                continue;
            }
            if cam_depth.is_some() {
                match cam_entry {
                    None => {
                        cam_entry = Some(indent);
                        c.cameras += 1;
                        continue;
                    }
                    Some(e) if indent == e => {
                        c.cameras += 1;
                        continue;
                    }
                    Some(e) if indent < e => {
                        cam_depth = None;
                        cam_entry = None;
                    }
                    Some(_) => {}
                }
            }
            let head = s
                .split(':')
                .next()
                .unwrap_or("")
                .trim_start_matches('-')
                .trim();
            if s.starts_with("- path:")
                || s.starts_with("- url:")
                || s.contains("rtsp://")
                || s.contains("rtmp://")
            {
                c.inputs += 1;
                continue;
            }
            if s.starts_with("roles:")
                || s.starts_with("- detect")
                || s.starts_with("- record")
                || s.starts_with("- audio")
                || s.starts_with("- restream")
            {
                c.roles += 1;
                continue;
            }
            if [
                "detect",
                "record",
                "snapshots",
                "objects",
                "zones",
                "motion",
                "review",
                "audio",
                "birdseye",
                "live",
                "onvif",
                "ptz",
                "timestamp_style",
                "genai",
                "semantic_search",
                "face_recognition",
                "lpr",
                "classification",
                "notifications",
                "exports",
                "previews",
                "events",
                "alerts",
                "timelapse",
            ]
            .contains(&head)
            {
                c.features += 1;
                continue;
            }
            if [
                "type",
                "device",
                "model",
                "num_threads",
                "labels",
                "labelmap",
                "labelmap_path",
                "path",
                "width",
                "height",
                "input_tensor",
                "input_pixel_format",
                "input_dtype",
                "confluent",
            ]
            .contains(&head)
            {
                c.detectors += 1;
                continue;
            }
            if [
                "hwaccel_args",
                "hwaccel",
                "input_args",
                "output_args",
                "global_args",
                "ffmpeg",
                "retry_interval",
            ]
            .contains(&head)
                || (s.starts_with("- ") && s.contains('='))
            {
                c.ffmpeg_args += 1;
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
            "mqtt:\n",
            "  host: localhost\n",
            "  user: u\n",
            "detectors:\n",
            "  coral:\n",
            "    type: edgetpu\n",
            "    device: usb\n",
            "go2rtc:\n",
            "cameras:\n",
            "  porch:\n",
            "    ffmpeg:\n",
            "      inputs:\n",
            "        - path: rtsp://cam/stream\n",
            "          roles:\n",
            "            - detect\n",
            "            - record\n",
            "        - path: rtsp://cam/sub\n",
            "          roles:\n",
            "            - audio\n",
            "      hwaccel_args: preset-vaapi\n",
            "    detect:\n",
            "      width: 1280\n",
            "      height: 720\n",
            "    record:\n",
            "      enabled: true\n",
            "    objects:\n",
            "      track:\n",
            "        - person\n",
            "        - car\n",
            "  garden:\n",
            "    ffmpeg:\n",
            "      inputs:\n",
            "        - path: rtsp://cam2/main\n",
            "          roles:\n",
            "            - detect\n",
            "birdseye:\n",
            "  enabled: true\n",
            "ui:\n",
            "  timezone: utc\n",
        );
        let c = Frigate::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 6);
        assert_eq!(c.cameras, 2);
        assert_eq!(c.inputs, 3);
        assert_eq!(c.roles, 7);
        assert_eq!(c.features, 3);
        assert_eq!(c.detectors, 4);
        assert_eq!(c.ffmpeg_args, 3);
    }

    #[test]
    fn rejects_other() {
        assert!(Frigate::parse(b"foo = 1").is_none());
    }
}
