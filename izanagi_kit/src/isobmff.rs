//! ISO Base Media File Format (ISO 14496-12) — the box/atom tree of
//! `.mp4`/`.mov`/`.heic`/`.m4a` files: every box is `size u32BE +
//! type[4]` (`size==1` → 64-bit largesize, `size==0` → to EOF,
//! `type=="uuid"` → 16-byte extended tag), and container boxes hold
//! child boxes.
//!
//! ```
//! // ftyp + moov(trak) minimal tree.
//! let mut d = Vec::new();
//! d.extend_from_slice(&(24u32).to_be_bytes());
//! d.extend_from_slice(b"ftyp");
//! d.extend_from_slice(b"isom");
//! d.extend_from_slice(&[0; 4]);
//! d.extend_from_slice(b"isomiso2");
//! // moov { trak { } }
//! let inner = {
//!     let mut t = Vec::new();
//!     t.extend_from_slice(&(8u32).to_be_bytes());
//!     t.extend_from_slice(b"trak");
//!     t
//! };
//! d.extend_from_slice(&((8 + inner.len()) as u32).to_be_bytes());
//! d.extend_from_slice(b"moov");
//! d.extend_from_slice(&inner);
//! let f = izanagi_kit::isobmff::parse(&d).unwrap();
//! assert_eq!(f.len(), 2);
//! let kids = izanagi_kit::isobmff::children(&d, &f[1]).unwrap();
//! assert_eq!(kids[0].ty, *b"trak");
//! ```

/// A box reference — `ty` is the 4-byte tag, `at`/`size` cover the
/// whole box (header included); `data_at`/`data_len` the payload.
#[derive(Debug, Clone)]
pub struct Bx {
    /// FourCC tag (`b"ftyp"`, `b"moov"`, …).
    pub ty: [u8; 4],
    /// Box start offset (at the size field).
    pub at: usize,
    /// Total box bytes (header + data).
    pub size: usize,
    /// Payload start (after size+type/largesize/uuid fields).
    pub data_at: usize,
    /// Payload length.
    pub data_len: usize,
    /// `uuid` extended type bytes when `ty == *b"uuid"`.
    pub uuid: Option<[u8; 16]>,
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        ((*d.get(at)? as u32) << 24)
            | ((*d.get(at + 1)? as u32) << 16)
            | ((*d.get(at + 2)? as u32) << 8)
            | (*d.get(at + 3)? as u32),
    )
}
fn be64(d: &[u8], at: usize) -> Option<u64> {
    let hi = be32(d, at)? as u64;
    let lo = be32(d, at + 4)? as u64;
    Some((hi << 32) | lo)
}

/// Container types whose payload is itself a box list.
/// `meta` carries a 4-byte version/flags field first.
pub fn is_container(ty: &[u8; 4]) -> bool {
    matches!(
        ty,
        b"moov"
            | b"trak"
            | b"mdia"
            | b"minf"
            | b"dinf"
            | b"stbl"
            | b"edts"
            | b"udta"
            | b"moof"
            | b"traf"
            | b"mfra"
            | b"skip"
            | b"strk"
            | b"sinf"
            | b"schi"
            | b"tref"
    ) || ty == b"meta"
}

/// Reads one box at `at`; returns `None` on truncation.
fn read_box(d: &[u8], at: usize, end: usize) -> Option<Bx> {
    if at + 8 > end || at + 8 > d.len() {
        return None;
    }
    let sz = be32(d, at)?;
    let mut ty = [0u8; 4];
    ty.copy_from_slice(&d[at + 4..at + 8]);
    let mut header = 8usize;
    let mut size = sz as u64;
    if sz == 1 {
        size = be64(d, at + 8)?;
        header = 16;
    } else if sz == 0 {
        size = (end - at) as u64; // extends to parent/EOF
    }
    let mut uuid = None;
    if &ty == b"uuid" {
        let mut u = [0u8; 16];
        u.copy_from_slice(d.get(at + header..at + header + 16)?);
        uuid = Some(u);
        header += 16;
    }
    let size = usize::try_from(size).ok()?;
    if size < header || at + size > end {
        return None;
    }
    Some(Bx {
        ty,
        at,
        size,
        data_at: at + header,
        data_len: size - header,
        uuid,
    })
}

/// The top-level box list. `None` on truncation.
pub fn parse(d: &[u8]) -> Option<Vec<Bx>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < d.len() {
        let b = read_box(d, at, d.len())?;
        at += b.size;
        out.push(b);
    }
    Some(out)
}

/// Children of a container box. `None` when `b` is not a container
/// or the child region is truncated. `meta` skips its 4-byte
/// version/flags word.
pub fn children(d: &[u8], b: &Bx) -> Option<Vec<Bx>> {
    if !is_container(&b.ty) {
        return None;
    }
    let mut at = b.data_at;
    if &b.ty == b"meta" {
        at += 4;
    }
    let end = b.data_at + b.data_len;
    let mut out = Vec::new();
    while at < end {
        let c = read_box(d, at, end)?;
        at += c.size;
        out.push(c);
    }
    Some(out)
}

/// Payload bytes of a box.
pub fn data<'a>(d: &'a [u8], b: &Bx) -> Option<&'a [u8]> {
    d.get(b.data_at..b.data_at + b.data_len)
}

/// First top-level box with the tag.
pub fn find<'a>(bs: &'a [Bx], ty: &[u8; 4]) -> Option<&'a Bx> {
    bs.iter().find(|b| &b.ty == ty)
}

/// Descends a type path — `find_path(&d, &top, &[b"moov", b"trak",
/// b"mdia"])`.
pub fn find_path(d: &[u8], bs: &[Bx], path: &[&[u8; 4]]) -> Option<Bx> {
    let mut cur: Option<Bx> = None;
    let mut list: Vec<Bx> = bs.to_vec();
    for (i, &ty) in path.iter().enumerate() {
        let b = list.iter().find(|x| &x.ty == ty)?.clone();
        if i + 1 == path.len() {
            cur = Some(b);
            break;
        }
        list = children(d, &b)?;
    }
    cur
}

/// Major brand of a `ftyp` box (first 4 bytes of its payload).
pub fn major_brand(d: &[u8], ftyp: &Bx) -> Option<[u8; 4]> {
    if &ftyp.ty != b"ftyp" {
        return None;
    }
    let p = data(d, ftyp)?;
    let mut t = [0u8; 4];
    t.copy_from_slice(p.get(0..4)?);
    Some(t)
}

/// Compatible brands (every 4 bytes after brand+version).
pub fn compatible_brands(d: &[u8], ftyp: &Bx) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    if let Some(p) = data(d, ftyp) {
        for i in (8..p.len()).step_by(4) {
            if let Ok(t) = <[u8; 4]>::try_from(&p[i..i + 4]) {
                out.push(t);
            }
        }
    }
    out
}

/// `mvhd` movie header: `(version, timescale, duration)`.
pub fn mvhd(d: &[u8], b: &Bx) -> Option<(u8, u32, u64)> {
    let p = data(d, b)?;
    let v = *p.first()?;
    let (ts, dur) = match v {
        0 => (be32(p, 12)?, be32(p, 20)? as u64),
        1 => (be32(p, 20)?, be64(p, 28)?),
        _ => return None,
    };
    Some((v, ts, dur))
}

/// `tkhd` track header: `(version, track_id, duration)`.
pub fn tkhd(d: &[u8], b: &Bx) -> Option<(u8, u32, u64)> {
    let p = data(d, b)?;
    let v = *p.first()?;
    match v {
        0 => Some((v, be32(p, 12)?, be32(p, 20)? as u64)),
        1 => Some((v, be32(p, 20)?, be64(p, 28)?)),
        _ => None,
    }
}

/// `stts` decoding-time deltas: count then `n` (count, delta) pairs.
pub fn stts(d: &[u8], b: &Bx) -> Option<Vec<(u32, u32)>> {
    let p = data(d, b)?;
    let n = be32(p, 4)? as usize;
    let mut out = Vec::with_capacity(n.min(1 << 20));
    for i in 0..n {
        out.push((be32(p, 8 + i * 8)?, be32(p, 12 + i * 8)?));
    }
    Some(out)
}

/// `stsz` sample sizes: `(default_size, Vec<per-sample sizes>)`.
pub fn stsz(d: &[u8], b: &Bx) -> Option<(u32, Vec<u32>)> {
    let p = data(d, b)?;
    let def = be32(p, 4)?;
    let n = be32(p, 8)? as usize;
    if def != 0 {
        return Some((def, Vec::new()));
    }
    let mut out = Vec::with_capacity(n.min(1 << 22));
    for i in 0..n {
        out.push(be32(p, 12 + i * 4)?);
    }
    Some((def, out))
}

/// `stco`/`co64` chunk offsets.
pub fn chunk_offsets(d: &[u8], b: &Bx) -> Option<Vec<u64>> {
    let p = data(d, b)?;
    let n = be32(p, 4)? as usize;
    let mut out = Vec::with_capacity(n.min(1 << 22));
    let wide = &b.ty == b"co64";
    for i in 0..n {
        out.push(if wide {
            be64(p, 8 + i * 8)?
        } else {
            be32(p, 8 + i * 4)? as u64
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // ftyp + moov{ mvhd + trak{ tkhd + mdia{ minf{ stbl{ stts } } } } }
        let mut mvhd_p = vec![0u8; 24]; // v0 header: ver+flags+ct+mt+ts+dur
        mvhd_p[12..16].copy_from_slice(&1000u32.to_be_bytes()); // timescale
        mvhd_p[20..24].copy_from_slice(&5000u32.to_be_bytes()); // duration
        let mut mvhd = Vec::new();
        mvhd.extend_from_slice(&((8 + 24) as u32).to_be_bytes());
        mvhd.extend_from_slice(b"mvhd");
        mvhd.extend_from_slice(&mvhd_p);

        let mut tkhd_p = vec![0u8; 24];
        tkhd_p[12..16].copy_from_slice(&7u32.to_be_bytes()); // track id
        tkhd_p[20..24].copy_from_slice(&123u32.to_be_bytes()); // duration
        let mut tkhd = Vec::new();
        tkhd.extend_from_slice(&((8 + 24) as u32).to_be_bytes());
        tkhd.extend_from_slice(b"tkhd");
        tkhd.extend_from_slice(&tkhd_p);

        let mut stts_p = vec![0u8; 8];
        stts_p[4..8].copy_from_slice(&1u32.to_be_bytes());
        stts_p.extend_from_slice(&100u32.to_be_bytes());
        stts_p.extend_from_slice(&40u32.to_be_bytes());
        let mut stts = Vec::new();
        stts.extend_from_slice(&((8 + 16) as u32).to_be_bytes());
        stts.extend_from_slice(b"stts");
        stts.extend_from_slice(&stts_p);

        let mut trak = Vec::new();
        trak.extend_from_slice(b"");
        let trak_payload = tkhd;
        trak.extend_from_slice(&((8 + trak_payload.len()) as u32).to_be_bytes());
        trak.extend_from_slice(b"trak");
        trak.extend_from_slice(&trak_payload);

        let mut stsz_p = vec![0u8; 12];
        stsz_p[4..8].copy_from_slice(&2u32.to_be_bytes()); // default size 2
        stsz_p[8..12].copy_from_slice(&3u32.to_be_bytes()); // count 3
        let mut stsz = Vec::new();
        stsz.extend_from_slice(&((8 + 12) as u32).to_be_bytes());
        stsz.extend_from_slice(b"stsz");
        stsz.extend_from_slice(&stsz_p);

        let mut stco_p = vec![0u8; 8];
        stco_p[4..8].copy_from_slice(&1u32.to_be_bytes());
        stco_p.extend_from_slice(&1024u32.to_be_bytes());
        let mut stco = Vec::new();
        stco.extend_from_slice(&((8 + 12) as u32).to_be_bytes());
        stco.extend_from_slice(b"stco");
        stco.extend_from_slice(&stco_p);

        let mut moov = Vec::new();
        let moov_payload = [mvhd, trak, stts, stsz, stco].concat();
        moov.extend_from_slice(&((8 + moov_payload.len()) as u32).to_be_bytes());
        moov.extend_from_slice(b"moov");
        moov.extend_from_slice(&moov_payload);

        let mut d = Vec::new();
        d.extend_from_slice(&28u32.to_be_bytes());
        d.extend_from_slice(b"ftyp");
        d.extend_from_slice(b"isom");
        d.extend_from_slice(&[0; 4]);
        d.extend_from_slice(b"isomiso2avc1");
        d.extend_from_slice(&moov);
        d
    }

    #[test]
    fn parses_boxes_and_paths() {
        let d = fixture();
        let top = parse(&d).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(&top[0].ty, b"ftyp");
        let ftyp = find(&top, b"ftyp").unwrap();
        assert_eq!(major_brand(&d, ftyp), Some(*b"isom"));
        assert!(compatible_brands(&d, ftyp).contains(b"isom"));
        let mv = find_path(&d, &top, &[b"moov", b"trak", b"tkhd"]).unwrap();
        assert_eq!(tkhd(&d, &mv).unwrap(), (0, 7, 123));
    }

    #[test]
    fn mvhd_and_stts() {
        let d = fixture();
        let top = parse(&d).unwrap();
        let mv = find_path(&d, &top, &[b"moov", b"mvhd"]).unwrap();
        assert_eq!(mvhd(&d, &mv).unwrap(), (0, 1000, 5000));
        let st = find_path(&d, &top, &[b"moov", b"stts"]).unwrap();
        assert_eq!(stts(&d, &st).unwrap(), vec![(100, 40)]);
        assert!(is_container(b"moov"));
        assert!(!is_container(b"ftyp"));
        let sz = find_path(&d, &top, &[b"moov", b"stsz"]).unwrap();
        assert_eq!(stsz(&d, &sz).unwrap(), (2, Vec::new()));
        let co = find_path(&d, &top, &[b"moov", b"stco"]).unwrap();
        assert_eq!(chunk_offsets(&d, &co).unwrap(), vec![1024]);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).unwrap().is_empty());
        assert!(parse(&[0, 0, 0, 4]).is_none()); // too small for a header
        let mut bad = fixture();
        bad[0] = 0xFF; // ftyp size huge
        assert!(parse(&bad).is_none());
        let mut bad2 = fixture();
        bad2.truncate(30); // mid-box
        assert!(parse(&bad2).is_none());
    }
}
