//! Caligari trueSpace `.cob` / `.scn` — object and scene files.
//!
//! Files open with a `Caligari V` banner, a `NN.NN` version, and a
//! three-letter mode whose first letter is `A` (ASCII) or `B`
//! (binary) and whose third letter marks `O`bject/`S`cene/`H`ead.
//! Chunk ids (`Obj1`, `PolH`, `Grp `, `Mat1`, `NAME`, `Came`,
//! `Lght`) appear in both text and binary bodies.
//!
//! ```
//! let d = b"Caligari V00\x2e01ALH\nObj1 V0\x2e06 Id 1\nPolH V0\x2e05\nMat1 V0\x2e07\n";
//! let f = izanagi_kit::cob::parse(d).unwrap();
//! assert_eq!(f.version, "00\x2e01");
//! assert!(!f.binary);
//! assert_eq!(f.objects, 1);
//! assert_eq!(f.materials, 1);
//! ```
//!
//! Reference: Caligari trueSpace COB file format notes (various
//! community mirrors; FileFormat.info COB entry). Integer-only.

/// Parsed `.cob`/`.scn` header + chunk census.
#[derive(Debug, Clone, PartialEq)]
pub struct Cob {
    /// Version digits after `Caligari V` (e.g. `00.01`).
    pub version: String,
    /// Three-letter mode suffix (e.g. `ALH`, `BLH`).
    pub mode: String,
    /// `true` when the mode starts with `B` (binary body).
    pub binary: bool,
    /// `Obj1` object chunks.
    pub objects: u32,
    /// `PolH` polygon chunks.
    pub polygons: u32,
    /// `Grp ` group chunks.
    pub groups: u32,
    /// `Mat1` material chunks.
    pub materials: u32,
    /// `NAME` name chunks.
    pub names: u32,
    /// `Came` camera chunks.
    pub cameras: u32,
    /// `Lght` light chunks.
    pub lights: u32,
}

fn count(d: &[u8], tok: &[u8]) -> u32 {
    if d.len() < tok.len() {
        return 0;
    }
    d.windows(tok.len()).filter(|w| *w == tok).count() as u32
}

/// Parse the `Caligari V` banner and census known chunk ids.
/// `None` when the banner or its mode suffix is absent.
pub fn parse(d: &[u8]) -> Option<Cob> {
    if !d.starts_with(b"Caligari V") {
        return None;
    }
    let mut ve = 10;
    while ve < d.len() && (d[ve].is_ascii_digit() || d[ve] == b'.') {
        ve += 1;
    }
    if ve == 10 || !d[10..ve].contains(&b'.') || ve + 3 > d.len() {
        return None;
    }
    let mode = &d[ve..ve + 3];
    if !(mode[0] == b'A' || mode[0] == b'B')
        || !mode[1].is_ascii_alphabetic()
        || !mode[2].is_ascii_alphabetic()
    {
        return None;
    }
    Some(Cob {
        version: core::str::from_utf8(&d[10..ve]).ok()?.to_string(),
        mode: core::str::from_utf8(mode).ok()?.to_string(),
        binary: mode[0] == b'B',
        objects: count(d, b"Obj1"),
        polygons: count(d, b"PolH"),
        groups: count(d, b"Grp "),
        materials: count(d, b"Mat1"),
        names: count(d, b"NAME"),
        cameras: count(d, b"Came"),
        lights: count(d, b"Lght"),
    })
}

/// `true` when the `Caligari V` banner is present.
pub fn detect(d: &[u8]) -> bool {
    d.starts_with(b"Caligari V")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"Caligari V00\x2e01ALH\nObj1 V0\x2e06 Id 1\nPolH V0\x2e05\nMat1 V0\x2e07\nNAME foo\n";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.version, "00\x2e01");
        assert_eq!(f.mode, "ALH");
        assert!(!f.binary);
        assert_eq!(f.objects, 1);
        assert_eq!(f.polygons, 1);
        assert_eq!(f.materials, 1);
        assert_eq!(f.names, 1);
        assert_eq!(f.groups, 0);
        assert_eq!(f.cameras, 0);
        assert_eq!(f.lights, 0);
    }

    #[test]
    fn binary_and_scene_modes() {
        let f = parse(b"Caligari V00\x2e01BLH\x00\x01Obj1Obj1").unwrap();
        assert!(f.binary);
        assert_eq!(f.objects, 2);
        let s = parse(b"Caligari V00\x2e01ASH\nGrp  V0\x2e01\nLght V0\x2e01\n").unwrap();
        assert_eq!(s.mode, "ASH");
        assert_eq!(s.groups, 1);
        assert_eq!(s.lights, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"Caligari V").is_none()); // no version/mode
        assert!(parse(b"Caligari V0001ALH").is_none()); // no dot
        assert!(parse(b"Caligari V00\x2e01XLH").is_none()); // bad mode letter
        assert!(parse(b"NotCaligari V00\x2e01ALH").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"caligari v00\x2e01alh"));
    }
}
