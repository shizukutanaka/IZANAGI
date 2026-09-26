//! VMware VMDK extent descriptor (the text `.vmdk` file).
//!
//! The descriptor is a small text file: `#`-comments, `key=value`
//! pairs (whitespace around `=` tolerated), and *extent lines* of the
//! form `RW <sectors> <TYPE> "<file>" [offset]`. Sparse VMDKs point
//! at a binary extent whose own header starts with magic `KDMV`.
//!
//! ```
//! use izanagi_kit::vmdk::{parse, Access, Kind};
//!
//! let s = "# Disk DescriptorFile\nversion=1\nCID=00ffffff\n\
//!          createType=\"monolithicFlat\"\n\
//!          RW 63 FLAT \"disk-flat.vmdk\" 0\n";
//! let v = parse(s).unwrap();
//! assert_eq!(v.version, Some(1));
//! assert_eq!(v.cid, Some(0x00ff_ffff));
//! assert_eq!(v.create_type.as_deref(), Some("monolithicFlat"));
//! let e = &v.extents[0];
//! assert_eq!((e.access, e.kind), (Access::ReadWrite, Kind::Flat));
//! assert_eq!(e.sectors, 63);
//! assert_eq!(e.file.as_deref(), Some("disk-flat.vmdk"));
//! assert_eq!(e.offset, Some(0));
//! ```

use std::string::String;
use std::vec::Vec;

/// Sparse binary-extent magic (`0x564D444B`, "KDMV" little-endian).
pub const SPARSE_MAGIC: u32 = 0x564D_444B;

/// Extent access mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Access {
    /// `RW` — read-write extent.
    ReadWrite,
    /// `RDONLY` — read-only extent.
    ReadOnly,
    /// `NOACCESS` — present but not addressable.
    NoAccess,
}

/// Extent storage type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// `SPARSE` — grain-table sparse file.
    Sparse,
    /// `FLAT` — preallocated raw image.
    Flat,
    /// `ZERO` — synthesizes zeroes (no backing file).
    Zero,
    /// `SESPARSE` — space-efficient sparse (vSphere).
    SeSparse,
    /// `VMFS*` — raw device mappings (`VMFS`, `VMFSSPARSE`, `VMFSRDM`, `VMFSRAW`).
    Vmfs,
    /// Any other type keyword.
    Other,
}

/// One extent line from the descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct Extent {
    /// Access mode.
    pub access: Access,
    /// Size in 512-byte sectors.
    pub sectors: u64,
    /// Extent type.
    pub kind: Kind,
    /// Type keyword as written (e.g. `"FLAT"`).
    pub kind_raw: String,
    /// Backing file name, if the extent has one.
    pub file: Option<String>,
    /// Byte offset into the backing file, if present.
    pub offset: Option<u64>,
}

/// A parsed VMDK descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct Vmdk {
    /// `version=N`, if present.
    pub version: Option<u32>,
    /// `CID=hex` content id, if present.
    pub cid: Option<u32>,
    /// `parentCID=hex`, if present.
    pub parent_cid: Option<u32>,
    /// `createType="..."`, if present.
    pub create_type: Option<String>,
    /// `ddb.` database entries (`key` without the `ddb.` prefix).
    pub ddb: Vec<(String, String)>,
    /// Extent lines in file order.
    pub extents: Vec<Extent>,
}

fn unquote(s: &str) -> Option<String> {
    let t = s.trim();
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        Some(String::from(t.get(1..t.len() - 1)?))
    } else {
        None
    }
}

fn extent(line: &str) -> Option<Extent> {
    let (head, rest) = line.split_once(' ')?;
    let access = match head {
        "RW" => Access::ReadWrite,
        "RDONLY" => Access::ReadOnly,
        "NOACCESS" => Access::NoAccess,
        _ => return None,
    };
    // <sectors> <TYPE> "<file>" [offset] — filename may contain spaces
    let mut it = rest.trim_start().splitn(2, ' ');
    let sectors: u64 = it.next()?.trim().parse().ok()?;
    let rest2 = it.next()?.trim_start();
    let (kind_raw, tail) = rest2.split_once(' ').unwrap_or((rest2, ""));
    let kind = match kind_raw {
        "SPARSE" => Kind::Sparse,
        "FLAT" => Kind::Flat,
        "ZERO" => Kind::Zero,
        "SESPARSE" => Kind::SeSparse,
        k if k.starts_with("VMFS") => Kind::Vmfs,
        _ => Kind::Other,
    };
    let tail = tail.trim_start();
    let (file, tail2) = if tail.is_empty() {
        (None, "")
    } else if tail.starts_with('"') {
        let end = tail.get(1..)?.find('"')? + 1;
        (
            Some(String::from(tail.get(1..end)?)),
            tail.get(end + 1..).unwrap_or("").trim(),
        )
    } else {
        let (w, r) = tail.split_once(' ').unwrap_or((tail, ""));
        (Some(String::from(w)), r.trim())
    };
    let offset = if tail2.is_empty() {
        None
    } else {
        Some(tail2.parse().ok()?)
    };
    Some(Extent {
        access,
        sectors,
        kind,
        kind_raw: String::from(kind_raw),
        file,
        offset,
    })
}

/// Parse a VMDK text descriptor. Returns `None` when there is no
/// recognizable `key=value` or extent content at all (i.e. the text
/// is not a descriptor).
pub fn parse(s: &str) -> Option<Vmdk> {
    let mut v = Vmdk {
        version: None,
        cid: None,
        parent_cid: None,
        create_type: None,
        ddb: Vec::new(),
        extents: Vec::new(),
    };
    let mut found = false;
    for raw in s.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(e) = extent(line) {
            v.extents.push(e);
            found = true;
            continue;
        }
        if let Some((k, val)) = line.split_once('=') {
            let key = k.trim();
            let value = val.trim();
            found = true;
            match key {
                "version" => v.version = value.parse().ok(),
                "CID" => v.cid = u32::from_str_radix(value, 16).ok(),
                "parentCID" => v.parent_cid = u32::from_str_radix(value, 16).ok(),
                "createType" => {
                    v.create_type = unquote(value).or_else(|| Some(String::from(value)))
                }
                _ if key.starts_with("ddb.") => {
                    v.ddb.push((
                        String::from(&key[4..]),
                        unquote(value).unwrap_or_else(|| String::from(value)),
                    ));
                }
                _ => {}
            }
        }
    }
    if found {
        Some(v)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESC: &str = "# Disk DescriptorFile\n\
        version=1\n\
        CID=00ffffff\n\
        parentCID=ffffffff\n\
        createType=\"twoGbMaxExtentSparse\"\n\
        # Extent description\n\
        RW 4192256 SPARSE \"disk-s001.vmdk\"\n\
        RW 4192256 SPARSE \"disk-s002.vmdk\"\n\
        RW 2109440 FLAT \"disk-flat.vmdk\" 0\n\
        # The Disk Data Base\n\
        #DDB\n\
        ddb.virtualHWVersion = \"4\"\n";

    #[test]
    fn parses_fields() {
        let v = parse(DESC).unwrap();
        assert_eq!(v.version, Some(1));
        assert_eq!(v.cid, Some(0x00ff_ffff));
        assert_eq!(v.parent_cid, Some(0xffff_ffff));
        assert_eq!(v.create_type.as_deref(), Some("twoGbMaxExtentSparse"));
        assert_eq!(v.ddb.len(), 1);
        assert_eq!(v.ddb[0].0, "virtualHWVersion");
        assert_eq!(v.ddb[0].1, "4");
    }

    #[test]
    fn parses_extents() {
        let v = parse(DESC).unwrap();
        assert_eq!(v.extents.len(), 3);
        assert_eq!(v.extents[0].access, Access::ReadWrite);
        assert_eq!(v.extents[0].kind, Kind::Sparse);
        assert_eq!(v.extents[0].sectors, 4_192_256);
        assert_eq!(v.extents[0].file.as_deref(), Some("disk-s001.vmdk"));
        assert_eq!(v.extents[0].offset, None);
        assert_eq!(v.extents[2].kind, Kind::Flat);
        assert_eq!(v.extents[2].offset, Some(0));
    }

    #[test]
    fn access_and_kind_variants() {
        let v = parse("RDONLY 10 VMFS \"raw.vmdk\"\nNOACCESS 1 ZERO\n").unwrap();
        assert_eq!(v.extents[0].access, Access::ReadOnly);
        assert_eq!(v.extents[0].kind, Kind::Vmfs);
        assert_eq!(v.extents[1].access, Access::NoAccess);
        assert_eq!(v.extents[1].kind, Kind::Zero);
        let v2 = parse("RW 1 SESPARSE \"a.vmdk\"\nRW 1 XSPARSE \"b.vmdk\"\n").unwrap();
        assert_eq!(v2.extents[0].kind, Kind::SeSparse);
        assert_eq!(v2.extents[1].kind, Kind::Other);
    }

    #[test]
    fn rejects_and_tolerates() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("# only comments\n# more\n"), None);
        // garbage lines are skipped, real keys still count
        let v = parse("nonsense line here\nversion=2\n").unwrap();
        assert_eq!(v.version, Some(2));
        // unquoted createType falls back to raw value
        let v2 = parse("createType=streamOptimized\n").unwrap();
        assert_eq!(v2.create_type.as_deref(), Some("streamOptimized"));
    }

    #[test]
    fn sparse_magic_constant() {
        assert_eq!(SPARSE_MAGIC, 0x564D_444B);
    }
}
