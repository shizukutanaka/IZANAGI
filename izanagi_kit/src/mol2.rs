//! Tripos MOL2 (`.mol2`) molecule file reader.
//!
//! The file is split into `@<TRIPOS>SECTION` blocks. The
//! `MOLECULE` block carries the name, a counts line
//! (`atoms bonds substructures features sets`), `mol_type` and
//! `charge_type`. The `ATOM` block holds `id name x y z type
//! subst_id subst_name charge` columns — x/y/z are fixed-point
//! returned as milliunits (the kit never interprets floats).
//!
//! ```
//! use izanagi_kit::mol2::parse;
//!
//! let m = "@<TRIPOS>MOLECULE\nbenzene\n 12 12 1 0 0\nSMALL\nUSER_CHARGES\n\n\
//! @<TRIPOS>ATOM\n      1 C1         -1.1862     0.3512     0.0000 C.ar    1 BEN     0.0000\n\
//! @<TRIPOS>BOND\n     1    1    2 ar\n";
//! let p = parse(m.as_bytes()).unwrap();
//! assert_eq!(p.name, "benzene");
//! assert_eq!(p.atoms, 12);
//! assert_eq!(p.bonds, 12);
//! assert_eq!(p.atom_rows.len(), 1);
//! assert_eq!(p.atom_rows[0].x_milli, -1186);
//! assert_eq!(p.atom_rows[0].atom_type, "C.ar");
//! ```

/// One `ATOM` row (numeric id, name, coordinates, type, subst, charge).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtomRow {
    /// Atom id (1-based in the file).
    pub id: u32,
    /// Atom name.
    pub name: String,
    /// x × 1000.
    pub x_milli: i32,
    /// y × 1000.
    pub y_milli: i32,
    /// z × 1000.
    pub z_milli: i32,
    /// Sybyl atom type (e.g. `C.ar`, `N.pl3`).
    pub atom_type: String,
    /// Substructure id.
    pub subst_id: u32,
    /// Substructure name.
    pub subst_name: String,
    /// Charge × 10000.
    pub charge_milli10k: i32,
}

/// Parse `" -12.3456"` style fixed-point to `scale`-units
/// (e.g. scale=1000 → milliunits). Returns `None` on garbage.
fn fixed(s: &str, scale: i32) -> Option<i32> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let (whole, frac) = match t.split_once('.') {
        Some((w, f)) => (w, f),
        None => (t, ""),
    };
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let w: i32 = whole.parse().unwrap_or(0);
    let mut f: i32 = 0;
    let mut mul = scale;
    for c in frac.chars().take(10) {
        mul /= 10;
        f += c.to_digit(10)? as i32 * mul;
    }
    let v = w.checked_mul(scale)?.checked_add(f)?;
    Some(if neg { -v } else { v })
}

/// A parsed MOL2 file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mol2 {
    /// Molecule name (first MOLECULE line).
    pub name: String,
    /// Declared atom count.
    pub atoms: u32,
    /// Declared bond count.
    pub bonds: u32,
    /// Declared substructure count.
    pub substructures: u32,
    /// `mol_type` (SMALL, BIOPOLYMER, PROTEIN, ...).
    pub mol_type: String,
    /// `charge_type` (NO_CHARGES, USER_CHARGES, ...).
    pub charge_type: String,
    /// Parsed ATOM rows (up to `atoms`).
    pub atom_rows: Vec<AtomRow>,
    /// Section names seen.
    pub sections: Vec<String>,
}

/// Parse a MOL2 file. Returns `None` when the MOLECULE section is
/// missing or not UTF-8.
pub fn parse(d: &[u8]) -> Option<Mol2> {
    let s = core::str::from_utf8(d).ok()?;
    let mut mol: Option<Mol2> = None;
    let mut section = String::new();
    let mut lines = s.lines().peekable();
    while let Some(line) = lines.next() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("@<TRIPOS>") {
            section = rest.trim().to_string();
            if let Some(m) = mol.as_mut() {
                m.sections.push(section.clone());
            }
            continue;
        }
        match section.as_str() {
            "MOLECULE" => {
                if mol.is_none() {
                    let name = t.to_string();
                    let counts = lines.next().unwrap_or("");
                    let c: Vec<u32> = counts
                        .split_ascii_whitespace()
                        .take(5)
                        .map(|n| n.parse().unwrap_or(0))
                        .collect();
                    let mol_type = lines.next().unwrap_or("").trim().to_string();
                    let charge_type = lines.next().unwrap_or("").trim().to_string();
                    mol = Some(Mol2 {
                        name,
                        atoms: c.first().copied().unwrap_or(0),
                        bonds: c.get(1).copied().unwrap_or(0),
                        substructures: c.get(2).copied().unwrap_or(0),
                        mol_type,
                        charge_type,
                        atom_rows: Vec::new(),
                        sections: vec!["MOLECULE".to_string()],
                    });
                }
            }
            "ATOM" => {
                if t.is_empty() {
                    continue;
                }
                let f: Vec<&str> = t.split_ascii_whitespace().collect();
                if f.len() < 6 {
                    continue;
                }
                if let Some(m) = mol.as_mut() {
                    m.atom_rows.push(AtomRow {
                        id: f[0].parse().unwrap_or(0),
                        name: f[1].to_string(),
                        x_milli: fixed(f[2], 1000).unwrap_or(0),
                        y_milli: fixed(f[3], 1000).unwrap_or(0),
                        z_milli: fixed(f[4], 1000).unwrap_or(0),
                        atom_type: f[5].to_string(),
                        subst_id: f.get(6).and_then(|v| v.parse().ok()).unwrap_or(0),
                        subst_name: f.get(7).copied().unwrap_or("").to_string(),
                        charge_milli10k: f.get(8).and_then(|v| fixed(v, 10000)).unwrap_or(0),
                    });
                }
            }
            _ => {}
        }
    }
    mol
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_parse() {
        assert_eq!(fixed("-1.1862", 1000), Some(-1186));
        assert_eq!(fixed("0.3512", 1000), Some(351));
        assert_eq!(fixed("0.0000", 10000), Some(0));
        assert_eq!(fixed("-0.0041", 10000), Some(-41));
        assert_eq!(fixed("x", 1000), None);
        assert_eq!(fixed("", 1000), None);
    }

    #[test]
    fn molecule_and_atoms() {
        let m = "@<TRIPOS>MOLECULE\nwater\n 3 2 1 0 0\nSMALL\nUSER_CHARGES\n\n\
                 @<TRIPOS>ATOM\n 1 O1   0.0000  0.0000  0.1250 O.3    1 WAT -0.8340\n \
                 2 H1   0.9572  0.0000 -0.5000 H      1 WAT  0.4170\n \
                 3 H2  -0.2399  0.9270 -0.2500 H      1 WAT  0.4170\n\
                 @<TRIPOS>BOND\n 1  1  2  1\n 2  1  3  1\n\
                 @<TRIPOS>SUBSTRUCTURE\n 1 WAT 1";
        let p = parse(m.as_bytes()).unwrap();
        assert_eq!(p.name, "water");
        assert_eq!(p.atoms, 3);
        assert_eq!(p.bonds, 2);
        assert_eq!(p.substructures, 1);
        assert_eq!(p.mol_type, "SMALL");
        assert_eq!(p.charge_type, "USER_CHARGES");
        assert_eq!(p.atom_rows.len(), 3);
        let o = &p.atom_rows[0];
        assert_eq!(o.id, 1);
        assert_eq!(o.atom_type, "O.3");
        assert_eq!(o.z_milli, 125);
        assert_eq!(o.charge_milli10k, -8340);
        assert_eq!(p.atom_rows[2].x_milli, -239);
        assert!(p.sections.contains(&"BOND".to_string()));
        assert!(p.sections.contains(&"SUBSTRUCTURE".to_string()));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0xff]).is_none());
        assert!(parse(b"no molecule here").is_none());
        // ATOM before MOLECULE lands nowhere
        let p = parse(
            b"@<TRIPOS>ATOM\n 1 X 0 0 0 C.3\n@<TRIPOS>MOLECULE\nm\n 1 0 0 0 0\nSMALL\nNO_CHARGES\n",
        );
        assert_eq!(p.unwrap().atom_rows.len(), 0);
    }
}
