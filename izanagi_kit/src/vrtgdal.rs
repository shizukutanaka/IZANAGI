//! `.vrt` (GDAL Virtual Dataset XML) 検出モジュール。
//!
//! GDAL VRT は XML で、ルート `<VRTDataset rasterXSize="..."
//! rasterYSize="...">` と `<SRS>`/`<GeoTransform>`/`<VRTRasterBand>`/
//! `<SimpleSource>`/`<ComplexSource>`/`<SourceFilename>`/
//! `<SourceBand>`/`<SrcRect>`/`<DstRect>`/`<ColorInterp>`/
//! `<NoDataValue>`/`<LUT>`/`<KernelFilteredSource>` 要素で構成される。
//!
//! ```
//! let b = br#"<VRTDataset rasterXSize="512" rasterYSize="512">
//!   <SRS>EPSG:4326</SRS>
//!   <GeoTransform>-180, 0.35, 0, 90, 0, -0.35</GeoTransform>
//!   <VRTRasterBand dataType="Byte" band="1">
//!     <SimpleSource>
//!       <SourceFilename relativeToVRT="1">a.tif</SourceFilename>
//!     </SimpleSource>
//!   </VRTRasterBand>
//! </VRTDataset>"#;
//! let c = izanagi_kit::vrtgdal::parse(b);
//! assert!(izanagi_kit::vrtgdal::detect(b));
//! assert_eq!(c.elements, 9);
//! ```

const ELEMENTS: &[&str] = &[
    "AveragedSource",
    "BlockXSize",
    "BlockYSize",
    "CategoryNames",
    "ColorInterp",
    "ColorTable",
    "ComplexSource",
    "CoordinateTransformation",
    "Description",
    "DstRect",
    "Entry",
    "GeoTransform",
    "HideNoDataValue",
    "KernelFilteredSource",
    "LUT",
    "MaskBand",
    "MaxValue",
    "Minimum",
    "Maximum",
    "NoDataValue",
    "NODATA",
    "Offset",
    "OpenOptions",
    "Overview",
    "pixelFunctionType",
    "RasterXSize",
    "Scale",
    "ScaleRatio",
    "SimpleSource",
    "SourceBand",
    "SourceDataset",
    "SourceFilename",
    "SourceProperties",
    "SourceTransferType",
    "SrcRect",
    "SRS",
    "UnitType",
    "VRTDataset",
    "VRTRasterBand",
    "VRTWarpedDataset",
    "VRTWarpedDatasetItem",
    "VRTWarpedSource",
    "WarpedSource",
];

fn vrt_elem(t: &str) -> bool {
    let inner = t.trim_start_matches('<').trim_start_matches('/');
    let name: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    ELEMENTS.contains(&name.as_str()) && inner[name.len()..].starts_with([' ', '>', '/', '\t'])
}

/// `b` が GDAL VRT に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut root = 0usize;
    let mut elems = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if !tr.starts_with('<') {
            continue;
        }
        if tr.starts_with("<VRTDataset") || tr.starts_with("<VRTWarpedDataset") {
            root += 1;
            elems += 1;
            continue;
        }
        if vrt_elem(tr) {
            elems += 1;
        }
    }
    (root >= 1 && elems >= 3) || elems >= 6
}

/// GDAL VRT の統計。
#[derive(Debug, Default, Clone)]
pub struct VrtGdal {
    /// 既知要素行数。
    pub elements: usize,
    /// `<VRTDataset` ルート行数。
    pub root: usize,
}

/// `b` を GDAL VRT として統計する。
pub fn parse(b: &[u8]) -> VrtGdal {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = VrtGdal::default();
    for l in t.lines() {
        let tr = l.trim();
        if !tr.starts_with('<') {
            continue;
        }
        if tr.starts_with("<VRTDataset") || tr.starts_with("<VRTWarpedDataset") {
            c.root += 1;
            c.elements += 1;
            continue;
        }
        if vrt_elem(tr) {
            c.elements += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"<VRTDataset rasterXSize="512" rasterYSize="512">
<VRTRasterBand dataType="Byte" band="1">
</VRTRasterBand>
</VRTDataset>"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.root, 1);
    }

    #[test]
    fn detects_warped() {
        let b = br#"<VRTWarpedDataset>
<SourceDataset>x.tif</SourceDataset>
<VRTWarpedDatasetItem/>
</VRTWarpedDataset>"#;
        assert!(detect(b));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"<VRTDataset>\n"));
        assert!(!detect(b"<html><body>hi</body></html>"));
        assert!(!detect(b"VRTRasterBand band=1\nSourceFilename=a.tif\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.elements, 0);
    }
}
