//! PDB (Protein Data Bank) fixed-column record reader.
//!
//! Each record type occupies fixed columns of an 80-column card.
//! `ATOM`/`HETATM` columns: 7-11 serial, 13-16 atom name,
//! 18-20 residue name, 22 chain, 23-26 residue sequence, 31-54
//! x/y/z (8.3 fixed point), 77-78 element. `CRYST1` carries the
//! unit cell (a/b/c as 9.3, angles 7.2) and space group.
//! Coordinates are returned as integer milliunits — the kit never
//! interprets floats.
//!
//! ```
//! use izanagi_kit::pdb::parse;
//!
//! let line = "ATOM      1  CA  ALA A   1      12.345  -7.890   1.000  1.00 20.00           C ";
//! let p = parse(line.as_bytes()).unwrap();
//! assert_eq!(p.atoms.len(), 1);
//! assert_eq!(p.atoms[0].x_milli, 12345);
//! assert_eq!(p.atoms[0].y_milli, -7890);
//! assert_eq!(p.atoms[0].residue, "ALA");
//! ```

/// One `ATOM`/`HETATM` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Atom {
    /// True for `HETATM` (non-standard residue).
    pub het: bool,
    /// Atom serial number.
    pub serial: u32,
    /// Atom name (e.g. "CA").
    pub name: String,
    /// Residue name (e.g. "ALA").
    pub residue: String,
    /// Chain identifier (may be a space).
    pub chain: char,
    /// Residue sequence number.
    pub res_seq: i32,
    /// x coordinate × 1000.
    pub x_milli: i32,
    /// y coordinate × 1000.
    pub y_milli: i32,
    /// z coordinate × 1000.
    pub z_milli: i32,
    /// Element symbol (cols 77-78).
    pub element: String,
}

/// Unit cell from `CRYST1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// a, b, c lengths in Ångströms × 1000.
    pub abc_milli: (i32, i32, i32),
    /// α, β, γ angles in degrees × 100.
    pub angles_centi: (i32, i32, i32),
    /// Space group symbol (cols 56-66).
    pub space_group: String,
}

/// Parse `" -12.345"` style fixed-point into milliunits
/// (`frac_digits` = decimals after the point). Returns `None` on
/// garbage or when `frac_digits` mismatches.
fn fixed(s: &str, frac_digits: usize) -> Option<i32> {
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
    if frac.len() != frac_digits
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
        || whole.is_empty()
    {
        return None;
    }
    let w: i32 = whole.parse().unwrap_or(0);
    let mut scale = 1i32;
    for _ in 0..frac_digits {
        scale *= 10;
    }
    let f: i32 = if frac.is_empty() {
        0
    } else {
        frac.parse().unwrap_or(0)
    };
    let v = w.checked_mul(scale)?.checked_add(f)?;
    Some(if neg { -v } else { v })
}

fn field(d: &[u8], a: usize, b: usize) -> String {
    let end = b.min(d.len());
    if a >= end {
        return String::new();
    }
    core::str::from_utf8(&d[a..end])
        .unwrap_or("")
        .trim()
        .to_string()
}

/// A parsed PDB file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pdb {
    /// `ATOM`/`HETATM` records.
    pub atoms: Vec<Atom>,
    /// `CRYST1` unit cell, when present.
    pub cell: Option<Cell>,
    /// Other record-type names seen (HEADER, SEQRES, END, ...).
    pub others: Vec<String>,
}

/// Parse line by line. Returns `None` when input is not UTF-8.
pub fn parse(d: &[u8]) -> Option<Pdb> {
    let s = core::str::from_utf8(d).ok()?;
    let mut atoms = Vec::new();
    let mut cell = None;
    let mut others = Vec::new();
    for line in s.lines() {
        let b = line.as_bytes();
        let rec = field(b, 0, 6);
        match rec.as_str() {
            "ATOM" | "HETATM" => {
                atoms.push(Atom {
                    het: rec == "HETATM",
                    serial: field(b, 6, 11).parse().unwrap_or(0),
                    name: field(b, 12, 16),
                    residue: field(b, 17, 20),
                    chain: b.get(21).copied().map(char::from).unwrap_or(' '),
                    res_seq: field(b, 22, 26).parse().unwrap_or(0),
                    x_milli: fixed(&field(b, 30, 38), 3).unwrap_or(0),
                    y_milli: fixed(&field(b, 38, 46), 3).unwrap_or(0),
                    z_milli: fixed(&field(b, 46, 54), 3).unwrap_or(0),
                    element: field(b, 76, 78),
                });
            }
            "CRYST1" => {
                cell = Some(Cell {
                    abc_milli: (
                        fixed(&field(b, 6, 15), 3).unwrap_or(0),
                        fixed(&field(b, 15, 24), 3).unwrap_or(0),
                        fixed(&field(b, 24, 33), 3).unwrap_or(0),
                    ),
                    angles_centi: (
                        fixed(&field(b, 33, 40), 2).unwrap_or(0),
                        fixed(&field(b, 40, 47), 2).unwrap_or(0),
                        fixed(&field(b, 47, 54), 2).unwrap_or(0),
                    ),
                    space_group: field(b, 55, 66),
                });
            }
            _ => {
                if !rec.is_empty() {
                    others.push(rec);
                }
            }
        }
    }
    Some(Pdb {
        atoms,
        cell,
        others,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point() {
        assert_eq!(fixed("  12.345", 3), Some(12345));
        assert_eq!(fixed("  -7.890", 3), Some(-7890));
        assert_eq!(fixed("   1.000", 3), Some(1000));
        assert_eq!(fixed(" 90.00", 2), Some(9000));
        assert_eq!(fixed("bad", 3), None);
        assert_eq!(fixed("  ", 3), None);
        assert_eq!(fixed("1.23", 3), None); // digit-count mismatch
    }

    #[test]
    fn atoms_and_cell() {
        let txt = "HEADER    TEST\n\
                   ATOM      1  N   ALA A   1      11.104  -6.134   0.000  1.00 20.00           N \n\
                   ATOM      2  CA  ALA A   1      12.560   1.000  -0.500  1.00 20.00           C \n\
                   HETATM  100  O   HOH B   5      -1.000  -2.000  -3.250  1.00 50.00           O \n\
                   CRYST1   52.000   58.600   61.900  90.00  90.00 120.00 P 63\n\
                   END\n";
        let p = parse(txt.as_bytes()).unwrap();
        assert_eq!(p.atoms.len(), 3);
        let a0 = &p.atoms[0];
        assert!(!a0.het);
        assert_eq!(a0.serial, 1);
        assert_eq!(a0.name, "N");
        assert_eq!(a0.residue, "ALA");
        assert_eq!(a0.chain, 'A');
        assert_eq!(a0.res_seq, 1);
        assert_eq!(a0.x_milli, 11104);
        assert_eq!(a0.z_milli, 0);
        assert_eq!(a0.element, "N");
        let a2 = &p.atoms[2];
        assert!(a2.het);
        assert_eq!(a2.residue, "HOH");
        assert_eq!(a2.y_milli, -2000);
        let c = p.cell.unwrap();
        assert_eq!(c.abc_milli.2, 61900);
        assert_eq!(c.angles_centi.2, 12000);
        assert_eq!(c.space_group, "P 63");
        assert!(p.others.contains(&"HEADER".to_string()));
        assert!(p.others.contains(&"END".to_string()));
    }

    #[test]
    fn rejects_non_utf8() {
        assert!(parse(&[0xff, 0xfe]).is_none());
        assert!(parse(b"   \n").unwrap().atoms.is_empty());
    }
}
