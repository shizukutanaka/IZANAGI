//! BrainVision header (`*.vhdr`) census.
//!
//! INI-like: `[Brain Vision Data Format Header]`, `Data File=`/`Marker
//! File=`, `DataFormat`/`DataOrientation`/`NumberOfChannels`/`Sampling
//! Interval`, then `[Channel Infos]` `ChN=` entries and `[Binary Infos]`.
//!
//! ```
//! let s = b"[Brain Vision Data Format Header]\nData File=d.eeg\nMarker File=d.vmrk\nDataFormat=BINARY\nDataOrientation=MULTIPLEXED\nNumberOfChannels=2\nSampling Interval=2000\n[Channel Infos]\nCh1=FP1,,1,uv\nCh2=FP2,,1,uv\n[Binary Infos]\nBinaryFormat=INT_16\n";
//! assert!(izanagi_kit::vhdr::detect(s));
//! let v = izanagi_kit::vhdr::Vhdr::parse(s).unwrap();
//! assert_eq!(v.channels, 2);
//! assert_eq!(v.number_of_channels, 2);
//! assert_eq!(v.sampling_interval, 2000);
//! assert_eq!(v.sections, 3);
//! ```

/// Parsed census of a BrainVision `*.vhdr` header.
#[derive(Debug, Clone)]
pub struct Vhdr {
    /// `[...]` section headers.
    pub sections: usize,
    /// `Key=Value` lines.
    pub keys: usize,
    /// `Data File=` assignments.
    pub data_files: usize,
    /// `Marker File=` assignments.
    pub marker_files: usize,
    /// `DataFormat=` assignments.
    pub data_formats: usize,
    /// `DataOrientation=` assignments.
    pub orientations: usize,
    /// `NumberOfChannels=` value, or 0.
    pub number_of_channels: usize,
    /// `Sampling Interval=` value, or 0.
    pub sampling_interval: usize,
    /// `ChN=` channel entries.
    pub channels: usize,
    /// `[Channel Infos]` sections.
    pub channel_infos: usize,
    /// `[Binary Infos]` sections.
    pub binary_infos: usize,
    /// `BinaryFormat=` assignments.
    pub binary_formats: usize,
    /// `CodePage=` assignments.
    pub code_pages: usize,
    /// `MULTIPLEXED` orientation.
    pub multiplexed: usize,
    /// `VECTORIZED` orientation.
    pub vectorized: usize,
    /// `[Comment]` section lines.
    pub comments: usize,
    /// `Demodulation`/`Impedance` style hardware keys.
    pub hardware: usize,
}

fn val(t: &str, key: &str) -> usize {
    for l in t.lines() {
        if let Some(v) = l.trim().strip_prefix(key) {
            if let Some(v) = v.strip_prefix('=') {
                return v.trim().parse().unwrap_or(0);
            }
        }
    }
    0
}

/// Reports whether `b` looks like a BrainVision header.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.contains("[Brain Vision") || (t.contains("Data File=") && t.contains("NumberOfChannels="))
}

impl Vhdr {
    /// Parses `b` as a BrainVision header, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut v = Vhdr {
            sections: 0,
            keys: 0,
            data_files: 0,
            marker_files: 0,
            data_formats: 0,
            orientations: 0,
            number_of_channels: 0,
            sampling_interval: 0,
            channels: 0,
            channel_infos: 0,
            binary_infos: 0,
            binary_formats: 0,
            code_pages: 0,
            multiplexed: 0,
            vectorized: 0,
            comments: 0,
            hardware: 0,
        };
        let mut in_comment = false;
        for l in t.lines() {
            let l = l.trim_end();
            if l.starts_with('[') && l.ends_with(']') {
                v.sections += 1;
                in_comment = l == "[Comment]";
                if l == "[Channel Infos]" {
                    v.channel_infos += 1;
                }
                if l == "[Binary Infos]" {
                    v.binary_infos += 1;
                }
                continue;
            }
            if in_comment {
                if !l.trim().is_empty() {
                    v.comments += 1;
                }
                continue;
            }
            if let Some((k, val_s)) = l.split_once('=') {
                v.keys += 1;
                match k {
                    "Data File" => v.data_files += 1,
                    "Marker File" => v.marker_files += 1,
                    "DataFormat" => v.data_formats += 1,
                    "DataOrientation" => {
                        v.orientations += 1;
                        if val_s.contains("MULTIPLEXED") {
                            v.multiplexed += 1;
                        }
                        if val_s.contains("VECTORIZED") {
                            v.vectorized += 1;
                        }
                    }
                    "BinaryFormat" => v.binary_formats += 1,
                    "CodePage" => v.code_pages += 1,
                    "Demodulation" | "Impedance" | "Notchfilter" | "SoftwareFiltersEna" => {
                        v.hardware += 1;
                    }
                    _ if k.starts_with("Ch") && k[2..].bytes().all(|c| c.is_ascii_digit()) => {
                        v.channels += 1;
                    }
                    _ => {}
                }
            }
        }
        v.number_of_channels = val(t, "NumberOfChannels");
        v.sampling_interval = val(t, "Sampling Interval");
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"[Brain Vision Data Format Header]\nData File=d.eeg\nMarker File=d.vmrk\nDataFormat=BINARY\nDataOrientation=MULTIPLEXED\nNumberOfChannels=2\nSampling Interval=2000\n[Channel Infos]\nCh1=FP1,,1,uv\nCh2=FP2,,1,uv\n[Binary Infos]\nBinaryFormat=INT_16\n";

    #[test]
    fn parses_vhdr() {
        assert!(detect(S));
        let v = Vhdr::parse(S).unwrap();
        assert_eq!(v.channels, 2);
        assert_eq!(v.number_of_channels, 2);
        assert_eq!(v.sampling_interval, 2000);
        assert_eq!(v.sections, 3);
    }

    #[test]
    fn rejects_non_vhdr() {
        assert!(!detect(b"[x]"));
        assert!(Vhdr::parse(b"").is_none());
    }
}
