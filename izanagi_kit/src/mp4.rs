//! ISO Base Media File Format (ISO/IEC 14496-12) box walker — the
//! container grammar shared by MP4, MOV, and the fragmented-media family.
//! Every box is `size:u32 type:u32`; `size == 1` means a real 64-bit
//! `largesize` follows, `size == 0` means "to end of file", and the
//! `uuid` type carries an extra 16-byte usertype. [`boxes`] walks one
//! level; [`children`] descends into the container boxes (`moov`, `trak`,
//! `mdia`, `minf`, `stbl`…, with `meta`'s extra version/flags word and
//! `stsd`'s entry-count header handled). Typed readers: [`ftyp`] brand
//! sniffing, [`mvhd`] movie header, [`tkhd`] track header, [`mdhd`]
//! media header, [`hdlr`] handler type, [`stts`] sample timing.
//!
//! ```
//! use izanagi_kit::mp4::{boxes, ftyp};
//! let mut d = vec![0, 0, 0, 24];
//! d.extend_from_slice(b"ftyp");
//! d.extend_from_slice(b"isom");
//! d.extend_from_slice(&512u32.to_be_bytes());
//! d.extend_from_slice(b"isomiso2");
//! let f = ftyp(&d).unwrap();
//! assert_eq!(&f.major, b"isom");
//! assert_eq!(f.compatible, vec![*b"isom", *b"iso2"]);
//! assert_eq!(boxes(&d, 0, d.len()).unwrap().len(), 1);
//! ```

use std::vec::Vec;

/// One box's location inside the buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bx {
    /// Four-byte type (`ftyp`, `moov`, `uuid`…).
    pub kind: [u8; 4],
    /// Byte offset of the box header.
    pub at: usize,
    /// Total box size in bytes including header.
    pub size: u64,
    /// Header size (8, 16 for `largesize`, +16 again for `uuid`).
    pub head: usize,
}

fn u32be(d: &[u8], i: usize) -> Option<u32> {
    let b = d.get(i..i + 4)?;
    Some(u32::from(b[0]) << 24 | u32::from(b[1]) << 16 | u32::from(b[2]) << 8 | u32::from(b[3]))
}

fn u64be(d: &[u8], i: usize) -> Option<u64> {
    let b = d.get(i..i + 8)?;
    let mut v = 0u64;
    for &c in b {
        v = v << 8 | u64::from(c);
    }
    Some(v)
}

fn u16be(d: &[u8], i: usize) -> Option<u16> {
    let b = d.get(i..i + 2)?;
    Some(u16::from(b[0]) << 8 | u16::from(b[1]))
}

/// Walk the boxes in `d[at..end]`. `None` when a header is truncated, a
/// declared size overruns the range, or a `largesize` is below its
/// header — any box that doesn't fit cleanly. A trailing box sized to
/// `end` (`free`/`skip`, or `size == 0` extending to the range end) is
/// the legal terminator.
pub fn boxes(d: &[u8], at: usize, end: usize) -> Option<Vec<Bx>> {
    let mut v = Vec::new();
    let mut i = at;
    while i < end {
        let size32 = u32be(d, i)? as u64;
        let kind: [u8; 4] = d.get(i + 4..i + 8)?.try_into().ok()?;
        let mut head = 8usize;
        let mut size = size32;
        if size32 == 1 {
            size = u64be(d, i + 8)?;
            head = 16;
        } else if size32 == 0 {
            size = (end - i) as u64;
        }
        if &kind == b"uuid" {
            head += 16;
        }
        if size < head as u64 || i as u64 + size > end as u64 {
            return None;
        }
        v.push(Bx {
            kind,
            at: i,
            size,
            head,
        });
        i = i.checked_add(size as usize)?;
    }
    Some(v)
}

/// True when `kind` is a container box whose payload is child boxes.
/// (`meta` and `stsd` are containers too but carry a 4-byte header
/// before their children — [`children`] handles them.)
pub fn is_container(kind: [u8; 4]) -> bool {
    matches!(
        &kind,
        b"moov"
            | b"trak"
            | b"mdia"
            | b"minf"
            | b"stbl"
            | b"dinf"
            | b"edts"
            | b"udta"
            | b"moof"
            | b"traf"
            | b"mvex"
            | b"meta"
            | b"stsd"
    )
}

/// Child boxes of a container `parent`, honoring `meta`'s 4-byte
/// version/flags word and `stsd`'s `version/flags + entry_count` 8-byte
/// header. `None` for non-container kinds or malformed children.
pub fn children(d: &[u8], parent: &Bx) -> Option<Vec<Bx>> {
    let mut at = parent.at + parent.head;
    if &parent.kind == b"meta" {
        at += 4; // FullBox version/flags
    } else if &parent.kind == b"stsd" {
        at += 8; // FullBox version/flags + entry_count
    }
    let end = parent.at.checked_add(parent.size as usize)?;
    if at > end {
        return None;
    }
    boxes(d, at, end)
}

/// `ftyp` major brand, minor version, and compatible brands.
#[derive(Clone, Debug, PartialEq)]
pub struct Ftyp {
    /// Four-byte major brand (`isom`, `mp42`, `qt  `, `M4V `…).
    pub major: [u8; 4],
    /// Minor version (usually an encoding year or 0).
    pub minor: u32,
    /// Compatible brands list.
    pub compatible: Vec<[u8; 4]>,
}

/// Parse the `ftyp` box — must be the first box of the file.
pub fn ftyp(d: &[u8]) -> Option<Ftyp> {
    let bs = boxes(d, 0, d.len())?;
    let f = bs.first()?;
    if &f.kind != b"ftyp" {
        return None;
    }
    let mut i = f.at + f.head;
    let major: [u8; 4] = d.get(i..i + 4)?.try_into().ok()?;
    let minor = u32be(d, i + 4)?;
    i += 8;
    let mut compatible = Vec::new();
    let end = f.at + f.size as usize;
    while i + 4 <= end {
        compatible.push(d.get(i..i + 4)?.try_into().ok()?);
        i += 4;
    }
    if i != end {
        return None;
    }
    Some(Ftyp {
        major,
        minor,
        compatible,
    })
}

/// `mvhd` contents, version-normalized.
#[derive(Clone, Debug, PartialEq)]
pub struct Mvhd {
    /// Timescale (units per second) for durations.
    pub timescale: u32,
    /// Movie duration in timescale units.
    pub duration: u64,
}

/// Read an `mvhd` box body.
pub fn mvhd(d: &[u8], bx: &Bx) -> Option<Mvhd> {
    if &bx.kind != b"mvhd" {
        return None;
    }
    let i = bx.at + bx.head;
    match *d.get(i)? {
        0 => Some(Mvhd {
            timescale: u32be(d, i + 12)?,
            duration: u32be(d, i + 16)? as u64,
        }),
        1 => Some(Mvhd {
            timescale: u32be(d, i + 20)?,
            duration: u64be(d, i + 24)?,
        }),
        _ => None,
    }
}

/// `tkhd` contents, version-normalized.
#[derive(Clone, Debug, PartialEq)]
pub struct Tkhd {
    /// Track identifier.
    pub track_id: u32,
    /// Track duration in movie timescale units.
    pub duration: u64,
    /// Presentation size (`width`, `height`) as 16.16 fixed point.
    pub size: (u32, u32),
}

/// Read a `tkhd` box body.
pub fn tkhd(d: &[u8], bx: &Bx) -> Option<Tkhd> {
    if &bx.kind != b"tkhd" {
        return None;
    }
    let i = bx.at + bx.head;
    let v = *d.get(i)?;
    let (track_id, duration, wh) = match v {
        0 => (u32be(d, i + 12)?, u32be(d, i + 20)? as u64, i + 76),
        1 => (u32be(d, i + 20)?, u64be(d, i + 28)?, i + 88),
        _ => return None,
    };
    Some(Tkhd {
        track_id,
        duration,
        size: (u32be(d, wh)?, u32be(d, wh + 4)?),
    })
}

/// `mdhd` contents, version-normalized.
#[derive(Clone, Debug, PartialEq)]
pub struct Mdhd {
    /// Media timescale (units per second).
    pub timescale: u32,
    /// Media duration in timescale units.
    pub duration: u64,
    /// ISO-639-2/T packed 5-bit language code (three letters).
    pub language: u16,
}

/// Read an `mdhd` box body.
pub fn mdhd(d: &[u8], bx: &Bx) -> Option<Mdhd> {
    if &bx.kind != b"mdhd" {
        return None;
    }
    let i = bx.at + bx.head;
    match *d.get(i)? {
        0 => Some(Mdhd {
            timescale: u32be(d, i + 12)?,
            duration: u32be(d, i + 16)? as u64,
            language: u16be(d, i + 20)?,
        }),
        1 => Some(Mdhd {
            timescale: u32be(d, i + 20)?,
            duration: u64be(d, i + 24)?,
            language: u16be(d, i + 32)?,
        }),
        _ => None,
    }
}

/// Read a `hdlr` box's 4-byte handler type (`vide`, `soun`, `text`,
/// `meta`…).
pub fn hdlr(d: &[u8], bx: &Bx) -> Option<[u8; 4]> {
    if &bx.kind != b"hdlr" {
        return None;
    }
    d.get(bx.at + bx.head + 8..bx.at + bx.head + 12)?
        .try_into()
        .ok()
}

/// One run from an `stts` time-to-sample table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stts {
    /// How many consecutive samples share this delta.
    pub count: u32,
    /// Sample duration in media timescale units.
    pub delta: u32,
}

/// Read an `stts` box body: version/flags, entry count, then
/// `(count, delta)` pairs.
pub fn stts(d: &[u8], bx: &Bx) -> Option<Vec<Stts>> {
    if &bx.kind != b"stts" {
        return None;
    }
    let i = bx.at + bx.head;
    let n = u32be(d, i + 4)? as usize;
    let mut v = Vec::with_capacity(n);
    for k in 0..n {
        let at = i + 8 + k * 8;
        v.push(Stts {
            count: u32be(d, at)?,
            delta: u32be(d, at + 4)?,
        });
    }
    Some(v)
}

/// First box of `kind` in a walked list.
pub fn find<'a>(bs: &'a [Bx], kind: &[u8; 4]) -> Option<&'a Bx> {
    bs.iter().find(|b| &b.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut d = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        d.extend_from_slice(kind);
        d.extend_from_slice(body);
        d
    }

    #[test]
    fn walk_and_ftyp() {
        let mut d = bx(b"ftyp", b"isom\x00\x00\x02\x00isomiso2");
        d.extend_from_slice(&bx(b"free", &[0; 8]));
        let bs = boxes(&d, 0, d.len()).unwrap();
        assert_eq!(bs.len(), 2);
        assert_eq!(&bs[0].kind, b"ftyp");
        assert_eq!(&bs[1].kind, b"free");
        let f = ftyp(&d).unwrap();
        assert_eq!(&f.major, b"isom");
        assert_eq!(f.minor, 512);
        assert_eq!(f.compatible, vec![*b"isom", *b"iso2"]);
    }

    #[test]
    fn largesize_and_tail() {
        // largesize box
        let mut d = 1u32.to_be_bytes().to_vec();
        d.extend_from_slice(b"mdat");
        d.extend_from_slice(&20u64.to_be_bytes());
        d.extend_from_slice(&[0xAB; 4]);
        // size==0 box runs to EOF
        let mut t = d.clone();
        t.extend_from_slice(&0u32.to_be_bytes());
        t.extend_from_slice(b"free");
        t.extend_from_slice(&[9; 4]);
        let bs = boxes(&t, 0, t.len()).unwrap();
        assert_eq!(bs[0].size, 20);
        assert_eq!(bs[0].head, 16);
        assert_eq!(bs[1].size, 12);
    }

    #[test]
    fn nested_moov() {
        // moov { mvhd, trak { tkhd } }
        let mut mvhd_body = vec![0u8]; // version 0
        mvhd_body.extend_from_slice(&[0; 11]);
        mvhd_body.extend_from_slice(&1000u32.to_be_bytes()); // timescale
        mvhd_body.extend_from_slice(&2500u32.to_be_bytes()); // duration
        let mut tkhd_body = vec![0u8];
        tkhd_body.extend_from_slice(&[0; 11]);
        tkhd_body.extend_from_slice(&7u32.to_be_bytes()); // track_id
        tkhd_body.extend_from_slice(&[0; 4]);
        tkhd_body.extend_from_slice(&2500u32.to_be_bytes()); // duration
        tkhd_body.extend_from_slice(&[0; 52]);
        tkhd_body.extend_from_slice(&1280u32.to_be_bytes()); // width <<16
        tkhd_body.extend_from_slice(&720u32.to_be_bytes());
        let mut moov = bx(b"mvhd", &mvhd_body);
        let trak_body = bx(b"tkhd", &tkhd_body);
        let mut trak = bx(b"trak", &trak_body);
        moov.append(&mut trak);
        let mut file = bx(b"ftyp", b"isom\0\0\0\0");
        let moov_box = bx(b"moov", &moov);
        file.extend_from_slice(&moov_box);
        let top = boxes(&file, 0, file.len()).unwrap();
        let moov_bx = find(&top, b"moov").unwrap();
        let kids = children(&file, moov_bx).unwrap();
        assert_eq!(kids.len(), 2);
        let m = mvhd(&file, find(&kids, b"mvhd").unwrap()).unwrap();
        assert_eq!((m.timescale, m.duration), (1000, 2500));
        let trak_kids = children(&file, find(&kids, b"trak").unwrap()).unwrap();
        let t = tkhd(&file, &trak_kids[0]).unwrap();
        assert_eq!(t.track_id, 7);
        assert_eq!(t.size, (1280, 720));
    }

    #[test]
    fn media_boxes() {
        // mdia { mdhd(v0), hdlr }, stbl { stts } — plus is_container sweep
        let mut mdhd_body = vec![0u8];
        mdhd_body.extend_from_slice(&[0; 11]);
        mdhd_body.extend_from_slice(&48000u32.to_be_bytes()); // timescale
        mdhd_body.extend_from_slice(&96000u32.to_be_bytes()); // duration
        mdhd_body.extend_from_slice(&0x55C4u16.to_be_bytes()); // "und"
        let mut hdlr_body = vec![0u8]; // version
        hdlr_body.extend_from_slice(&[0; 3]); // flags
        hdlr_body.extend_from_slice(&[0; 4]); // pre_defined
        hdlr_body.extend_from_slice(b"soun"); // handler_type
        let mut stts_body = vec![0u8];
        stts_body.extend_from_slice(&[0; 3]);
        stts_body.extend_from_slice(&1u32.to_be_bytes()); // entry_count
        stts_body.extend_from_slice(&1024u32.to_be_bytes()); // count
        stts_body.extend_from_slice(&512u32.to_be_bytes()); // delta
        let mut mdia = bx(b"mdhd", &mdhd_body);
        mdia.extend_from_slice(&bx(b"hdlr", &hdlr_body));
        let file = bx(b"mdia", &mdia);
        let top = boxes(&file, 0, file.len()).unwrap();
        let mdia_bx = &top[0];
        assert!(is_container(mdia_bx.kind));
        assert!(!is_container(*b"mdat"));
        let kids = children(&file, mdia_bx).unwrap();
        let m = mdhd(&file, find(&kids, b"mdhd").unwrap()).unwrap();
        assert_eq!(
            (m.timescale, m.duration, m.language),
            (48000, 96000, 0x55C4)
        );
        assert_eq!(
            &hdlr(&file, find(&kids, b"hdlr").unwrap()).unwrap(),
            b"soun"
        );
        // stsd skips its 8-byte header before children
        let stsd_body = {
            let mut v = vec![0u8, 0, 0, 0]; // version/flags
            v.extend_from_slice(&1u32.to_be_bytes()); // entry_count
            v.extend_from_slice(&bx(b"mp4a", &[0; 8]));
            v
        };
        let file2 = bx(b"stsd", &stsd_body);
        let stsd_kids = children(&file2, &boxes(&file2, 0, file2.len()).unwrap()[0]).unwrap();
        assert_eq!(&stsd_kids[0].kind, b"mp4a");
        // stts table
        let file3 = bx(b"stts", &stts_body);
        let s = stts(&file3, &boxes(&file3, 0, file3.len()).unwrap()[0]).unwrap();
        assert_eq!(
            s,
            vec![Stts {
                count: 1024,
                delta: 512
            }]
        );
    }

    #[test]
    fn bad_inputs() {
        assert!(boxes(&[], 0, 0).unwrap().is_empty());
        let short = bx(b"free", &[0; 4]);
        assert!(boxes(&short[..9], 0, 9).is_none()); // box overruns
        let mut d = bx(b"free", &[0; 4]);
        d.truncate(6);
        assert!(boxes(&d, 0, d.len()).is_none()); // truncated header
        assert!(ftyp(&bx(b"free", &[0; 4])).is_none()); // ftyp not first
        assert!(ftyp(&[]).is_none());
    }
}
