//! Autodesk EAGLE `.sch`/`.brd` (XML) の検出・カウント。
//!
//! `<eagle version="…"><drawing>…` ルート。
//! `<layer>` レイヤ、`<part>`/`<element>` 部品、`<signal>`/`<net>` 信号、
//! `<wire>`/`<junction>`/`<label>`/`pinref` 接続、`<pad>`/`<smd>`/`<via>`/`<hole>`
//! パッド、`<package>`/`<deviceset>`/`<symbol>` ライブラリ、
//! `<attribute>`/`<text>`/`<note>` 属性を分類する。
//!
//! ```
//! let cfg = br#"<eagle version="9.6"><drawing><settings><grid distance="0.1"/>
//!               <layers><layer number="1" name="Top"/><layer number="16" name="Bottom"/></layers>
//!               <board><signals><signal name="GND"><wire x1="0" y1="0" x2="1" y2="0" width="0.1" layer="16"/></signal></signals></board>
//!               </drawing></eagle>"#;
//! assert!(izanagi_kit::eaglexml::detect(cfg));
//! let c = izanagi_kit::eaglexml::parse(cfg).unwrap();
//! assert_eq!(c.layers, 2);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<layer>` レイヤ数。
    pub layers: usize,
    /// `<part>`/`<element>`/`<instance>` 部品数。
    pub parts: usize,
    /// `<signal>`/`<net>`/`<bus>` 信号・ネット数。
    pub signals: usize,
    /// `<wire>`/`<junction>`/`<label>`/`<pinref>`/`<segment>` 接続図形数。
    pub wires: usize,
    /// `<pad>`/`<smd>`/`<via>`/`<hole>` パッド・ビア数。
    pub pads: usize,
    /// `<package>`/`<packages>`/`<deviceset>`/`<device>`/`<symbol>`/`<symbols>`/`<library>`/`<libraries>` ライブラリ数。
    pub libraries: usize,
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

/// `b` が EAGLE XML かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.layers >= 1 && c.entries - c.misc >= 3)
}

/// `b` を EAGLE XML として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    if it.first().copied() != Some("eagle") {
        return None;
    }
    it.remove(0);
    let mut c = Counts {
        entries: 0,
        layers: 0,
        parts: 0,
        signals: 0,
        wires: 0,
        pads: 0,
        libraries: 0,
        misc: 0,
    };
    for name in it {
        c.entries += 1;
        match name {
            "layer" => c.layers += 1,
            "part" | "element" | "instance" => c.parts += 1,
            "signal" | "net" | "bus" => c.signals += 1,
            "wire" | "junction" | "label" | "pinref" | "segment" => c.wires += 1,
            "pad" | "smd" | "via" | "hole" => c.pads += 1,
            "package" | "packages" | "deviceset" | "device" | "symbol" | "symbols" | "library"
            | "libraries" => c.libraries += 1,
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
    <!DOCTYPE eagle SYSTEM "eagle.dtd">
    <eagle version="9.6.2">
    <drawing>
    <settings><setting alwaysvectorfont="no"/><grid distance="0.1" unitdist="inch" unit="inch"/></settings>
    <layers>
    <layer number="1" name="Top" color="4" fill="1" visible="yes" active="yes"/>
    <layer number="16" name="Bottom" color="1" fill="1" visible="yes" active="yes"/>
    <layer number="20" name="Dimension" color="24" fill="1" visible="yes" active="yes"/>
    </layers>
    <board>
    <plain><wire x1="0" y1="0" x2="10" y2="0" width="0" layer="20"/></plain>
    <libraries>
    <library name="rcl"><packages>
    <package name="R0603"><pad name="1" x="-0.8" y="0" drill="0" shape="square"/></package>
    <package name="C0603"><smd name="1" x="-0.8" y="0" dx="0.9" dy="0.8" layer="1"/></package>
    </packages></library>
    </libraries>
    <elements>
    <element name="R1" library="rcl" package="R0603" value="10k" x="5" y="5"/>
    <element name="C1" library="rcl" package="C0603" value="100n" x="8" y="5"/>
    </elements>
    <signals>
    <signal name="GND">
    <wire x1="5" y1="5" x2="5" y2="8" width="0.3" layer="16"/>
    <junction x="5" y="8"/>
    <via x="6" y="6" extent="1-16" drill="0.4"/>
    <contactref element="R1" pad="1"/>
    </signal>
    <signal name="VCC">
    <wire x1="8" y1="5" x2="8" y2="8" width="0.3" layer="1"/>
    <hole x="2" y="2" drill="1.0"/>
    </signal>
    </signals>
    </board>
    </drawing>
    </eagle>"#;

    #[test]
    fn detects_eagle() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.layers, 3);
        assert_eq!(c.parts, 2);
        assert_eq!(c.signals, 2);
        assert_eq!(c.wires, 4);
        assert_eq!(c.pads, 4);
        assert_eq!(c.libraries, 5);
        assert!(c.misc >= 5);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<root><a/></root>"#));
        assert!(!detect(b"plain"));
    }
}
