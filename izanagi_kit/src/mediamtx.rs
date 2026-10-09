//! MediaMTX / rtsp-simple-server `mediamtx.yml` census.
//!
//! mediamtx.yml is flat-ish YAML: `logLevel`, `logDestinations`,
//! `readTimeout`, `writeTimeout`, `writeQueueSize`, `udpMaxPayloadSize`,
//! `runOnConnect`, protocol toggles and addresses
//! (`rtsp`/`rtspAddress`/`rtpAddress`/`rtcpAddress`/
//! `rtmp`/`rtmpAddress`/`hls`/`hlsAddress`/`webrtc`/`webrtcAddress`/
//! `srt`/`srtAddress`), `api`/`apiAddress`, `metrics`/`metricsAddress`,
//! `pprof`/`pprofAddress`, `playback`/`playbackAddress`,
//! `auth*` (`authMethod`/`authInternalUsers`/`authHTTPAddress`/
//! `authJWTJWKS`), and a `paths:` map whose keys are path names
//! with per-path options (`source`/`sourceProtocol`/
//! `sourceOnDemand`/`record`/`recordPath`/`recordFormat`).
//!
//! ```rust
//! let c = izanagi_kit::mediamtx::Mediamtx::parse(b"logLevel: info\nrtsp: yes\npaths:\n  cam1:\n").unwrap();
//! assert_eq!(c.path_entries, 1);
//! ```

use crate::textutil::strip_bom;
/// mediamtx.yml census.
#[derive(Debug, Clone)]
pub struct Mediamtx {
    /// Total `key:`/`key: value` lines.
    pub keys: usize,
    /// Keys matching the known MediaMTX option list.
    pub known: usize,
    /// Path entries under `paths:`.
    pub path_entries: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "logLevel",
    "logDestinations",
    "logFile",
    "readTimeout",
    "writeTimeout",
    "writeQueueSize",
    "udpMaxPayloadSize",
    "externalAuthenticationURL",
    "api",
    "apiAddress",
    "metrics",
    "metricsAddress",
    "pprof",
    "pprofAddress",
    "playback",
    "playbackAddress",
    "runOnConnect",
    "runOnConnectRestart",
    "runOnDisconnect",
    "rtsp",
    "protocols",
    "encryption",
    "rtspAddress",
    "rtspsAddress",
    "rtpAddress",
    "rtcpAddress",
    "multicastIPRange",
    "multicastRTPPort",
    "multicastRTCPPort",
    "serverKey",
    "serverCert",
    "authMethods",
    "rtmp",
    "rtmpAddress",
    "rtmpEncryption",
    "rtmpsAddress",
    "rtmpServerKey",
    "rtmpServerCert",
    "hls",
    "hlsAddress",
    "hlsEncryption",
    "hlsServerKey",
    "hlsServerCert",
    "hlsAlwaysRemux",
    "hlsVariant",
    "hlsSegmentCount",
    "hlsSegmentDuration",
    "hlsPartDuration",
    "hlsSegmentMaxSize",
    "hlsAllowOrigin",
    "hlsTrustedProxies",
    "hlsDirectory",
    "hlsMuxerCloseAfter",
    "webrtc",
    "webrtcAddress",
    "webrtcEncryption",
    "webrtcServerKey",
    "webrtcServerCert",
    "webrtcAllowOrigin",
    "webrtcTrustedProxies",
    "webrtcLocalUDPAddress",
    "webrtcLocalTCPAddress",
    "webrtcIPsFromInterfaces",
    "webrtcAdditionalHosts",
    "webrtcICEServers2",
    "srt",
    "srtAddress",
    "authMethod",
    "authInternalUsers",
    "authHTTPAddress",
    "authHTTPExclude",
    "authJWTJWKS",
    "authJWTClaimKey",
    "paths",
];

/// Whether the buffer looks like mediamtx.yml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.contains("paths:")
        && t.lines()
            .filter(|l| {
                let s = l.trim_start();
                KEYS.iter()
                    .any(|k| s.starts_with(k) && s[k.len()..].starts_with(':'))
            })
            .count()
            >= 3
}

impl Mediamtx {
    /// Parse mediamtx.yml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            keys: 0,
            known: 0,
            path_entries: 0,
            comments: 0,
        };
        let mut in_paths = false;
        for l in t.lines() {
            let s = l.trim_end();
            if s.trim().is_empty() {
                continue;
            }
            if s.trim().starts_with('#') {
                c.comments += 1;
                continue;
            }
            let indent = s.len() - s.trim_start().len();
            let body = s.trim_start();
            if indent == 0 {
                in_paths = body == "paths:" || body.starts_with("paths:");
            }
            let key = body.split(':').next().unwrap_or("");
            if key.is_empty() || !body.contains(':') {
                continue;
            }
            c.keys += 1;
            if indent == 0 && KEYS.contains(&key) {
                c.known += 1;
            }
            if in_paths && indent == 2 {
                c.path_entries += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mediamtx_yml() {
        let b = concat!(
            "# mediamtx.yml\n",
            "logLevel: info\n",
            "readTimeout: 10s\n",
            "rtsp: yes\n",
            "rtspAddress: :8554\n",
            "hls: yes\n",
            "hlsAddress: :8888\n",
            "webrtc: yes\n",
            "paths:\n",
            "  cam1:\n",
            "    source: rtsp://cam1\n",
            "  proxied:\n",
            "    source: rtsp://other/cam2\n",
        );
        let c = Mediamtx::parse(b.as_bytes()).unwrap();
        assert_eq!(c.keys, 12);
        assert_eq!(c.known, 8);
        assert_eq!(c.path_entries, 2);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Mediamtx::parse(b"a: 1\nb: 2\n").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
