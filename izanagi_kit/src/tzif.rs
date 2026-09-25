//! TZif — the `/usr/share/zoneinfo` binary format (RFC 8536): a
//! `TZif` + version header, a 32-bit v1 data block, and for v2/v3 a
//! second header plus the 64-bit block and a POSIX-TZ footer.
//! [`parse`] prefers the 64-bit block when present; [`Tzif::offset_at`]
//! resolves a Unix timestamp to a UTC offset in seconds, honouring
//! the footer rule that instants past the last transition repeat the
//! last-described pattern (approximated here by the last type —
//! full POSIX-TZ evaluation is out of scope).
//!
//! ```
//! use izanagi_kit::tzif::parse;
//! // minimal v1 file: one "UTC" type at offset 0
//! let mut d = b"TZif\x00".to_vec();
//! d.extend_from_slice(&[0; 15]); // reserved
//! for n in [0u32, 0, 0, 0, 1, 4] { // gmt/std/leap/time/typecnt/charcnt
//!     d.extend_from_slice(&[(n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8]);
//! }
//! d.extend_from_slice(&[0, 0, 0, 0]); // utoff = 0
//! d.extend_from_slice(&[0, 1]);       // dst = 0, abbr index 1
//! d.extend_from_slice(b"\0UTC\0");    // abbreviation chars
//! let z = parse(&d).unwrap();
//! assert_eq!(z.offset_at(1_700_000_000).unwrap(), 0);
//! assert_eq!(z.types[0].abbr, "UTC");
//! ```

use std::vec::Vec;

/// One local-time type record.
#[derive(Clone, Debug, PartialEq)]
pub struct TzType {
    /// Offset from UTC in seconds.
    pub offset: i32,
    /// True if this type is daylight-saving.
    pub is_dst: bool,
    /// Abbreviation (e.g. "EST", "JST"); empty if the index was
    /// out of range.
    pub abbr: String,
}

/// A parsed zone file.
#[derive(Clone, Debug, PartialEq)]
pub struct Tzif {
    /// Transition instants (Unix seconds) → type index.
    pub transitions: Vec<(i64, u32)>,
    /// The type table.
    pub types: Vec<TzType>,
    /// Optional POSIX-TZ footer string (v2/v3).
    pub footer: Option<String>,
}

fn be32(d: &[u8], i: usize) -> Option<i64> {
    let mut v = 0i64;
    for k in 0..4 {
        v = (v << 8) | *d.get(i + k)? as i64;
    }
    Some(v)
}

fn be64(d: &[u8], i: usize) -> Option<i64> {
    let mut v = 0i64;
    for k in 0..8 {
        v = (v << 8) | *d.get(i + k)? as i64;
    }
    Some(v)
}

fn sig(v: i64, wide: bool) -> i64 {
    // sign-extend a 32- or 64-bit big-endian field
    if !wide && v >= 0x8000_0000 {
        v - 0x1_0000_0000
    } else {
        v
    }
}

struct Hdr {
    timecnt: usize,
    typecnt: usize,
    charcnt: usize,
    leapcnt: usize,
    ttisstdcnt: usize,
    ttisgmtcnt: usize,
}

fn hdr(d: &[u8]) -> Option<(u8, Hdr, usize)> {
    if d.get(..4)? != b"TZif" {
        return None;
    }
    let ver = *d.get(4)?;
    let g = |n: usize| -> Option<usize> { Some(be32(d, 20 + n * 4)? as usize) };
    Some((
        ver,
        Hdr {
            ttisgmtcnt: g(0)?,
            ttisstdcnt: g(1)?,
            leapcnt: g(2)?,
            timecnt: g(3)?,
            typecnt: g(4)?,
            charcnt: g(5)?,
        },
        44,
    ))
}

fn block(d: &[u8], mut i: usize, h: &Hdr, wide: bool, t: &mut Tzif) -> Option<usize> {
    let tsz = if wide { 8 } else { 4 };
    let mut tr = Vec::with_capacity(h.timecnt);
    for _ in 0..h.timecnt {
        let v = if wide { be64(d, i)? } else { be32(d, i)? };
        tr.push(sig(v, wide));
        i += tsz;
    }
    for (k, &ts) in tr.iter().enumerate() {
        let idx = *d.get(i + k)? as u32;
        if (idx as usize) >= h.typecnt {
            return None;
        }
        t.transitions.push((ts, idx));
    }
    i += h.timecnt;
    let mut types = Vec::with_capacity(h.typecnt);
    let mut abidx = Vec::with_capacity(h.typecnt);
    for _ in 0..h.typecnt {
        let off = sig(be32(d, i)?, false) as i32;
        let dst = *d.get(i + 4)? != 0;
        abidx.push(*d.get(i + 5)? as usize);
        types.push((off, dst));
        i += 6;
    }
    let chars = d.get(i..i.checked_add(h.charcnt)?)?;
    i += h.charcnt;
    for (k, &(off, dst)) in types.iter().enumerate() {
        let a = abidx[k];
        let abbr = if a < chars.len() {
            let end = chars[a..]
                .iter()
                .position(|&c| c == 0)
                .map(|p| a + p)
                .unwrap_or(chars.len());
            String::from_utf8_lossy(&chars[a..end]).into_owned()
        } else {
            String::new()
        };
        t.types.push(TzType {
            offset: off,
            is_dst: dst,
            abbr,
        });
    }
    // skip leapsecond records
    i += h.leapcnt.checked_mul(if wide { 12 } else { 8 })?;
    i += h.ttisstdcnt;
    i += h.ttisgmtcnt;
    Some(i)
}

/// Parse a TZif file; `None` on a bad magic, truncated header, or
/// out-of-range type index. v2/v3 files use their 64-bit block and
/// expose the footer.
pub fn parse(d: &[u8]) -> Option<Tzif> {
    let (ver, h1, mut i) = hdr(d)?;
    let mut t = Tzif {
        transitions: Vec::new(),
        types: Vec::new(),
        footer: None,
    };
    i = block(d, i, &h1, false, &mut t)?;
    if ver >= 2 {
        // second header + 64-bit block
        let (_v2, h2, mut j) = hdr(d.get(i..)?)?;
        j += i;
        let mut t2 = Tzif {
            transitions: Vec::new(),
            types: Vec::new(),
            footer: None,
        };
        j = block(d, j, &h2, true, &mut t2)?;
        t = t2;
        // footer: \n POSIX-TZ \n
        if let Some(rest) = d.get(j..) {
            if rest.first() == Some(&b'\n') {
                let inner = &rest[1..];
                if let Some(e) = inner.iter().position(|&c| c == b'\n') {
                    let f = String::from_utf8_lossy(&inner[..e]).into_owned();
                    if !f.is_empty() {
                        t.footer = Some(f);
                    }
                }
            }
        }
    }
    Some(t)
}

impl Tzif {
    /// UTC offset (seconds) in effect at Unix time `t`. With no
    /// transitions, the first type applies. Before the first
    /// transition the first *non-DST* type is used when present
    /// (the RFC's standard-time rule), else the first type.
    /// `None` only when the file carries no type records.
    pub fn offset_at(&self, t: i64) -> Option<i32> {
        let idx = match self.transitions.partition_point(|&(ts, _)| ts <= t) {
            0 => self
                .types
                .iter()
                .position(|ty| !ty.is_dst)
                .unwrap_or_else(|| {
                    *self.transitions.first().map(|(_, i)| i).unwrap_or(&0) as usize
                }),
            n => self.transitions[n - 1].1 as usize,
        };
        self.types.get(idx).map(|ty| ty.offset)
    }

    /// The [`TzType`] in effect at `t` (same lookup as
    /// [`Tzif::offset_at`] but returning the record).
    pub fn type_at(&self, t: i64) -> Option<&TzType> {
        let idx = match self.transitions.partition_point(|&(ts, _)| ts <= t) {
            0 => self
                .types
                .iter()
                .position(|ty| !ty.is_dst)
                .unwrap_or_else(|| {
                    *self.transitions.first().map(|(_, i)| i).unwrap_or(&0) as usize
                }),
            n => self.transitions[n - 1].1 as usize,
        };
        self.types.get(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w32(v: i32) -> [u8; 4] {
        let u = v as u32;
        [(u >> 24) as u8, (u >> 16) as u8, (u >> 8) as u8, u as u8]
    }

    /// Fixture builder: types carry an explicit abbr index.
    fn v1x(types: &[(i32, bool, u8, &str)], trs: &[(i64, u32)], chars: &[u8]) -> Vec<u8> {
        let mut d = b"TZif\x00".to_vec();
        d.extend_from_slice(&[0; 15]);
        for n in [
            0u32,
            0,
            0,
            trs.len() as u32,
            types.len() as u32,
            chars.len() as u32,
        ] {
            d.extend_from_slice(&w32(n as i32));
        }
        for &(t, _) in trs {
            d.extend_from_slice(&w32(t as i32));
        }
        for &(_, i) in trs {
            d.push(i as u8);
        }
        for &(o, dst, ai, _) in types {
            d.extend_from_slice(&w32(o));
            d.push(dst as u8);
            d.push(ai);
        }
        d.extend_from_slice(chars);
        d
    }

    #[test]
    fn single_type() {
        let d = v1x(&[(32400, false, 1, "JST")], &[], b"\0JST\0");
        let z = parse(&d).unwrap();
        assert_eq!(z.types[0].offset, 32400);
        assert_eq!(z.types[0].abbr, "JST");
        assert_eq!(z.offset_at(0).unwrap(), 32400);
        assert_eq!(z.offset_at(999_999_999).unwrap(), 32400);
    }

    #[test]
    fn transitions_pick_type() {
        // EST(-18000) ↔ EDT(-14400): switch at t=1000 and t=2000
        let d = v1x(
            &[(-18000, false, 1, "EST"), (-14400, true, 5, "EDT")],
            &[(1000, 1), (2000, 0)],
            b"\0EST\0EDT\0",
        );
        let z = parse(&d).unwrap();
        assert_eq!(z.offset_at(0).unwrap(), -18000); // pre-first: std type
        assert_eq!(z.offset_at(999).unwrap(), -18000);
        assert_eq!(z.offset_at(1000).unwrap(), -14400);
        assert_eq!(z.offset_at(2000).unwrap(), -18000);
        assert_eq!(z.type_at(1500).unwrap().abbr, "EDT");
    }

    #[test]
    fn v2_uses_64bit_block() {
        // build a v2 file: header(ver 2) + v1(empty) block + header2 + 64-bit block
        let inner = v1x(&[(3600, false, 1, "X")], &[(5_000_000_000, 0)], b"\0X\0");
        // retag: version byte 2, then the same body written twice with 64-bit times
        let mut d = b"TZif\x02".to_vec();
        d.extend_from_slice(&[0; 15]);
        // v1 header: zero counts everywhere
        for _ in 0..6 {
            d.extend_from_slice(&w32(0));
        }
        // second header: time=1, type=1, char=3
        d.extend_from_slice(b"TZif\x02");
        d.extend_from_slice(&[0; 15]);
        for n in [0u32, 0, 0, 1, 1, 3] {
            d.extend_from_slice(&w32(n as i32));
        }
        // 64-bit transition at 5e9
        let t64 = 5_000_000_000i64 as u64;
        for k in (0..8).rev() {
            d.push((t64 >> (k * 8)) as u8);
        }
        d.push(0); // type idx
        d.extend_from_slice(&w32(3600));
        d.push(0);
        d.push(1); // abbr idx
        d.extend_from_slice(b"\0X\0");
        d.extend_from_slice(b"\nEST5EDT,M3.2.0/2,M11.1.0/2\n");
        let z = parse(&d).unwrap();
        assert_eq!(z.offset_at(4_999_999_999).unwrap(), 3600); // pre-transition: std type
        assert_eq!(z.offset_at(6_000_000_000).unwrap(), 3600);
        assert!(z.footer.is_some());
        let _ = inner;
    }

    #[test]
    fn bad_inputs() {
        assert!(parse(b"").is_none());
        assert!(parse(b"ABCD").is_none());
        // type index out of range
        let mut d = v1x(&[(0, false, 0, "")], &[(1, 7)], b"\0");
        assert!(parse(&d).is_none());
        // truncated
        d.truncate(30);
        assert!(parse(&d).is_none());
    }

    #[test]
    fn determinism() {
        let d = v1x(&[(0, false, 1, "UTC")], &[], b"\0UTC\0");
        assert_eq!(parse(&d), parse(&d));
    }
}
