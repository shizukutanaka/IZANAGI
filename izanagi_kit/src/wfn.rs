//! Gaussian `.wfn` wavefunction file — the AIM-format text dump of a
//! quantum-chemistry calculation. Line 1 is the title; line 2 is the
//! `GAUSSIAN   <nmo> MOL ORBITALS   <nprim> PRIMITIVES   <natoms>
//! NUCLEI` header (variants also allow `GTO`/`NATOMS` wording), then
//! centre, type, exponent and orbital blocks.
//!
//! `parse` reads the three integer counts from line 2; all other
//! fields stay text.
//!
//! ```
//! let f = b"title\nGAUSSIAN          5 MOL ORBITALS    21 PRIMITIVES        3 NUCLEI\n";
//! let w = izanagi_kit::wfn::parse(f).unwrap();
//! assert_eq!(w.nmo, 5);
//! assert_eq!(w.nprim, 21);
//! assert_eq!(w.natoms, 3);
//! assert!(izanagi_kit::wfn::parse(b"t\nx y z\n").is_none());
//! ```

/// Parsed `.wfn` header.
#[derive(Debug, Clone, PartialEq)]
pub struct Wfn {
    /// Molecular-orbital count.
    pub nmo: usize,
    /// Primitive count.
    pub nprim: usize,
    /// Nucleus count.
    pub natoms: usize,
}

/// Parse a `.wfn`; `None` without the line-2 count pattern.
pub fn parse(d: &[u8]) -> Option<Wfn> {
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines();
    let _title = lines.next()?;
    let hdr = lines.next()?;
    let toks: Vec<&str> = hdr.split_whitespace().collect();
    // `GAUSSIAN <nmo> MOL ORBITALS <nprim> PRIMITIVES <natoms> NUCLEI`
    if toks.len() < 7 || !toks[0].eq_ignore_ascii_case("GAUSSIAN") {
        return None;
    }
    if !toks[2].eq_ignore_ascii_case("MOL")
        || !toks[3].eq_ignore_ascii_case("ORBITALS")
        || !toks[5].eq_ignore_ascii_case("PRIMITIVES")
    {
        return None;
    }
    let last = toks.last()?;
    if !last.eq_ignore_ascii_case("NUCLEI") && !last.eq_ignore_ascii_case("ATOMS") {
        return None;
    }
    Some(Wfn {
        nmo: toks[1].parse().ok()?,
        nprim: toks[4].parse().ok()?,
        natoms: toks[6].parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"mol\nGAUSSIAN 10 MOL ORBITALS 30 PRIMITIVES 4 NUCLEI\n";
        let w = parse(f).unwrap();
        assert_eq!(w.nmo, 10);
        assert_eq!(w.natoms, 4);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"title only\n").is_none());
        assert!(parse(b"t\nGAUSSIAN 5 XX ORBITALS 3 PRIMITIVES 1 NUCLEI\n").is_none());
    }
}
