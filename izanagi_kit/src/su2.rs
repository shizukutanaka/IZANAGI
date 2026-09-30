//! SU2 CFD mesh file (`.su2`): flat text — `NDIME= n`,
//! `NELEM= n` + element connectivity lines, `NPOIN= n` + node
//! coordinates, then `NMARK= n` and `MARKER_TAG= name` /
//! `MARKER_ELEMS= n` per boundary marker.
//!
//! ```
//! let d = b"NDIME= 2\nNELEM= 1\n9 0 1 2 0\nNPOIN= 3\n0 0 0\n1 0 1\n0 1 2\nNMARK= 1\nMARKER_TAG= wall\nMARKER_ELEMS= 1\n3 0 1\n";
//! let s = izanagi_kit::su2::parse(d).unwrap();
//! assert_eq!(s.ndime, 2);
//! assert_eq!(s.nelem, 1);
//! assert_eq!(s.npoin, 3);
//! assert_eq!(s.markers[0].0, "wall");
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed SU2 mesh summary.
#[derive(Clone, Debug)]
pub struct Su2 {
    /// `NDIME` — problem dimension (2 or 3).
    pub ndime: u64,
    /// `NELEM` — interior element count.
    pub nelem: u64,
    /// `NPOIN` — node count.
    pub npoin: u64,
    /// Boundary markers `(tag, MARKER_ELEMS)` in file order.
    pub markers: Vec<(String, u64)>,
}

fn get_num(t: &str) -> Option<u64> {
    let (_, v) = t.split_once('=')?;
    v.trim().parse().ok()
}

/// Parse an SU2 mesh; `None` without `NDIME=` + `NPOIN=`/`NELEM=`.
pub fn parse(d: &[u8]) -> Option<Su2> {
    let s = std::str::from_utf8(d).ok()?;
    let mut ndime = None;
    let mut nelem = None;
    let mut npoin = None;
    let mut markers: Vec<(String, u64)> = Vec::new();
    let mut pending_tag: Option<String> = None;
    for line in s.lines() {
        let t = line.trim();
        let u = t.to_uppercase();
        if u.starts_with("NDIME") {
            ndime = get_num(t);
        } else if u.starts_with("NELEM") {
            nelem = get_num(t);
        } else if u.starts_with("NPOIN") {
            npoin = get_num(t);
        } else if u.starts_with("MARKER_TAG") {
            if let Some((_, v)) = t.split_once('=') {
                pending_tag = Some(v.trim().to_string());
            }
        } else if u.starts_with("MARKER_ELEMS") {
            if let (Some(tag), Some(n)) = (pending_tag.take(), get_num(t)) {
                markers.push((tag, n));
            }
        }
    }
    let ndime = ndime?;
    if !(2..=3).contains(&ndime) || nelem.is_none() && npoin.is_none() {
        return None;
    }
    Some(Su2 {
        ndime,
        nelem: nelem.unwrap_or(0),
        npoin: npoin.unwrap_or(0),
        markers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let d = b"NDIME= 3\nNELEM= 2\nNPOIN= 5\nNMARK= 2\nMARKER_TAG= inlet\nMARKER_ELEMS= 3\nMARKER_TAG= outlet\nMARKER_ELEMS= 3\n";
        let s = parse(d).unwrap();
        assert_eq!(s.ndime, 3);
        assert_eq!(s.nelem, 2);
        assert_eq!(s.npoin, 5);
        assert_eq!(
            s.markers,
            vec![("inlet".to_string(), 3), ("outlet".to_string(), 3)]
        );
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"NDIME= 5\nNELEM= 1\n").is_none()); // bad dim
        assert!(parse(b"NDIME= 2\n").is_none()); // no elements/points
    }
}
