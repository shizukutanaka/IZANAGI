//! Mixxx 設定 `mixxx.cfg` の検出・カウント。
//!
//! `[Section]` セクションと `key value` (または `key=value`) エントリの
//! INI 風形式。セクション名の系統別にライブラリ/サウンド/コントロール/
//! ブロードキャスト/エフェクトへ分類する。
//!
//! ```
//! let cfg = br#"[App]
//! mixer 1
//! [Master]
//! latency 64
//! sample_rate 48000
//! [Broadcast 1]
//! enabled 0
//! "#;
//! assert!(izanagi_kit::mixxx::detect(cfg));
//! let c = izanagi_kit::mixxx::parse(cfg).unwrap();
//! assert_eq!(c.sections, 3);
//! ```

/// ライブラリ・再生系セクションの接頭辞。
const LIBRARY: &[&str] = &[
    "Playlist",
    "Library",
    "TrackDirectory",
    "Recording",
    "AutoDJ",
    "Analyze",
    "BPM",
    "ReplayGain",
    "Waveform",
    "SoundSource",
    "PromoTracks",
    "Tracks",
    "Crates",
    "BrowseModel",
    "Settings",
    "KCollection",
];
/// オーディオ出力系セクションの接頭辞。
const SOUND: &[&str] = &[
    "Soundcard",
    "Master",
    "Headphone",
    "Sampler",
    "SamplerBank",
    "OutputRack",
    "Monitor",
    "Latency",
    "EngineClock",
    "Passthrough",
    "Auxiliary",
];
/// コントロール入力系セクションの接頭辞。
const CONTROLS: &[&str] = &[
    "Controls",
    "VinylControl",
    "Keyboard",
    "Controller",
    "Midi",
    "Scripts",
    "Spinny",
    "VUMeter",
];
/// ブロードキャスト系セクションの接頭辞。
const BROADCAST: &[&str] = &["Broadcast", "Shoutcast", "Livebroadcast"];
/// エフェクト系セクションの接頭辞。
const EFFECTS: &[&str] = &[
    "EffectRack",
    "QuickEffectRack",
    "EqualizerRack",
    "Effect",
    "ChainPreset",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `[Section]` セクション数。
    pub sections: usize,
    /// `key value`/`key=value` エントリ総数。
    pub entries: usize,
    /// ライブラリ系セクション内のエントリ数。
    pub library: usize,
    /// オーディオ出力系セクション内のエントリ数。
    pub sound: usize,
    /// コントロール系セクション内のエントリ数。
    pub controls: usize,
    /// ブロードキャスト系セクション内のエントリ数。
    pub broadcast: usize,
    /// エフェクト系セクション内のエントリ数。
    pub effects: usize,
    /// その他セクション内のエントリ数。
    pub misc: usize,
}

/// セクション名の系統を返す。
fn family(section: &str) -> fn(&mut Counts) -> &mut usize {
    if BROADCAST.iter().any(|p| section.starts_with(p)) {
        |c: &mut Counts| &mut c.broadcast
    } else if EFFECTS.iter().any(|p| section.starts_with(p)) {
        |c: &mut Counts| &mut c.effects
    } else if SOUND.iter().any(|p| section.starts_with(p)) {
        |c: &mut Counts| &mut c.sound
    } else if CONTROLS.iter().any(|p| section.starts_with(p)) {
        |c: &mut Counts| &mut c.controls
    } else if LIBRARY.iter().any(|p| section.starts_with(p)) {
        |c: &mut Counts| &mut c.library
    } else {
        |c: &mut Counts| &mut c.misc
    }
}

/// `b` が `mixxx.cfg` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.sections >= 2 && c.entries - c.misc >= 3)
}

/// `b` を `mixxx.cfg` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        sections: 0,
        entries: 0,
        library: 0,
        sound: 0,
        controls: 0,
        broadcast: 0,
        effects: 0,
        misc: 0,
    };
    let mut section = "";
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') {
            if let Some(end) = t.find(']') {
                section = &t[1..end];
                c.sections += 1;
                continue;
            }
        }
        if section.is_empty() {
            continue;
        }
        // `key value` または `key=value` 形式。先頭トークンがキー。
        let Some(head) = t.split_whitespace().next() else {
            continue;
        };
        let key = head.split('=').next().unwrap_or(head);
        if key.is_empty()
            || !key
                .bytes()
                .all(|x| x.is_ascii_alphanumeric() || matches!(x, b'_' | b'.' | b'-' | b'/'))
        {
            continue;
        }
        c.entries += 1;
        let slot = family(section)(&mut c);
        *slot += 1;
    }
    if c.sections >= 1 && c.entries >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#";; Mixxx Settings
[App]
mixer 1
samplerates 48000
[Soundcard]
samplerate 48000
latency 64
[Master]
enabled 1
[Playlist]
auto_reload 0
[Library]
TracksTableRowHeight 32
[Recording]
path ~/music
[Broadcast 1]
enabled 0
host example.com
[Shoutcast]
enabled 1
[VinylControl]
input_type timecode
[Controls]
RateDir 0
[EffectRack1]
num_effectunits 2
[EffectRack1_EffectUnit1]
group [Channel1]
[AutoDJ]
enabled 0
[Unknown]
foo 1
"#;

    #[test]
    fn detects_mixxx() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.sections, 14);
        assert_eq!(c.entries, 17);
        assert_eq!(c.library, 4);
        assert_eq!(c.sound, 3);
        assert_eq!(c.controls, 2);
        assert_eq!(c.broadcast, 3);
        assert_eq!(c.effects, 2);
        assert_eq!(c.misc, 3);
    }

    #[test]
    fn rejects_other_ini() {
        assert!(!detect(b"[one]\nkey 1\n"));
        assert!(!detect(b"plain"));
    }
}
