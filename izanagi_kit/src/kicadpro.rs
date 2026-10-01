//! KiCad プロジェクト `.kicad_pro` (JSON) の検出・カウント。
//!
//! `board`/`pcbnew`/`schematic`/`sheets`/`cvpcb`/`libraries`/`text_variables`
//! 等 KiCad 固有のトップレベルキーを持つ JSON 設定。
//!
//! ```
//! let cfg = br#"{"meta":{"version":1},"board":{"design_settings":{}},
//!                "schematic":{},"sheets":[],"text_variables":{},"libraries":{}}"#;
//! assert!(izanagi_kit::kicadpro::detect(cfg));
//! let c = izanagi_kit::kicadpro::parse(cfg).unwrap();
//! assert_eq!(c.entries, 8);
//! ```

/// KiCad 既知キー。
const KNOWN_KEYS: &[&str] = &[
    "meta",
    "version",
    "generator",
    "board",
    "boards",
    "design_settings",
    "page_layout_descr_file",
    "pcbnew",
    "schematic",
    "sheets",
    "sheet",
    "cvpcb",
    "libraries",
    "library",
    "netlist",
    "erc",
    "bom",
    "text_variables",
    "variables",
    "footprint_filters",
    "footprint_filter",
    "plot_settings",
    "net_settings",
    "schematic_paths",
    "page_layout",
    "title_block",
    "comment_1",
    "comment_2",
    "comment_3",
    "comment_4",
    "comment_5",
    "comment_6",
    "comment_7",
    "comment_8",
    "comment_9",
    "field_fabrication",
    "field_value",
    "field_datasheet",
    "rules",
    "severity",
    "legacy_editing",
    "legacy_lib_dir",
    "last_paths",
    "open_project_files",
    "project",
    "env_var",
    "3d_viewer",
    "sym_lib_table",
    "fp_lib_table",
];

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// JSON キートークン総数。
    pub entries: usize,
    /// `board`/`design_settings`/`page_layout*`/`net_settings`/`plot_settings` 等ボード系キー数。
    pub board: usize,
    /// `schematic`/`sheets`/`sheet`/`cvpcb`/`erc`/`bom`/`schematic_paths` 等回路図系キー数。
    pub schematic: usize,
    /// `libraries`/`library`/`netlist`/`footprint_filter*`/`sym_lib_table`/`fp_lib_table` 等ライブラリ系キー数。
    pub libraries: usize,
    /// `meta`/`version`/`generator`/`project`/`title_block`/`comment_*`/`text_variables`/`variables` 等メタ・変数系キー数。
    pub meta: usize,
    /// その他キー数。
    pub misc: usize,
}

/// `b` が `.kicad_pro` かどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.entries - c.misc >= 3)
}

/// キートークンを走査 (`"name"` の直後が `:` のもの)。
fn keys(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            let start = i + 1;
            let mut j = start;
            let mut esc = false;
            while j < bytes.len() {
                if esc {
                    esc = false;
                } else if bytes[j] == 92 {
                    esc = true;
                } else if bytes[j] == b'"' {
                    break;
                }
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            let mut k = j + 1;
            while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' {
                let name = &text[start..j];
                if !name.is_empty() {
                    out.push(name);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    out
}

/// `b` を `.kicad_pro` として解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    if !text.trim_start().starts_with('{') {
        return None;
    }
    let mut c = Counts {
        entries: 0,
        board: 0,
        schematic: 0,
        libraries: 0,
        meta: 0,
        misc: 0,
    };
    for key in keys(text) {
        c.entries += 1;
        if !KNOWN_KEYS.contains(&key) {
            c.misc += 1;
            continue;
        }
        if matches!(
            key,
            "board"
                | "boards"
                | "design_settings"
                | "page_layout_descr_file"
                | "pcbnew"
                | "net_settings"
                | "plot_settings"
                | "page_layout"
                | "3d_viewer"
                | "legacy_editing"
                | "last_paths"
        ) {
            c.board += 1;
        } else if matches!(
            key,
            "schematic" | "sheets" | "sheet" | "cvpcb" | "erc" | "bom" | "schematic_paths"
        ) {
            c.schematic += 1;
        } else if matches!(
            key,
            "libraries"
                | "library"
                | "netlist"
                | "footprint_filters"
                | "footprint_filter"
                | "sym_lib_table"
                | "fp_lib_table"
                | "legacy_lib_dir"
        ) {
            c.libraries += 1;
        } else {
            c.meta += 1;
        }
    }
    let hit = c.board + c.schematic + c.libraries + c.meta;
    if hit >= 2 {
        Some(c)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = br#"{
        "board": {"design_settings": {}, "page_layout_descr_file": ""},
        "boards": [],
        "cvpcb": {"equivalence_files": []},
        "libraries": {"pinned_footprint_libs": [], "pinned_symbol_libs": []},
        "meta": {"filename": "x.kicad_pro", "version": 1},
        "net_settings": {},
        "pcbnew": {"last_paths": {}},
        "schematic": {"drawing": {}},
        "sheets": [],
        "text_variables": {}
    }"#;

    #[test]
    fn detects_kicad_pro() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.entries, 19);
        assert_eq!(c.board, 7);
        assert_eq!(c.schematic, 3);
        assert_eq!(c.libraries, 1);
        assert_eq!(c.meta, 3);
        assert_eq!(c.misc, 5);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(br#"{"name":"x","version":1}"#));
        assert!(!detect(b"not json"));
    }
}
