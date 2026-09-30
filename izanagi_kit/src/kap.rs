//! BSB/KAP — Maptech raster nautical chart header.
//!
//! Text prolog: `!` comment lines, then a `BSB/` general-parameters line
//! (`NA=`, `NU=`, `RA=w,h`, `DU=`) and a `VER/` version line before the
//! binary image strip.
//!
//! ```
//! let d = b"! produced by test\nBSB/NA=CHART1,NU=1,RA=800,600,DU=300\nVER/2.0\n";
//! let k = izanagi_kit::kap::parse(d).unwrap();
//! assert_eq!(k.name, "CHART1");
//! assert_eq!(k.ra, Some((800, 600)));
//! ```

/// Parsed KAP chart header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kap {
    /// `NA=` chart name.
    pub name: String,
    /// `NU=` chart number.
    pub number: u32,
    /// `RA=` raster size (width, height) in pixels.
    pub ra: Option<(u32, u32)>,
    /// `DU=` resolution (dots per unit inch).
    pub dpi: u32,
    /// `VER/` chart format version text (e.g. `2\x2e0`).
    pub ver: String,
    /// Header keyword lines seen (`BSB`, `KNP`, `KNQ`, `CED`, …), in order.
    pub sections: Vec<String>,
}

/// `KEY=` value inside a `BSB/`-style parameter line.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pos = line.find(key)?;
    let rest = &line[pos + key.len()..];
    let end = rest.find(',').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Parse a KAP header; `None` without `BSB/` and `VER/` lines.
pub fn parse(d: &[u8]) -> Option<Kap> {
    let s = std::str::from_utf8(d).ok()?;
    let mut name = None;
    let mut number = 0u32;
    let mut ra = None;
    let mut dpi = 0u32;
    let mut ver = None;
    let mut sections = Vec::new();
    for raw in s.lines() {
        let line = raw.trim();
        if line.starts_with('!') || line.is_empty() {
            continue;
        }
        if let Some(slash) = line.find('/') {
            let key = &line[..slash];
            if !key
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            {
                break; // binary strip begins
            }
            sections.push(key.to_string());
            match key {
                "BSB" => {
                    let rest = &line[slash + 1..];
                    name = field(rest, "NA=").map(str::to_string);
                    number = field(rest, "NU=").and_then(|v| v.parse().ok()).unwrap_or(0);
                    dpi = field(rest, "DU=").and_then(|v| v.parse().ok()).unwrap_or(0);
                    // RA takes two comma-separated ints, so it can't use
                    // the generic `KEY=` (first-comma-terminated) reader.
                    let ra_pos = rest
                        .find(",RA=")
                        .map(|i| i + 4)
                        .or_else(|| rest.strip_prefix("RA=").map(|_| 0));
                    ra = ra_pos.and_then(|i| {
                        let v = &rest[i..];
                        let (w, h) = v.split_once(',')?;
                        let w = w.trim().parse().ok()?;
                        let h_end = h.find(|c: char| !c.is_ascii_digit()).unwrap_or(h.len());
                        Some((w, h[..h_end].parse().ok()?))
                    });
                }
                "VER" => ver = Some(line[slash + 1..].trim().to_string()),
                _ => {}
            }
        }
    }
    if sections.iter().all(|s| s != "BSB") {
        return None;
    }
    Some(Kap {
        name: name?,
        number,
        ra,
        dpi,
        ver: ver?,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"! comment\nBSB/NA=TEST,NU=12,RA=1024,768,DU=254\nKNP/SC=25000\nVER/3.07\n";
        let k = parse(d).unwrap();
        assert_eq!(k.name, "TEST");
        assert_eq!(k.number, 12);
        assert_eq!(k.ra, Some((1024, 768)));
        assert_eq!(k.dpi, 254);
        assert_eq!(k.ver, "3.07");
        assert_eq!(k.sections, vec!["BSB", "KNP", "VER"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"VER/3.0\n").is_none()); // no BSB
        assert!(parse(b"BSB/RA=1,1\n").is_none()); // no NA, no VER
    }
}
