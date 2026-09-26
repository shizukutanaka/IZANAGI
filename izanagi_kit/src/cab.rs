//! Microsoft Cabinet (.cab) archives (MS-CAB spec).
//!
//! A cabinet is a 36-byte header followed by folder records, file
//! records, and per-folder data blocks. Files are sliced out of their
//! folder's *concatenated uncompressed* stream. Compression modes:
//! `0` stored, `1` MSZIP (each CFDATA block is a `CK` signature
//! followed by a raw DEFLATE stream — decoded by
//! [`crate::inflate::inflate`]).
//!
//! ```
//! // Minimal stored-compression cabinet: 1 folder, 1 file "hi"="abc".
//! let mut cab = Vec::new();
//! let files_off = 36 + 8; // file record follows header+folder
//! let data_off = files_off + 16 + 3; // data follows record+"hi\0"
//! cab.extend_from_slice(b"MSCF");
//! cab.extend_from_slice(&[0; 4]);
//! cab.extend_from_slice(&((data_off + 8 + 3) as u32).to_le_bytes()); // cbCabinet
//! cab.extend_from_slice(&[0; 4]);
//! cab.extend_from_slice(&(files_off as u32).to_le_bytes()); // coffFiles
//! cab.extend_from_slice(&[0; 4]);
//! cab.extend_from_slice(&[3, 1]); // version 3.1
//! cab.extend_from_slice(&1u16.to_le_bytes()); // cFolders
//! cab.extend_from_slice(&1u16.to_le_bytes()); // cFiles
//! cab.extend_from_slice(&[0; 6]); // flags, setID, iCabinet
//! cab.extend_from_slice(&(data_off as u32).to_le_bytes()); // CFFOLDER
//! cab.extend_from_slice(&1u16.to_le_bytes()); // cCFData
//! cab.extend_from_slice(&0u16.to_le_bytes()); // stored
//! cab.extend_from_slice(&3u32.to_le_bytes()); // CFFILE: cbFile
//! cab.extend_from_slice(&0u32.to_le_bytes()); // uoffFolderStart
//! cab.extend_from_slice(&0u16.to_le_bytes()); // iFolder
//! cab.extend_from_slice(&[0; 6]); // date, time, attribs
//! cab.extend_from_slice(b"hi\0");
//! cab.extend_from_slice(&[0; 4]); // CFDATA: csum
//! cab.extend_from_slice(&3u16.to_le_bytes()); // cbData
//! cab.extend_from_slice(&3u16.to_le_bytes()); // cbUncomp
//! cab.extend_from_slice(b"abc");
//! let c = izanagi_kit::cab::parse(&cab).unwrap();
//! assert_eq!(c.files[0].name, b"hi");
//! assert_eq!(izanagi_kit::cab::extract(&cab, &c, 0).unwrap(), b"abc");
//! ```

fn le16(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | ((*d.get(at + 1)? as u16) << 8))
}
fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        (*d.get(at)? as u32)
            | ((*d.get(at + 1)? as u32) << 8)
            | ((*d.get(at + 2)? as u32) << 16)
            | ((*d.get(at + 3)? as u32) << 24),
    )
}

/// Compression scheme of a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comp {
    /// Plain bytes.
    Stored,
    /// MSZIP — raw DEFLATE per block after a 2-byte `CK` signature.
    Mszip,
    /// Anything else (Quantum, LZX) is kept but not decodable here.
    Other(u16),
}

/// A CFFOLDER record.
#[derive(Debug, Clone, Copy)]
pub struct Folder {
    /// Absolute offset of the first CFDATA block.
    pub data_at: usize,
    /// Number of CFDATA blocks.
    pub blocks: u16,
    /// Compression.
    pub comp: Comp,
}

/// A CFFILE record.
#[derive(Debug, Clone)]
pub struct File {
    /// Uncompressed size in bytes.
    pub size: u32,
    /// Byte offset into the folder's uncompressed stream.
    pub offset: u32,
    /// Folder index, or `0xFFFE`/`0xFFFF` for files continued from /
    /// spilling into adjacent cabinets — see [`folder_idx`].
    pub folder: u16,
    /// Packed DOS date.
    pub date: u16,
    /// Packed DOS time.
    pub time: u16,
    /// Attribute bits.
    pub attribs: u16,
    /// File name bytes (NUL stripped).
    pub name: Vec<u8>,
}

/// A parsed cabinet.
#[derive(Debug)]
pub struct Cab {
    /// Declared cabinet size (`cbCabinet`).
    pub size: u32,
    /// Header flags (bit 1 prev link, bit 2 next link, bit 4 reserved
    /// areas present).
    pub flags: u16,
    /// Set identifier shared by a cabinet chain.
    pub set_id: u16,
    /// This cabinet's index within the set.
    pub index: u16,
    /// Folder records.
    pub folders: Vec<Folder>,
    /// File records.
    pub files: Vec<File>,
}

/// Maps a file's `folder` field to a `Cab.folders` index; `None` when
/// the file is continued across cabinets or out of range.
pub fn folder_idx(c: &Cab, f: &File) -> Option<usize> {
    match f.folder {
        0xFFFE | 0xFFFF => None,
        i => {
            let i = i as usize;
            if i < c.folders.len() {
                Some(i)
            } else {
                None
            }
        }
    }
}

/// Parses the CFHEADER + CFFOLDER + CFFILE tables.
/// `None` on bad magic, truncation, or out-of-range offsets.
pub fn parse(d: &[u8]) -> Option<Cab> {
    if d.len() < 36 || &d[0..4] != b"MSCF" {
        return None;
    }
    let size = le32(d, 8)?;
    let files_at = le32(d, 16)? as usize;
    let folder_count = le16(d, 26)? as usize;
    let file_count = le16(d, 28)? as usize;
    let flags = le16(d, 30)?;
    let set_id = le16(d, 32)?;
    let index = le16(d, 34)?;

    // Reserved areas (flag bit 4): cbCFHeader(u16) cbCFFolder(u8)
    // cbCFData(u8) at bytes 36..40, then cbCFHeader extra bytes, then
    // cbCFFolder extra bytes after each folder record.
    let (hdr_extra, folder_extra) = if flags & 4 != 0 {
        (le16(d, 36)? as usize, *d.get(38)? as usize)
    } else {
        (0, 0)
    };
    let mut fat = if flags & 4 != 0 { 40 + hdr_extra } else { 36 };

    let mut frecs = Vec::with_capacity(folder_count.min(1 << 15));
    for _ in 0..folder_count {
        let data_at = le32(d, fat)? as usize;
        let blocks = le16(d, fat + 4)?;
        let ty = le16(d, fat + 6)?;
        let comp = match ty & 0xF {
            0 => Comp::Stored,
            1 => Comp::Mszip,
            t => Comp::Other(t),
        };
        frecs.push(Folder {
            data_at,
            blocks,
            comp,
        });
        fat += 8 + folder_extra;
    }

    if files_at < fat || files_at > d.len() {
        return None;
    }
    let mut fvec = Vec::with_capacity(file_count.min(1 << 20));
    let mut at = files_at;
    for _ in 0..file_count {
        let size = le32(d, at)?;
        let offset = le32(d, at + 4)?;
        let folder = le16(d, at + 8)?;
        let date = le16(d, at + 10)?;
        let time = le16(d, at + 12)?;
        let attribs = le16(d, at + 14)?;
        let name_start = at + 16;
        let mut end = name_start;
        while end < d.len() && d[end] != 0 {
            end += 1;
        }
        if end >= d.len() {
            return None;
        }
        fvec.push(File {
            size,
            offset,
            folder,
            date,
            time,
            attribs,
            name: d[name_start..end].to_vec(),
        });
        at = end + 1;
    }
    Some(Cab {
        size,
        flags,
        set_id,
        index,
        folders: frecs,
        files: fvec,
    })
}

/// Reassembles one folder's uncompressed stream: walks its CFDATA
/// blocks, decoding `Stored` (raw) or `Mszip` (`CK` + DEFLATE) payloads.
/// `None` on an undecodable compressor or a truncated block.
pub fn folder_bytes(d: &[u8], c: &Cab, fi: usize) -> Option<Vec<u8>> {
    let f = c.folders.get(fi)?;
    let mut out = Vec::new();
    let mut at = f.data_at;
    for _ in 0..f.blocks {
        let cb_data = le16(d, at + 4)? as usize;
        let payload_start = at + 8;
        let payload_end = payload_start.checked_add(cb_data)?;
        if payload_end > d.len() {
            return None;
        }
        let payload = &d[payload_start..payload_end];
        match f.comp {
            Comp::Stored => out.extend_from_slice(payload),
            Comp::Mszip => {
                if payload.len() < 2 || &payload[0..2] != b"CK" {
                    return None;
                }
                let dec = crate::inflate::inflate(&payload[2..])?;
                out.extend_from_slice(&dec);
            }
            Comp::Other(_) => return None,
        }
        at = payload_end;
    }
    Some(out)
}

/// Returns the `i`-th file's contents; `None` when non-extractable
/// (cross-cabinet link, undecodable compressor, out-of-range slice).
pub fn extract(d: &[u8], c: &Cab, i: usize) -> Option<Vec<u8>> {
    let f = c.files.get(i)?;
    let fi = folder_idx(c, f)?;
    let folder_data = folder_bytes(d, c, fi)?;
    let s = f.offset as usize;
    let e = s.checked_add(f.size as usize)?;
    if e > folder_data.len() {
        return None;
    }
    Some(folder_data[s..e].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn le32v(v: u32) -> [u8; 4] {
        [v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8]
    }
    fn le16v(v: u16) -> [u8; 2] {
        [v as u8, (v >> 8) as u8]
    }

    /// Minimal cabinet: 1 folder (MSZIP), 1 file "hi" = "hello".
    fn cab_fixture() -> Vec<u8> {
        // Layout (MS-CAB):
        //   0..36  CFHEADER (cbCabinet@8, coffFiles@16, cFolders@26,
        //          cFiles@28, flags@30, setID@32, iCabinet@34)
        //   36..44 CFFOLDER {data_area=63, blocks=1, comp=MSZIP}
        //   44..60 CFFILE  {size=5, off=0, folder=0, date, time, attribs}
        //   60..63 "hi\0"
        //   63..   CFDATA {csum, cbData, cbUncomp} + "CK" + deflate
        let raw = crate::deflate::deflate(b"hello");
        let mut cab = Vec::new();
        let total = 63 + 8 + 2 + raw.len();
        cab.extend_from_slice(b"MSCF");
        cab.extend_from_slice(&[0; 4]); // reserved1
        cab.extend_from_slice(&le32v(total as u32)); // cbCabinet
        cab.extend_from_slice(&[0; 4]); // reserved2
        cab.extend_from_slice(&le32v(44)); // coffFiles
        cab.extend_from_slice(&[0; 4]); // reserved3
        cab.extend_from_slice(&[3, 1]); // verMinor/verMajor
        cab.extend_from_slice(&le16v(1)); // cFolders
        cab.extend_from_slice(&le16v(1)); // cFiles
        cab.extend_from_slice(&[0; 2]); // flags
        cab.extend_from_slice(&[0; 2]); // setID
        cab.extend_from_slice(&[0; 2]); // iCabinet
        assert_eq!(cab.len(), 36);
        // CFFOLDER
        cab.extend_from_slice(&le32v(63)); // data_area
        cab.extend_from_slice(&le16v(1)); // blocks
        cab.extend_from_slice(&le16v(1)); // MSZIP
                                          // CFFILE
        cab.extend_from_slice(&le32v(5)); // cbFile
        cab.extend_from_slice(&le32v(0)); // uoffFolderStart
        cab.extend_from_slice(&le16v(0)); // iFolder
        cab.extend_from_slice(&[0; 6]); // date, time, attribs
        cab.extend_from_slice(b"hi\0");
        assert_eq!(cab.len(), 63);
        // CFDATA
        cab.extend_from_slice(&[0; 4]); // csum
        cab.extend_from_slice(&le16v((raw.len() + 2) as u16)); // cbData
        cab.extend_from_slice(&le16v(5)); // cbUncomp
        cab.extend_from_slice(b"CK");
        cab.extend_from_slice(&raw);
        cab
    }

    #[test]
    fn parses_header_and_tables() {
        let c = parse(&cab_fixture()).unwrap();
        assert_eq!(c.folders.len(), 1);
        assert_eq!(c.files.len(), 1);
        assert_eq!(c.files[0].name, b"hi");
        assert_eq!(c.folders[0].comp, Comp::Mszip);
        assert_eq!(c.set_id, 0);
    }

    #[test]
    fn extracts_mszip_file() {
        let cab = cab_fixture();
        let c = parse(&cab).unwrap();
        assert_eq!(extract(&cab, &c, 0).unwrap(), b"hello");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"XXXX").is_none());
        let mut bad = cab_fixture();
        bad[26] = 9; // claims 9 folders, only 1 present
        assert!(parse(&bad).is_none());
        let mut bad2 = cab_fixture();
        let c = parse(&bad2).unwrap();
        let data_at = c.folders[0].data_at;
        bad2[data_at + 8] = b'X'; // break "CK" signature
        assert!(folder_bytes(&bad2, &c, 0).is_none());
        assert!(extract(&cab_fixture(), &c, 9).is_none());
    }

    #[test]
    fn folder_idx_handles_link_sentinels() {
        let c = parse(&cab_fixture()).unwrap();
        let f = File {
            size: 0,
            offset: 0,
            folder: 0xFFFF,
            date: 0,
            time: 0,
            attribs: 0,
            name: vec![],
        };
        assert_eq!(folder_idx(&c, &f), None);
        let f2 = File { folder: 0, ..f };
        assert_eq!(folder_idx(&c, &f2), Some(0));
        let f3 = File { folder: 7, ..f2 };
        assert_eq!(folder_idx(&c, &f3), None);
    }
}
