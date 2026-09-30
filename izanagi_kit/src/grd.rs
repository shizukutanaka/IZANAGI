//! Golden Software Surfer grid (`.grd`): the ASCII form begins with
//! `DSAA`, then `nx ny`, `xmin xmax`, `ymin ymax`, `zmin zmax` lines
//! (values kept as text tokens — no floats); the binary v6+ form
//! begins with `DSRB` + u32 header-size/version pair.
//!
//! ```
//! let d = b"DSAA\n4 3\n0.0 4.0\n0.0 3.0\n0.0 9.0\n1 2 3 4\n";
//! let g = izanagi_kit::grd::parse(d).unwrap();
//! assert_eq!(g.kind, izanagi_kit::grd::Kind::Ascii);
//! assert_eq!(g.nx, 4);
//! assert_eq!(g.ny, 3);
//! ```

/// Which Surfer grid encoding a file uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `DSAA` ASCII grid.
    Ascii,
    /// `DSRB` binary grid (v6+).
    Binary,
}

/// A parsed Surfer grid header.
#[derive(Clone, Debug)]
pub struct Grd {
    /// Encoding.
    pub kind: Kind,
    /// Column count.
    pub nx: u64,
    /// Row count.
    pub ny: u64,
    /// `xmin xmax` range verbatim text (ASCII kind only).
    pub xrange: Option<(std::string::String, std::string::String)>,
    /// `ymin ymax` range verbatim.
    pub yrange: Option<(std::string::String, std::string::String)>,
    /// `zmin zmax` range verbatim.
    pub zrange: Option<(std::string::String, std::string::String)>,
}

fn be_or_le_hdr(d: &[u8]) -> Option<(u64, u64)> {
    // Binary v6: "DSRB" + u32 header_len + u32 version, then the
    // grid section with nx/ny little-endian u32s at fixed offsets
    // (header_len 40? actually a sub-header). Read the first two
    // ints after the signature as header_len/version.
    if d.len() < 16 {
        return None;
    }
    let hlen = u32::from_le_bytes(d[4..8].try_into().ok()?);
    let ver = u32::from_le_bytes(d[8..12].try_into().ok()?);
    if hlen == 0 || ver == 0 || hlen > 1024 {
        return None;
    }
    // nx/ny live at 12..20 in the section header for simple files.
    let nx = u32::from_le_bytes(d[12..16].try_into().ok()?);
    Some((u64::from(nx), 0))
}

/// Parse a `.grd`; `None` for non-Surfer inputs.
pub fn parse(d: &[u8]) -> Option<Grd> {
    if d.starts_with(b"DSRB") {
        let (nx, _v) = be_or_le_hdr(d)?;
        // ny follows nx in the section record
        let ny = d
            .get(16..20)
            .and_then(|b| b.try_into().ok())
            .map(|a| u64::from(u32::from_le_bytes(a)))
            .unwrap_or(0);
        if nx == 0 {
            return None;
        }
        return Some(Grd {
            kind: Kind::Binary,
            nx,
            ny,
            xrange: None,
            yrange: None,
            zrange: None,
        });
    }
    if !d.starts_with(b"DSAA") {
        return None;
    }
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next()? != "DSAA" {
        return None;
    }
    let dims: Vec<u64> = lines
        .next()?
        .split_whitespace()
        .filter_map(|w| w.parse().ok())
        .collect();
    if dims.len() != 2 {
        return None;
    }
    let range = |lines: &mut dyn Iterator<Item = &str>| {
        let l = lines.next()?;
        let mut it = l.split_whitespace();
        let a = it.next()?.to_string();
        let b = it.next()?.to_string();
        Some((a, b))
    };
    let xrange = range(&mut lines);
    let yrange = range(&mut lines);
    let zrange = range(&mut lines);
    if xrange.is_none() || yrange.is_none() || zrange.is_none() {
        return None;
    }
    Some(Grd {
        kind: Kind::Ascii,
        nx: dims[0],
        ny: dims[1],
        xrange,
        yrange,
        zrange,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii() {
        let d = b"DSAA\n4 3\n0.0 4.0\n0.0 3.0\n-1.5 9.25\n1 2 3 4 5 6\n";
        let g = parse(d).unwrap();
        assert_eq!(g.kind, Kind::Ascii);
        assert_eq!(g.nx, 4);
        assert_eq!(g.ny, 3);
        assert_eq!(g.zrange.as_ref().unwrap().1, "9.25");
    }

    #[test]
    fn binary() {
        let mut d = b"DSRB".to_vec();
        d.extend_from_slice(&40u32.to_le_bytes()); // header len
        d.extend_from_slice(&1u32.to_le_bytes()); // version
        d.extend_from_slice(&10u32.to_le_bytes()); // nx
        d.extend_from_slice(&7u32.to_le_bytes()); // ny
        let g = parse(&d).unwrap();
        assert_eq!(g.kind, Kind::Binary);
        assert_eq!(g.nx, 10);
        assert_eq!(g.ny, 7);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"GRID\n1 1\n").is_none());
        assert!(parse(b"DSAA\nx y\n").is_none());
        assert!(parse(b"DSAA\n2 2\n0 1\n").is_none()); // truncated ranges
        assert!(parse(b"DSRB").is_none()); // truncated binary
    }
}
