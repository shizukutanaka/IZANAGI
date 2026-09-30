//! Gmsh `.msh` mesh file: text (or tagged-binary) sections delimited
//! by `$Name` / `$EndName` lines. The mandatory `$MeshFormat` line
//! carries `version filetype datasize` — e.g. `4.1 0 8` or
//! `2.2 1 8` (filetype 1 = binary).
//!
//! ```
//! let d = b"$MeshFormat\n4.1 0 8\n$EndMeshFormat\n$Nodes\n$EndNodes\n";
//! let m = izanagi_kit::msh::parse(d).unwrap();
//! assert_eq!(m.version, "4.1");
//! assert!(!m.binary);
//! assert!(m.sections.contains(&"Nodes".to_string()));
//! ```

use std::string::String;
use std::vec::Vec;

/// A parsed Gmsh mesh file.
#[derive(Clone, Debug)]
pub struct Msh {
    /// Format version string, e.g. `"2.2"`, `"4.1"`.
    pub version: String,
    /// `filetype` == 1 → binary sections.
    pub binary: bool,
    /// Declared `datasize` (8 for real Gmsh).
    pub datasize: u32,
    /// Section names seen (`Nodes`, `Elements`, … — without `$`).
    pub sections: Vec<String>,
}

/// Parse a `.msh` file; `None` without a well-formed `$MeshFormat`
/// block at the start.
pub fn parse(d: &[u8]) -> Option<Msh> {
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines().map(str::trim).filter(|l| !l.is_empty());
    if lines.next()? != "$MeshFormat" {
        return None;
    }
    let fmt = lines.next()?;
    let mut it = fmt.split_whitespace();
    let version = it.next()?.to_string();
    let binary = it.next()? == "1";
    let datasize: u32 = it.next()?.parse().ok()?;
    if !(version.starts_with('2') || version.starts_with('4')) {
        return None;
    }
    if lines.next()? != "$EndMeshFormat" {
        return None;
    }
    let mut sections = Vec::new();
    for raw in lines {
        if let Some(name) = raw.strip_prefix('$') {
            if !name.starts_with("End") && !name.is_empty() {
                sections.push(name.to_string());
            }
        }
    }
    Some(Msh {
        version,
        binary,
        datasize,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v41_ascii() {
        let d = b"$MeshFormat\n4.1 0 8\n$EndMeshFormat\n$Entities\n1 0 0 0\n$EndEntities\n$Nodes\n$EndNodes\n";
        let m = parse(d).unwrap();
        assert_eq!(m.version, "4.1");
        assert!(!m.binary);
        assert_eq!(m.datasize, 8);
        assert_eq!(m.sections, vec!["Entities", "Nodes"]);
    }

    #[test]
    fn v22_binary_flag() {
        let d = b"$MeshFormat\n2.2 1 8\n$EndMeshFormat\n";
        let m = parse(d).unwrap();
        assert!(m.binary);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"$Nodes\n").is_none());
        assert!(parse(b"$MeshFormat\n4.1 0\n$EndMeshFormat\n").is_none()); // missing field
        assert!(parse(b"$MeshFormat\n4.1 0 8\n$Nodes\n").is_none()); // no EndMeshFormat
        assert!(parse(b"$MeshFormat\n9.9 0 8\n$EndMeshFormat\n").is_none()); // bogus ver
    }
}
