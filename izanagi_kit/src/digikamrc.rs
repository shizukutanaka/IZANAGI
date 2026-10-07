//! `digikamrc` 検出モジュール。
//!
//! digiKam の設定は KDE 風 INI 形式で、`[Album Settings]`/
//! `[AlbumsView]`/`[Camera Behavior]`/`[General Settings]`/
//! `[Image Viewer Settings]`/`[IconView]`/`[Metadata Settings]`/
//! `[ToolTips Settings]`/`[Slideshow Settings]`/
//! `[Collection Settings]`/`[Tag Properties]`/`[ExpoBlending]`/
//! `[Image Quality Settings]`/`[Theme Settings]`/`[Versioning Settings]`
//! 等のセクションと `AlbumPath`/`DefaultIconSize`/`IconShowName`/
//! `IconShowResolution`/`IconShowTags`/`IconShowOverlays`/
//! `PreviewLoadFullImageSize`/`SaveImageTags`/`SaveFaceTags`/
//! `UseXF20`/`AutoTrashItems`/`ShowTrashDialog`/`TrashCan`/
//! `ratingFilter`/`MinimumRating`/`ShowDateTime`/`TagIcon`/
//! `FaceDetector`/`ApplyRotate`/`MetadataWritingMode`/
//! `ReadFromSidecar`/`WritingXMP` 等のキーで構成される。
//!
//! ```
//! let b = b"[Album Settings]\n\
//!           AlbumPath=/home/user/Pictures\n\
//!           AutoTrashItems=true\n\
//!           [IconView]\n\
//!           DefaultIconSize=32\n\
//!           IconShowName=true\n";
//! let c = izanagi_kit::digikamrc::parse(b);
//! assert!(izanagi_kit::digikamrc::detect(b));
//! assert_eq!(c.sections, 2);
//! ```

const SECTIONS: &[&str] = &[
    "[Album Settings]",
    "[AlbumsView]",
    "[Camera Behavior]",
    "[Collection Settings]",
    "[ExpoBlending]",
    "[Face Settings]",
    "[General Settings]",
    "[IconView]",
    "[Image Editor Settings]",
    "[Image Quality Settings]",
    "[Image Viewer Settings]",
    "[Item Properties]",
    "[LightTable Settings]",
    "[Metadata Settings]",
    "[Queue Settings]",
    "[Recycle Bin Settings]",
    "[Rename Settings]",
    "[Slideshow Settings]",
    "[Tag Properties]",
    "[Theme Settings]",
    "[ToolTips Settings]",
    "[Versioning Settings]",
];

const KEYS: &[&str] = &[
    "AlbumPath",
    "ApplyRotate",
    "AutoTrashItems",
    "CameraAutoFilter",
    "CameraAutoRename",
    "CameraConvertJpeg",
    "CameraDelete",
    "CameraDefaultName",
    "CameraFilter",
    "CameraUseTargetIDs",
    "DefaultIconSize",
    "DefaultVaultPath",
    "FaceDetector",
    "IconShowName",
    "IconShowOverlays",
    "IconShowResolution",
    "IconShowTags",
    "IconViewFont",
    "ItemLeftClickAction",
    "MinimumRating",
    "MetadataWritingMode",
    "PreviewLoadFullImageSize",
    "PreviewShowIcons",
    "ratingFilter",
    "ReadFromSidecar",
    "SaveFaceTags",
    "SaveImageTags",
    "ScanAtStart",
    "SearchTextType",
    "ShowDateTime",
    "ShowFormat",
    "ShowTrashDialog",
    "SortImagesBy",
    "TagIcon",
    "TrashCan",
    "UseXF20",
    "UseXMPSidecar",
    "VideoThumbnailGenerator",
    "WritingXMP",
];

fn is_section(t: &str) -> bool {
    SECTIONS.contains(&t)
}

fn key_present(t: &str) -> bool {
    let k = t.split('=').next().unwrap_or("").trim();
    KEYS.contains(&k)
}

/// `b` が digikamrc に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut secs = 0usize;
    let mut keys = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') || tr.starts_with(';') {
            continue;
        }
        if is_section(tr) {
            secs += 1;
        } else if key_present(tr) {
            keys += 1;
        }
    }
    (secs >= 1 && keys >= 2) || keys >= 3 || secs >= 3
}

/// digikamrc の統計。
#[derive(Debug, Default, Clone)]
pub struct Digikamrc {
    /// 既知セクション数。
    pub sections: usize,
    /// 既知キー行数。
    pub keys: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を digikamrc として統計する。
pub fn parse(b: &[u8]) -> Digikamrc {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Digikamrc::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') || tr.starts_with(';') {
            c.comments += 1;
            continue;
        }
        if is_section(tr) {
            c.sections += 1;
        } else if key_present(tr) {
            c.keys += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[IconView]\nDefaultIconSize=32\nIconShowName=true\n";
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.sections, 1);
    }

    #[test]
    fn detects_keys() {
        let b = b"AlbumPath=/pics\nAutoTrashItems=true\nSaveImageTags=true\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"[IconView]\n"));
        assert!(!detect(b"[section]\nkey = x\nfoo = y\n"));
        assert!(!detect(b"AlbumPath=/pics\nAutoTrashItems=true\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.sections, 0);
    }
}
