//! `default.conf` (gqrx) 検出モジュール。
//!
//! Gqrx (SDR 受信機) の設定は INI 形式で、`[general]`/`[input]`/
//! `[receiver]`/`[audio]`/`[demod]`/`[recording]`/`[network]`
//! セクションと `demod`/`filter_low_cut`/`freq`/`gain`/`antenna`
//! 等のキーで構成される。
//!
//! ```
//! let b = br#"[general]
//! fft_rate = 15
//! [input]
//! device = "osmosdr=0"
//! [receiver]
//! demod = 1
//! filter_width = 10000
//! "#;
//! let c = izanagi_kit::gqrxconf::parse(b);
//! assert!(izanagi_kit::gqrxconf::detect(b));
//! assert_eq!(c.sections, 3);
//! ```

const SECTIONS: &[&str] = &[
    "[audio]",
    "[demod]",
    "[dsp]",
    "[general]",
    "[gui]",
    "[input]",
    "[iq_tool]",
    "[network]",
    "[receiver]",
    "[recording]",
    "[remote_control]",
    "[scanner]",
];

const KEYS: &[&str] = &[
    "agc",
    "antenna",
    "audio_gain",
    "bandwidth",
    "bpf",
    "color_scheme",
    "corrections",
    "cw_offset",
    "decay",
    "demod",
    "device",
    "docking",
    "dtr",
    "fft_rate",
    "filter_hi_cut",
    "filter_low_cut",
    "filter_width",
    "freq",
    "gain",
    "hang_time",
    "inversion",
    "latency",
    "lo_offset",
    "max_dev",
    "nb_on",
    "notch_freq",
    "panadapter",
    "ppm",
    "rds",
    "recorder_location",
    "sample_rate",
    "sql_level",
    "squelch",
    "tau",
];

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が gqrx 設定に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if SECTIONS.contains(&tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 3
}

/// gqrx 設定の統計。
#[derive(Debug, Default, Clone)]
pub struct GqrxConf {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を gqrx 設定として統計する。
pub fn parse(b: &[u8]) -> GqrxConf {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = GqrxConf::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if SECTIONS.contains(&tr) {
            c.sections += 1;
        } else if key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"[input]
device = "osmosdr=0"
demod = 1
freq = 144800000
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_keys_only() {
        let b = br#"demod = 2
filter_low_cut = -5000
filter_hi_cut = 5000
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.keys, 3);
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[input]\n"));
        assert!(!detect(b"[general]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"demod = 1\nfreq = 1\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
