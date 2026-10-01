//! REAPER プロジェクト `.rpp` ファイルの検出・カウント。
//!
//! `<REAPER_PROJECT ver "name" timestamp` で始まる行指向テキスト形式。
//! `<TRACK … >` トラックブロック、`<ITEM` アイテム、`<SOURCE` ソース、
//! `VST `/`AU `/`JS `/`DX `/`LV2 ` エフェクト行、`MARKER ` マーカーを分類する。
//!
//! ```
//! let rpp = br#"<REAPER_PROJECT 0.1 "7.0" 1700000000
//!   SAMPLERATE 48000 0 0
//!   <TRACK
//!     NAME lead
//!     <ITEM
//!       POSITION 0.0
//!       <SOURCE WAVE
//!         FILE "lead.wav"
//!       >
//!     >
//!   >
//! >"#;
//! assert!(izanagi_kit::reaper::detect(rpp));
//! let c = izanagi_kit::reaper::parse(rpp).unwrap();
//! assert_eq!(c.tracks, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// エントリ総数 (ブロック・設定行)。
    pub entries: usize,
    /// `<TRACK` トラックブロック数。
    pub tracks: usize,
    /// `<ITEM` アイテム数。
    pub items: usize,
    /// エフェクト行数 (`VST`/`AU`/`JS`/`DX`/`LV2`/`CLAP`)。
    pub fx: usize,
    /// `MARKER` マーカー・リージョン行数。
    pub markers: usize,
    /// `<SOURCE` メディアソースブロック数。
    pub sources: usize,
    /// その他行数 (設定・終端 `>` など)。
    pub misc: usize,
}

/// `b` が REAPER プロジェクトかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.tracks >= 1 || c.items >= 1 || c.fx >= 1)
}

/// `b` を REAPER `.rpp` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    let mut lines = text.lines();
    let head = lines.next()?.trim_start();
    if !head.starts_with("<REAPER_PROJECT") {
        return None;
    }
    let mut c = Counts {
        entries: 0,
        tracks: 0,
        items: 0,
        fx: 0,
        markers: 0,
        sources: 0,
        misc: 0,
    };
    for line in lines {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        c.entries += 1;
        if t.starts_with("<TRACK") {
            c.tracks += 1;
        } else if t.starts_with("<ITEM") {
            c.items += 1;
        } else if t.starts_with("<SOURCE") {
            c.sources += 1;
        } else if t.starts_with("MARKER ") || t == "MARKER" {
            c.markers += 1;
        } else if t.starts_with("VST ")
            || t.starts_with("AU ")
            || t.starts_with("JS ")
            || t.starts_with("DX ")
            || t.starts_with("LV2 ")
            || t.starts_with("CLAP ")
            || t.starts_with("<FXCHAIN")
        {
            c.fx += 1;
        } else {
            c.misc += 1;
        }
    }
    if c.entries >= 3 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"<REAPER_PROJECT 0.1 "7.06" 1700000000
  RIPPLE 0
  GROUPOVERRIDE 0 0 0
  AUTOXFADE 1
  ENVATTACH 3
  SAMPLERATE 48000 0 0
  <RENDER_CFG
    ZXZhdxgAAA==
  >
  <TRACK {AAAABBBB-0000-1111-2222-333344445555}
    NAME drums
    PEAKCOL 16576
    VOLPAN 1.0 0.0 -1.0 -1.0 1.0
    MUTESOLO 0 0 0
    <FXCHAIN
      WNDRECT 0 0 0 0
      SHOW 0
      LASTSEL 0
      DOCKED 0
      BYPASS 0 0 0
      VST "VST: ReaEQ (Cockos)" reaeq.vst.so 0 ""
      FLOATPOS 0 0 0 0
      FXID {12345678-9ABC-DEF0-1234-56789ABCDEF0}
      WAK 0 0
    >
    <ITEM
      POSITION 0.0
      LENGTH 4.0
      NAME "kick.wav"
      <SOURCE WAVE
        FILE "media/kick.wav"
      >
    >
    <ITEM
      POSITION 4.0
      LENGTH 4.0
      <SOURCE MIDI
        HASDATA 1 960 QN
      >
    >
  >
  MARKER 1 0 "start" 0 0 1
  MARKER 2 32 "end" 0 0 1
>"#;

    #[test]
    fn detects_rpp() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.tracks, 1);
        assert_eq!(c.items, 2);
        assert_eq!(c.sources, 2);
        assert_eq!(c.fx, 2);
        assert_eq!(c.markers, 2);
        assert!(c.misc >= 8);
    }

    #[test]
    fn rejects_other_text() {
        assert!(!detect(b"[section]\nkey=value\n"));
        assert!(!detect(b"plain"));
    }
}
