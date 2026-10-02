//! LMMS プロジェクト `.mmp` (XML) の検出・カウント。
//!
//! `<lmms-project type="song" version="…">` ルート。
//! `<track>` トラック、`<pattern>`/`<bbtco>`/`<tco>`/`<automationpattern>` パターン、
//! `<note>` ノート、`<fxchannel>`/`<fx>` エフェクト、楽器プラグイン要素を分類する。
//!
//! ```
//! let xml = br#"<lmms-project type="song" version="1.2" creator="LMMS">
//!               <song bpm="140"><trackcontainer><track type="0" name="t">
//!               <instrumenttrack><instrument name="tripleoscillator"><tripleoscillator/></instrument></instrumenttrack>
//!               </track></trackcontainer></song>
//!               </lmms-project>"#;
//! assert!(izanagi_kit::lmms::detect(xml));
//! let c = izanagi_kit::lmms::parse(xml).unwrap();
//! assert_eq!(c.tracks, 1);
//! ```

/// 楽器・ジェネレータ系プラグインのタグ名。
const INSTRUMENTS: &[&str] = &[
    "audiofileprocessor",
    "tripleoscillator",
    "vestige",
    "vstplugin",
    "ladspaplugin",
    "zynaddsubfx",
    "zynaddsubfxremote",
    "lb302",
    "kicker",
    "monstro",
    "nes",
    "organic",
    "papu",
    "patman",
    "sf2player",
    "sfxr",
    "sid",
    "watsyn",
    "bitinvader",
    "freeboy",
    "opulenz",
    "xpressive",
    "vibed",
    "stereoenhancer",
    "gigplayer",
    "samplerco",
    "carlabase",
    "carlapatchbay",
    "carlarack",
    "remoteplugin",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<track>` トラック数。
    pub tracks: usize,
    /// `<note>` ノート数。
    pub notes: usize,
    /// `<pattern>`/`<bbtco>`/`<tco>`/`<automationpattern>`/`<sampletco>` パターン・クリップ数。
    pub patterns: usize,
    /// `<fxchannel>`/`<fx>`/`<effect>`/`<ladspacontrols>`/`<controls>` エフェクト数。
    pub fx: usize,
    /// 楽器・ジェネレータプラグイン要素数。
    pub instruments: usize,
    /// その他タグ数。
    pub misc: usize,
}

/// 開始タグ名を順に列挙。
fn tags(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        if j < bytes.len() && matches!(bytes[j], b'!' | b'?' | b'/') {
            i += 1;
            continue;
        }
        let start = j;
        while j < bytes.len()
            && (bytes[j].is_ascii_alphanumeric() || matches!(bytes[j], b'_' | b'-' | b'.' | b':'))
        {
            j += 1;
        }
        if j > start {
            out.push(&text[start..j]);
        }
        i = j;
    }
    out
}

/// `b` が LMMS プロジェクトかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を LMMS `.mmp` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    if it.first().copied() != Some("lmms-project") {
        return None;
    }
    it.remove(0);
    let mut c = Counts {
        entries: 0,
        tracks: 0,
        notes: 0,
        patterns: 0,
        fx: 0,
        instruments: 0,
        misc: 0,
    };
    for name in it {
        c.entries += 1;
        match name {
            "track" => c.tracks += 1,
            "note" => c.notes += 1,
            "pattern" | "bbtco" | "tco" | "automationpattern" | "sampletco" => c.patterns += 1,
            "fxchannel" | "fx" | "effect" | "ladspacontrols" | "controls" => c.fx += 1,
            _ if INSTRUMENTS.contains(&name) => c.instruments += 1,
            _ => c.misc += 1,
        }
    }
    if c.entries >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<?xml version="1.0"?>
    <!DOCTYPE lmms-project>
    <lmms-project type="song" version="1.2" creator="LMMS" creatorversion="1.2.2">
    <head timesig_numerator="4" timesig_denominator="4" bpm="140" masterpitch="0" mastervol="100"/>
    <song>
    <trackcontainer width="600" x="0" y="0" maximized="0" minimized="0" position="0">
    <track type="0" name="melody" muted="0" solo="0">
    <instrumenttrack pitch="0" pitchrange="1" fxch="0" usemasterpitch="1">
    <instrument name="tripleoscillator">
    <tripleoscillator finel0="0" finel1="0" finel2="0"/>
    </instrument>
    <chordcreator chord="0" chordrange="1"/>
    <arpeggiator arp="0" arprange="1"/>
    <eldata ftype="0" fc="1000"/>
    </instrumenttrack>
    <pattern pos="0" len="192" name="melody" type="0" steps="16">
    <note key="60" pos="0" len="24" vol="100" pan="0"/>
    <note key="64" pos="24" len="24" vol="100" pan="0"/>
    <note key="67" pos="48" len="48" vol="100" pan="0"/>
    </pattern>
    </track>
    <track type="1" name="beat" muted="0" solo="0">
    <bbtrack>
    <bbtco pos="0" len="192"/>
    <bbtco pos="192" len="192"/>
    </bbtrack>
    <pattern pos="0" len="192" name="bb" type="1" steps="16">
    <note key="36" pos="0" len="12" vol="100" pan="0"/>
    </pattern>
    </track>
    <track type="3" name="aut" muted="0" solo="0">
    <automationtrack>
    <automationpattern pos="0" len="192" name="cutoff" tens="1" prog="1"/>
    </automationtrack>
    </track>
    </trackcontainer>
    <timeline lp0pos="0" lp1pos="192" lpstate="0" stoppoint="-1"/>
    <fxchannel num="0" name="master">
    <fx>
    <effect name="ladspaeffect">
    <ladspacontrols ports="2"/>
    </effect>
    </fx>
    </fxchannel>
    <fxchannel num="1" name="fx1">
    <fx>
    <effect name="bassbooster"/>
    </fx>
    </fxchannel>
    </song>
    </lmms-project>"#;

    #[test]
    fn detects_mmp() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tracks, 3);
        assert_eq!(c.notes, 4);
        assert_eq!(c.patterns, 5);
        assert_eq!(c.fx, 7);
        assert_eq!(c.instruments, 1);
        assert!(c.misc >= 8);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<root><a/></root>"#));
        assert!(!detect(b"plain"));
    }
}
