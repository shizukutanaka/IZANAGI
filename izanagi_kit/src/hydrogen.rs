//! Hydrogen ソング `.h2song` / ドラムキット `.h2drumkit` (XML) の検出・カウント。
//!
//! `<song>` / `<drumkit_info>` ルート。
//! `<pattern>` パターン、`<instrument>` インストゥルメント、`<note>` ノート、
//! `<layer>` レイヤ、`<patternID>` シーケンス、`<component>` コンポーネントを分類する。
//!
//! ```
//! let xml = br#"<song><bpm>120</bpm>
//!               <patternList><pattern><noteList><note><velocity>0.8</velocity></note></noteList></pattern></patternList>
//!               <instrumentList><instrument><layer><filename>k.wav</filename></layer></instrument></instrumentList>
//!               </song>"#;
//! assert!(izanagi_kit::hydrogen::detect(xml));
//! let c = izanagi_kit::hydrogen::parse(xml).unwrap();
//! assert_eq!(c.patterns, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<pattern>` パターン数。
    pub patterns: usize,
    /// `<instrument>` インストゥルメント数 (ノート内参照を含む)。
    pub instruments: usize,
    /// `<note>` ノート数。
    pub notes: usize,
    /// `<layer>` サンプルレイヤ数。
    pub layers: usize,
    /// `<patternID>` シーケンス参照数。
    pub sequence: usize,
    /// `<component>`/`<drumkitComponent>` コンポーネント数。
    pub components: usize,
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

/// `b` が Hydrogen ソング/キットかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を Hydrogen XML として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    match it.first().copied() {
        Some("song") => {
            // 汎用 `<song>` と区別するため Hydrogen 固有の子要素を要求。
            if !text.contains("<instrumentList")
                && !text.contains("<patternList")
                && !text.contains("<patternSequence")
                && !text.contains("<drumkit")
            {
                return None;
            }
        }
        Some("drumkit_info") => {}
        _ => return None,
    }
    it.remove(0);
    let mut c = Counts {
        entries: 0,
        patterns: 0,
        instruments: 0,
        notes: 0,
        layers: 0,
        sequence: 0,
        components: 0,
        misc: 0,
    };
    for name in it {
        c.entries += 1;
        match name {
            "pattern" => c.patterns += 1,
            "instrument" => c.instruments += 1,
            "note" => c.notes += 1,
            "layer" => c.layers += 1,
            "patternID" => c.sequence += 1,
            "component" | "drumkitComponent" => c.components += 1,
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

    const SAMPLE: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
    <song>
    <version>1.2.0</version>
    <bpm>140.5</bpm>
    <volume>0.5</volume>
    <metronomeVolume>0.5</metronomeVolume>
    <name>demo</name>
    <author>dev</author>
    <license>CC0</license>
    <patternSequence>
    <group>
    <patternID>intro</patternID>
    <patternID>verse</patternID>
    </group>
    </patternSequence>
    <patternList>
    <pattern>
    <pattern_name>intro</pattern_name>
    <beat_resolution>4</beat_resolution>
    <denominator>4</denominator>
    <noteList>
    <note><position>0</position><leadlag>0</leadlag><velocity>0.8</velocity>
    <pan_L>1</pan_L><pan_R>1</pan_R><pitch>0</pitch><key>C0</key>
    <length>-1</length><instrument>0</instrument></note>
    <note><position>48</position><leadlag>0</leadlag><velocity>0.9</velocity>
    <pan_L>1</pan_L><pan_R>1</pan_R><pitch>0</pitch><key>C0</key>
    <length>-1</length><instrument>1</instrument></note>
    </noteList>
    </pattern>
    <pattern>
    <pattern_name>verse</pattern_name>
    <noteList>
    <note><position>0</position><leadlag>0</leadlag><velocity>1.0</velocity>
    <pan_L>1</pan_L><pan_R>1</pan_R><pitch>0</pitch><key>C0</key>
    <length>-1</length><instrument>0</instrument></note>
    </noteList>
    </pattern>
    </patternList>
    <virtualPatternList/>
    <instrumentList>
    <instrument>
    <id>0</id><name>kick</name><volume>1.0</volume>
    <drumkit>kit1</drumkit>
    <layer><filename>kick.wav</filename><min>0</min><max>1</max><gain>1.0</gain><pitch>0</pitch></layer>
    </instrument>
    <instrument>
    <id>1</id><name>snare</name><volume>1.0</volume>
    <layer><filename>snare.wav</filename><min>0</min><max>1</max></layer>
    <layer><filename>snare2.wav</filename><min>0.5</min><max>1</max></layer>
    </instrument>
    </instrumentList>
    <bpmTimeLine/>
    </song>"#;

    #[test]
    fn detects_h2song() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.patterns, 2);
        assert_eq!(c.instruments, 5);
        assert_eq!(c.notes, 3);
        assert_eq!(c.layers, 3);
        assert_eq!(c.sequence, 2);
        assert!(c.misc >= 10);
    }

    #[test]
    fn detects_drumkit() {
        let kit = br#"<drumkit_info><name>kit</name><author>a</author>
            <instrumentList><instrument><name>k</name><layer><filename>k.wav</filename></layer></instrument></instrumentList>
            </drumkit_info>"#;
        assert!(detect(kit));
        let c = parse(kit).unwrap();
        assert_eq!(c.instruments, 1);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<song><title>x</title></song>"#));
        assert!(!detect(b"plain"));
    }
}
