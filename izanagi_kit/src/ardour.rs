//! Ardour セッション `.ardour` (XML) の検出・カウント。
//!
//! `<Session version="…" sample-rate="…">` ルート。
//! `<Route>` トラック/バス、`<Region>` リージョン、`<Playlist>` プレイリスト、
//! `<Location>` マーカー、`<Plugin>` プラグイン、`<Source>` ソースを分類する。
//!
//! ```
//! let xml = br#"<Session version="7001" sample-rate="48000">
//!               <Sources><Source name="kick.wav"/></Sources>
//!               <Regions><Region name="kick-1"/></Regions>
//!               <Playlists><Playlist name="drums"/></Playlists>
//!               <Routes><Route name="drums"><Processor><Plugin type="lv2"/></Processor></Route></Routes>
//!               </Session>"#;
//! assert!(izanagi_kit::ardour::detect(xml));
//! let c = izanagi_kit::ardour::parse(xml).unwrap();
//! assert_eq!(c.routes, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 開始タグ総数。
    pub entries: usize,
    /// `<Route>` トラック・バス数。
    pub routes: usize,
    /// `<Region>` リージョン数。
    pub regions: usize,
    /// `<Playlist>` プレイリスト数。
    pub playlists: usize,
    /// `<Location>` ロケーション・マーカー数。
    pub locations: usize,
    /// `<Plugin>` プラグイン数。
    pub plugins: usize,
    /// `<Source>`/`<Resource>` ソース数。
    pub sources: usize,
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

/// `b` が Ardour セッションかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 2)
}

/// `b` を Ardour `.ardour` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut it = tags(text);
    if it.first().copied() != Some("Session") {
        return None;
    }
    it.remove(0);
    let mut c = Counts {
        entries: 0,
        routes: 0,
        regions: 0,
        playlists: 0,
        locations: 0,
        plugins: 0,
        sources: 0,
        misc: 0,
    };
    for name in it {
        c.entries += 1;
        match name {
            "Route" => c.routes += 1,
            "Region" => c.regions += 1,
            "Playlist" => c.playlists += 1,
            "Location" => c.locations += 1,
            "Plugin" => c.plugins += 1,
            "Source" | "Resource" => c.sources += 1,
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
    <Session version="7001" sample-rate="48000" id-counter="103">
    <Config/>
    <Metadata/>
    <Sources>
    <Source name="kick.wav" type="audio"/>
    <Source name="snare.wav" type="audio"/>
    </Sources>
    <Regions>
    <Region name="kick-1" start="0" length="192000"/>
    <Region name="snare-1" start="96000" length="96000"/>
    </Regions>
    <Playlists>
    <Playlist name="drums" orig-track-id="9">
    <Region name="kick-1"/>
    </Playlist>
    <Playlist name="bass" orig-track-id="10"/>
    </Playlists>
    <Routes>
    <Route name="drums" id="9">
    <PresentationInfo/>
    <IO>
    <Port type="audio" name="drums/audio_in 1"/>
    </IO>
    <Processor><Plugin type="lv2" id="100"/></Processor>
    <Processor><Plugin type="ladspa" id="1042"/></Processor>
    </Route>
    <Route name="bass" id="10"/>
    <Route name="master" id="1" default-play="1"/>
    </Routes>
    <Locations>
    <Location name="session" start="0" end="384000" flags="IsSessionRange"/>
    </Locations>
    <Bundles/>
    <Monitor/>
    </Session>"#;

    #[test]
    fn detects_ardour() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.routes, 3);
        assert_eq!(c.regions, 3);
        assert_eq!(c.playlists, 2);
        assert_eq!(c.locations, 1);
        assert_eq!(c.plugins, 2);
        assert_eq!(c.sources, 2);
        assert!(c.misc >= 6);
    }

    #[test]
    fn rejects_other_xml() {
        assert!(!detect(br#"<root><a/></root>"#));
        assert!(!detect(b"plain"));
    }
}
