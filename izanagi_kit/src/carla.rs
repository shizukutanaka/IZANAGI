//! Carla パッチベイプロジェクト `.carxp` (XML) の検出・カウント。
//!
//! `<!DOCTYPE CARLA-PRESET>` / `<CARLA-PRESET>` ルート。
//! `<Plugin>` プラグイン、`<Parameter>` パラメータ、`<Cable>` 接続ケーブル、
//! `<CustomData>` カスタムデータ、`<Info>` 情報ブロックを分類する。
//!
//! ```
//! let xml = br#"<!DOCTYPE CARLA-PRESET>
//!               <CARLA-PRESET VERSION='2.0'>
//!               <Info><Name>test</Name></Info>
//!               <Data><Plugin><Data>
//!               <CustomData><Key>k</Key><Value>v</Value></CustomData>
//!               </Data></Plugin></Data>
//!               </CARLA-PRESET>"#;
//! assert!(izanagi_kit::carla::detect(xml));
//! let c = izanagi_kit::carla::parse(xml).unwrap();
//! assert_eq!(c.plugins, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<Plugin>` プラグイン数。
    pub plugins: usize,
    /// `<Parameter>` パラメータ数。
    pub parameters: usize,
    /// `<Cable>` パッチケーブル数。
    pub cables: usize,
    /// `<CustomData>` カスタムデータ数。
    pub customdata: usize,
    /// `<Info>`/`<Name>`/`<Type>`/`<Symbol>` 情報タグ数。
    pub info: usize,
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

/// `b` が Carla プロジェクトかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.plugins >= 1 || c.cables >= 1 || c.entries - c.misc >= 3)
}

/// `b` を Carla `.carxp` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    if it.first().copied() != Some("CARLA-PRESET") {
        return None;
    }
    it.remove(0);
    let mut c = Counts {
        entries: 0,
        plugins: 0,
        parameters: 0,
        cables: 0,
        customdata: 0,
        info: 0,
        misc: 0,
    };
    for name in it {
        c.entries += 1;
        match name {
            "Plugin" => c.plugins += 1,
            "Parameter" => c.parameters += 1,
            "Cable" => c.cables += 1,
            "CustomData" => c.customdata += 1,
            "Info" | "Name" | "Type" | "Symbol" | "Comment" | "Badge" => c.info += 1,
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

    const SAMPLE: &[u8] = br#"<?xml version='1.0' encoding='UTF-8'?>
    <!DOCTYPE CARLA-PRESET>
    <CARLA-PRESET VERSION='2.5' juceVersion='6.1.2'>
    <Info>
    <Type>PLUGIN</Type>
    <Name>session1</Name>
    <Badge>SYNTH</Badge>
    </Info>
    <Data>
    <Plugin>
    <Info>
    <Type>LV2</Type>
    <Name>synth</Name>
    <Binary>/usr/lib/lv2/synth.so</Binary>
    <Label>http://example.org/synth</Label>
    </Info>
    <Data>
    <CustomData>
    <Type>text</Type>
    <Key>chunk</Key>
    <Value>AAAA</Value>
    </CustomData>
    <Parameter>
    <index>0</index>
    <name>cutoff</name>
    <symbol>cutoff</symbol>
    <default>0.5</default>
    <Value>0.75</Value>
    </Parameter>
    <Parameter>
    <index>1</index>
    <name>resonance</name>
    <symbol>res</symbol>
    <default>0.0</default>
    <Value>0.25</Value>
    </Parameter>
    </Data>
    </Plugin>
    <Plugin>
    <Info>
    <Type>INTERNAL</Type>
    <Name>audiofile</Name>
    </Info>
    <Data/>
    </Plugin>
    <Patchbay>
    <Cable>
    <type>audio</type>
    <outGroup>1</outGroup>
    <outPort>2</outPort>
    <inGroup>3</inGroup>
    <inPort>4</inPort>
    </Cable>
    <Cable>
    <type>midi</type>
    <outGroup>5</outGroup>
    <outPort>6</outPort>
    <inGroup>7</inGroup>
    <inPort>8</inPort>
    </Cable>
    </Patchbay>
    <Transport>
    <Playing>false</Playing>
    <Frame>0</Frame>
    <BPM>120.0</BPM>
    </Transport>
    </Data>
    </CARLA-PRESET>"#;

    #[test]
    fn detects_carxp() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.plugins, 2);
        assert_eq!(c.parameters, 2);
        assert_eq!(c.cables, 2);
        assert_eq!(c.customdata, 1);
        assert!(c.info >= 7);
        assert!(c.misc >= 8);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<root><a/></root>"#));
        assert!(!detect(b"plain"));
    }
}
