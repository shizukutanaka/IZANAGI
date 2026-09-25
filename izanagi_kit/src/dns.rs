//! DNS wire-format messages (RFC 1035) — the protocol-layer sibling
//! of [`ip`](crate::ip).
//!
//! [`build_query`] emits a standard recursive `A`/`AAAA`/... query;
//! [`parse`] decodes a full message with label-compression pointers
//! (`0xC0`) followed — with a hard bound so pointer loops reject instead
//! of spinning. Non-ASCII labels are decoded lossily into `String`.
//! `RData` understands `A`, `AAAA`, `CNAME`/`NS`/`PTR` (as names),
//! `MX`, `SOA`, `TXT`; everything else is `Unknown` raw bytes.
//!
//! ```
//! use izanagi_kit::dns::{build_query, parse};
//! let q = build_query(0x1234, "www.example.com", 1).unwrap();
//! let m = parse(&q).unwrap();
//! assert_eq!(m.id, 0x1234);
//! assert_eq!(m.questions[0].name, "www.example.com");
//! ```

/// A parsed DNS message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// Transaction id.
    pub id: u16,
    /// QR bit — true for responses.
    pub is_response: bool,
    /// 4-bit opcode (0 = standard query).
    pub opcode: u8,
    /// Authoritative-answer bit.
    pub aa: bool,
    /// Truncation bit.
    pub tc: bool,
    /// Recursion-desired bit.
    pub rd: bool,
    /// Recursion-available bit.
    pub ra: bool,
    /// 4-bit response code (0 = NOERROR, 3 = NXDOMAIN).
    pub rcode: u8,
    /// Question section.
    pub questions: Vec<Question>,
    /// Answer section.
    pub answers: Vec<RR>,
    /// Authority section.
    pub authorities: Vec<RR>,
    /// Additional section.
    pub additionals: Vec<RR>,
}

/// `QNAME + QTYPE + QCLASS`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    /// Dot-separated name (labels lossy-UTF-8).
    pub name: String,
    /// Query type (1=A, 2=NS, 5=CNAME, 15=MX, 16=TXT, 28=AAAA, 255=ANY).
    pub qtype: u16,
    /// Query class (1 = IN).
    pub qclass: u16,
}

/// Resource record: name, type, class, ttl, and typed data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RR {
    /// Owner name.
    pub name: String,
    /// RR type.
    pub ty: u16,
    /// RR class.
    pub class: u16,
    /// Seconds.
    pub ttl: u32,
    /// Decoded rdata.
    pub data: RData,
}

/// Decoded rdata by type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RData {
    /// `A` — IPv4.
    A([u8; 4]),
    /// `AAAA` — IPv6.
    Aaaa([u8; 16]),
    /// `CNAME`, `NS`, or `PTR` — a domain name.
    Name(String),
    /// `MX` — preference + exchange name.
    Mx(u16, String),
    /// `SOA` — mname, rname, serial, refresh, retry, expire, minimum.
    Soa(String, String, u32, u32, u32, u32, u32),
    /// `TXT` — a list of character-strings.
    Txt(Vec<Vec<u8>>),
    /// Anything else — raw bytes.
    Unknown(Vec<u8>),
}

fn be16(d: &[u8], i: usize) -> Option<u16> {
    Some(((*d.get(i)? as u16) << 8) | *d.get(i + 1)? as u16)
}

fn be32(d: &[u8], i: usize) -> Option<u32> {
    Some(((be16(d, i)? as u32) << 16) | be16(d, i + 2)? as u32)
}

/// Decode a (possibly compressed) domain name at `pos`.
/// Returns `(name, end)` where `end` is the offset just past the
/// encoded form (after the first pointer, not the pointed-to data).
fn name(d: &[u8], pos: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
    let mut i = pos;
    let mut end = None;
    // At most one byte per label char + dots; pointer hops are bounded
    // by message length (each hop moves strictly backward… not
    // guaranteed, so bound by len regardless).
    for _ in 0..=d.len() {
        let l = *d.get(i)? as usize;
        match l & 0xC0 {
            0xC0 => {
                let ptr = ((l & 0x3F) << 8) | *d.get(i + 1)? as usize;
                if end.is_none() {
                    end = Some(i + 2);
                }
                if ptr >= d.len() {
                    return None;
                }
                i = ptr;
            }
            0 => {
                if l == 0 {
                    return Some((labels.join("."), end.unwrap_or(i + 1)));
                }
                if l > 63 {
                    return None;
                }
                let s = d.get(i + 1..i + 1 + l)?;
                labels.push(String::from_utf8_lossy(s).into_owned());
                i += 1 + l;
            }
            _ => return None, // 0x40/0x80 are reserved
        }
    }
    None
}

fn rr(d: &[u8], pos: usize) -> Option<(RR, usize)> {
    let (nm, mut i) = name(d, pos)?;
    let ty = be16(d, i)?;
    let class = be16(d, i + 2)?;
    let ttl = be32(d, i + 4)?;
    let rdlen = be16(d, i + 8)? as usize;
    i += 10;
    let rd_end = i.checked_add(rdlen)?;
    let raw = d.get(i..rd_end)?;
    let data = match ty {
        1 if rdlen == 4 => RData::A([raw[0], raw[1], raw[2], raw[3]]),
        28 if rdlen == 16 => {
            let mut a = [0u8; 16];
            a.copy_from_slice(raw);
            RData::Aaaa(a)
        }
        2 | 5 | 12 => RData::Name(name(d, i)?.0),
        15 => {
            let pref = be16(d, i)?;
            RData::Mx(pref, name(d, i + 2)?.0)
        }
        6 => {
            let (mname, p) = name(d, i)?;
            let (rname, p) = name(d, p)?;
            RData::Soa(
                mname,
                rname,
                be32(d, p)?,
                be32(d, p + 4)?,
                be32(d, p + 8)?,
                be32(d, p + 12)?,
                be32(d, p + 16)?,
            )
        }
        16 => {
            let mut txts = Vec::new();
            let mut j = i;
            while j < rd_end {
                let l = *d.get(j)? as usize;
                txts.push(d.get(j + 1..j + 1 + l)?.to_vec());
                j += 1 + l;
            }
            if j != rd_end {
                return None;
            }
            RData::Txt(txts)
        }
        _ => RData::Unknown(raw.to_vec()),
    };
    Some((
        RR {
            name: nm,
            ty,
            class,
            ttl,
            data,
        },
        rd_end,
    ))
}

/// Parse a whole DNS message; `None` on truncation, bad flags shape, or
/// malformed names/rdata.
pub fn parse(d: &[u8]) -> Option<Message> {
    if d.len() < 12 {
        return None;
    }
    let id = be16(d, 0)?;
    let flags = be16(d, 2)?;
    let qd = be16(d, 4)? as usize;
    let an = be16(d, 6)? as usize;
    let ns = be16(d, 8)? as usize;
    let ar = be16(d, 10)? as usize;
    let mut m = Message {
        id,
        is_response: flags & 0x8000 != 0,
        opcode: ((flags >> 11) & 0xF) as u8,
        aa: flags & 0x0400 != 0,
        tc: flags & 0x0200 != 0,
        rd: flags & 0x0100 != 0,
        ra: flags & 0x0080 != 0,
        rcode: (flags & 0xF) as u8,
        questions: Vec::new(),
        answers: Vec::new(),
        authorities: Vec::new(),
        additionals: Vec::new(),
    };
    let mut i = 12;
    for _ in 0..qd {
        let (nm, n) = name(d, i)?;
        i = n;
        m.questions.push(Question {
            name: nm,
            qtype: be16(d, i)?,
            qclass: be16(d, i + 2)?,
        });
        i += 4;
    }
    for (count, dst) in [
        (an, &mut m.answers),
        (ns, &mut m.authorities),
        (ar, &mut m.additionals),
    ] {
        for _ in 0..count {
            let (r, n) = rr(d, i)?;
            dst.push(r);
            i = n;
        }
    }
    Some(m)
}

/// Emit `name` as a sequence of length-prefixed labels + root.
fn push_name(out: &mut Vec<u8>, name: &str) -> Option<()> {
    for label in name.split('.') {
        if label.is_empty() || label.len() > 63 {
            return None;
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    Some(())
}

/// Big-endian u16 write by shifts (endian-independence rule).
fn w16(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

/// Build a standard recursive query (`RD=1`, one question, class IN).
/// `None` if `name` is malformed (empty or oversized labels).
pub fn build_query(id: u16, name: &str, qtype: u16) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(12 + name.len() + 6);
    w16(&mut out, id);
    w16(&mut out, 0x0100); // RD
    w16(&mut out, 1); // qd
    out.extend_from_slice(&[0; 6]); // an/ns/ar
    push_name(&mut out, name)?;
    w16(&mut out, qtype);
    w16(&mut out, 1);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_query_bytes() {
        let q = build_query(0x1234, "www.example.com", 1).unwrap();
        let want: Vec<u8> = vec![
            0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 3, b'w', b'w',
            b'w', 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0, 0, 1, 0, 1,
        ];
        assert_eq!(q, want);
        assert!(build_query(0, &format!("{}.x", "a".repeat(64)), 1).is_none());
    }

    #[test]
    fn parse_query_roundtrip() {
        let m = parse(&build_query(0xABCD, "a.b.c", 28).unwrap()).unwrap();
        assert!(!m.is_response && m.rd);
        assert_eq!(m.questions[0].qtype, 28);
        assert_eq!(m.questions[0].name, "a.b.c");
    }

    #[test]
    fn compressed_response() {
        // Query "example.com A" + answer with CNAME ptr 0xC00C.
        let mut d = build_query(0x1, "example.com", 1).unwrap();
        d[2] = 0x81;
        d[3] = 0x80; // response, RA, no error
        d[7] = 1; // an=1
        d.extend_from_slice(&[
            0xC0, 0x0C, // name ptr -> question name
            0x00, 0x01, 0x00, 0x01, // A IN
            0x00, 0x00, 0x00, 0x3C, // ttl 60
            0x00, 0x04, 93, 184, 216, 34, // 93.184.216.34
        ]);
        let m = parse(&d).unwrap();
        assert_eq!(m.answers.len(), 1);
        assert_eq!(m.answers[0].name, "example.com");
        assert_eq!(m.answers[0].data, RData::A([93, 184, 216, 34]));
        assert_eq!(m.answers[0].ttl, 60);
    }

    #[test]
    fn malformed_and_pointer_loops() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0; 11]).is_none());
        // Question name that pointers to itself at offset 12.
        let mut d = vec![0u8; 12];
        d[4] = 0;
        d[5] = 1; // qd=1
        d.extend_from_slice(&[0xC0, 0x0C]); // name = ptr to itself
        d.extend_from_slice(&[0, 1, 0, 1]);
        assert!(parse(&d).is_none());
        // Truncated question.
        let mut t = build_query(1, "ok", 1).unwrap();
        t.pop();
        assert!(parse(&t).is_none());
        // Reserved label form (0x40).
        let mut r = vec![0u8; 12];
        r[5] = 1;
        r.extend_from_slice(&[0x40, 0x00, 0, 1, 0, 1]);
        assert!(parse(&r).is_none());
    }

    #[test]
    fn typed_rdata() {
        // MX answer: pref 10, exchange mail.example.com (full labels).
        let mut d = build_query(0x9, "example.com", 15).unwrap();
        d[2] = 0x81;
        d[3] = 0x80;
        d[7] = 1;
        let mut rd = vec![0, 10]; // pref
        rd.extend_from_slice(&[4, b'm', b'a', b'i', b'l', 0xC0, 0x0C]);
        d.extend_from_slice(&[0xC0, 0x0C, 0, 15, 0, 1, 0, 0, 1, 0]);
        d.extend_from_slice(&[(rd.len() >> 8) as u8, rd.len() as u8]);
        d.extend_from_slice(&rd);
        let m = parse(&d).unwrap();
        assert_eq!(
            m.answers[0].data,
            RData::Mx(10, "mail.example.com".to_string())
        );
    }
}
