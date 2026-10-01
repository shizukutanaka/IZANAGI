//! Audacity プロジェクト `.aup` (XML) の検出・カウント。
//!
//! `<project xmlns="audacityproject" …>` ルート。
//! `<wavetrack>`/`<labeltrack>`/`<timetrack>`/`<notetrack>` トラック、
//! `<waveclip>` クリップ、`<sequence>`/`<waveblock>`/`<simpleblockfile>` データブロック、
//! `<label>` ラベル、`<import>` インポートを分類する。
//!
//! ```
//! let xml = br#"<project xmlns="audacityproject" version="1.3.0" audacityversion="3.4">
//!               <wavetrack name="t1" channel="0"><waveclip offset="0"><sequence maxsamples="8192"/></waveclip></wavetrack>
//!               <labeltrack name="labels"><label t="0.0" t1="1.0" title="mark"/></labeltrack>
//!               </project>"#;
//! assert!(izanagi_kit::audacity::detect(xml));
//! let c = izanagi_kit::audacity::parse(xml).unwrap();
//! assert_eq!(c.tracks, 2);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<wavetrack>`/`<labeltrack>`/`<timetrack>`/`<notetrack>` トラック数。
    pub tracks: usize,
    /// `<waveclip>`/`<timeclip>`/`<noteclip>`/`<silence>` クリップ数。
    pub clips: usize,
    /// `<sequence>` シーケンス数。
    pub sequences: usize,
    /// `<waveblock>`/`<simpleblockfile>`/`<pcmsampleblock>`/`<legacyblockfile>` ブロック数。
    pub blocks: usize,
    /// `<label>` ラベル数。
    pub labels: usize,
    /// `<import>` インポート数。
    pub imports: usize,
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

/// `b` が Audacity `.aup` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を Audacity `.aup` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    if it.first().copied() != Some("project") {
        return None;
    }
    it.remove(0);
    if !text.contains("audacityproject") {
        return None;
    }
    let mut c = Counts {
        entries: 0,
        tracks: 0,
        clips: 0,
        sequences: 0,
        blocks: 0,
        labels: 0,
        imports: 0,
        misc: 0,
    };
    for name in it {
        c.entries += 1;
        match name {
            "wavetrack" | "labeltrack" | "timetrack" | "notetrack" => c.tracks += 1,
            "waveclip" | "timeclip" | "noteclip" | "silence" => c.clips += 1,
            "sequence" => c.sequences += 1,
            "waveblock" | "simpleblockfile" | "pcmsampleblock" | "legacyblockfile" => {
                c.blocks += 1;
            }
            "label" => c.labels += 1,
            "import" => c.imports += 1,
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

    const SAMPLE: &[u8] = br#"<?xml version="1.0" standalone="no" ?>
    <!DOCTYPE project PUBLIC "-//audacityproject-1.3.0//DTD//EN">
    <project xmlns="audacityproject" version="1.3.0" audacityversion="3.4.2">
    <tags/>
    <wavetrack name="track1" channel="0" linked="1" mute="0" solo="0" height="150" rate="44100">
    <waveclip offset="0.0" trimLeft="0.0" trimRight="0.0" rate="44100">
    <sequence maxsamples="262144" sampleformat="262159" numsamples="88200">
    <waveblock start="0">
    <simpleblockfile filename="e0001.au" len="44100" min="-1.0" max="1.0" rms="0.5"/>
    </waveblock>
    <waveblock start="44100">
    <pcmsampleblock start="44100" len="44100" min="-1.0" max="1.0"/>
    </waveblock>
    </sequence>
    <envelope numpoints="0"/>
    </waveclip>
    </wavetrack>
    <wavetrack name="track2" channel="1">
    <waveclip offset="1.0">
    <sequence maxsamples="262144">
    <silence start="0" len="1000"/>
    </sequence>
    </waveclip>
    </wavetrack>
    <labeltrack name="markers" numlabels="2">
    <label t="0.0" t1="1.0" title="intro"/>
    <label t="2.0" t1="3.0" title="verse"/>
    </labeltrack>
    <timetrack name="time" height="150"/>
    <import filename="vocal.wav"/>
    </project>"#;

    #[test]
    fn detects_aup() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tracks, 4);
        assert_eq!(c.clips, 3);
        assert_eq!(c.sequences, 2);
        assert_eq!(c.blocks, 4);
        assert_eq!(c.labels, 2);
        assert_eq!(c.imports, 1);
        assert!(c.misc >= 2);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<project><a/></project>"#));
        assert!(!detect(b"plain"));
    }
}
