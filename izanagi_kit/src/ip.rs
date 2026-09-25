//! IPv4/IPv6 address and CIDR-block parsing, formatting, and
//! containment (RFC 791 / RFC 4291 / RFC 4632).
//!
//! [`Ipv6::parse`] accepts full `::` compression, embedded dotted-IPv4
//! tails, and uppercase hex; [`Ipv6::format`] renders the canonical
//! compressed lowercase form (RFC 5952). [`Cidr`] containment covers both
//! families. Malformed input degrades to `None`.
//!
//! ```
//! use izanagi_kit::ip::{Ipv4, Ipv6, Cidr};
//!
//! let a = Ipv4::parse("192.168.1.7").unwrap();
//! assert!(Cidr::parse("192.168.0.0/16").unwrap().contains_v4(a));
//! assert_eq!(Ipv6::parse("::ffff:192.168.1.7").unwrap().segments()[6], 0xc0a8);
//! ```

use std::string::String;
use std::vec::Vec;

/// An IPv4 host address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Ipv4(pub u32);

impl Ipv4 {
    /// Parse `a.b.c.d` — four decimal octets, no leading zeros beyond
    /// `"0"` itself (matching the modern strict-URL form).
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = [0u32; 4];
        let mut it = s.split('.');
        for p in parts.iter_mut() {
            let piece = it.next()?;
            if piece.is_empty()
                || piece.len() > 3
                || (piece.len() > 1 && piece.starts_with('0'))
                || !piece.bytes().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            let mut v = 0u32;
            for c in piece.bytes() {
                v = v * 10 + (c - b'0') as u32;
            }
            if v > 255 {
                return None;
            }
            *p = v;
        }
        if it.next().is_some() {
            return None;
        }
        Some(Self(
            (parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3],
        ))
    }

    /// The four octets.
    pub fn octets(self) -> [u8; 4] {
        [
            (self.0 >> 24) as u8,
            (self.0 >> 16) as u8,
            (self.0 >> 8) as u8,
            self.0 as u8,
        ]
    }

    /// Canonical `a.b.c.d` string.
    pub fn format(self) -> String {
        let o = self.octets();
        std::format!("{}.{}.{}.{}", o[0], o[1], o[2], o[3])
    }

    /// RFC 1918 private ranges: 10/8, 172.16/12, 192.168/16.
    pub fn is_private(self) -> bool {
        let o = self.octets();
        o[0] == 10 || (o[0] == 172 && (16..=31).contains(&o[1])) || (o[0] == 192 && o[1] == 168)
    }

    /// 127.0.0.0/8.
    pub fn is_loopback(self) -> bool {
        self.octets()[0] == 127
    }

    /// 169.254.0.0/16 link-local.
    pub fn is_link_local(self) -> bool {
        let o = self.octets();
        o[0] == 169 && o[1] == 254
    }
}

/// An IPv6 host address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Ipv6(pub [u16; 8]);

impl Ipv6 {
    /// Parse RFC 4291 text: eight hextets, one `::` run, optional
    /// embedded `a.b.c.d` tail in the last two hextets.
    pub fn parse(s: &str) -> Option<Self> {
        let mut seg = [0u16; 8];
        // Split at "::" at most once.
        let (head, tail) = match s.split_once("::") {
            Some((a, b)) => {
                if b.contains("::") {
                    return None;
                }
                (a, Some(b))
            }
            None => (s, None),
        };
        let head_parts = Self::half(head)?;
        let tail_parts = match tail {
            Some(t) => Self::half(t)?,
            None => Vec::new(),
        };
        let total = head_parts.len() + tail_parts.len();
        if tail.is_none() && total != 8 {
            return None;
        }
        if tail.is_some() && total > 7 {
            return None;
        }
        seg[..head_parts.len()].copy_from_slice(&head_parts);
        let zeros = 8 - total;
        seg[head_parts.len() + zeros..].copy_from_slice(&tail_parts);
        Some(Self(seg))
    }

    /// Parse one side of `::` into hextets (handles the v4 tail).
    fn half(s: &str) -> Option<Vec<u16>> {
        if s.is_empty() {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        let parts: Vec<&str> = s.split(':').collect();
        for (i, p) in parts.iter().enumerate() {
            if p.contains('.') {
                // Embedded IPv4 tail — must be last.
                if i != parts.len() - 1 {
                    return None;
                }
                let v = Ipv4::parse(p)?;
                out.push((v.0 >> 16) as u16);
                out.push(v.0 as u16);
            } else {
                if p.is_empty() || p.len() > 4 || !p.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return None;
                }
                let mut v = 0u16;
                for c in p.bytes() {
                    let d = (c as char).to_digit(16)? as u16;
                    v = (v << 4) | d;
                }
                out.push(v);
            }
        }
        Some(out)
    }

    /// The eight hextets.
    pub fn segments(self) -> [u16; 8] {
        self.0
    }

    /// Canonical RFC 5952 form: lowercase, longest zero run compressed
    /// (length ≥ 2, first run wins), no leading zeros.
    pub fn format(self) -> String {
        // Find the longest run of zero hextets.
        let (mut best_i, mut best_len, mut i) = (0usize, 0usize, 0usize);
        while i < 8 {
            if self.0[i] == 0 {
                let j = i;
                while i < 8 && self.0[i] == 0 {
                    i += 1;
                }
                if i - j > best_len {
                    best_i = j;
                    best_len = i - j;
                }
            } else {
                i += 1;
            }
        }
        if best_len < 2 {
            best_len = 0;
        }
        let mut s = String::new();
        let mut i = 0usize;
        while i < 8 {
            if i == best_i && best_len > 0 {
                s.push_str("::");
                i += best_len;
                continue;
            }
            if !s.is_empty() && !s.ends_with(':') {
                s.push(':');
            }
            s.push_str(&std::format!("{:x}", self.0[i]));
            i += 1;
        }
        if s.is_empty() {
            s.push_str("::");
        }
        s
    }

    /// IPv4-mapped `::ffff:a.b.c.d`.
    pub fn to_ipv4_mapped(self) -> Option<Ipv4> {
        if self.0[..5] == [0; 5] && self.0[5] == 0xffff {
            Some(Ipv4(((self.0[6] as u32) << 16) | self.0[7] as u32))
        } else {
            None
        }
    }
}

/// A CIDR block over either family.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cidr {
    /// IPv4 prefix.
    V4(Ipv4, u8),
    /// IPv6 prefix.
    V6(Ipv6, u8),
}

impl Cidr {
    /// Parse `address/prefixlen`.
    pub fn parse(s: &str) -> Option<Self> {
        let (a, p) = s.split_once('/')?;
        if p.is_empty() || p.len() > 3 || !p.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut bits = 0u32;
        for c in p.bytes() {
            bits = bits * 10 + (c - b'0') as u32;
        }
        if let Some(v4) = Ipv4::parse(a) {
            if bits > 32 {
                return None;
            }
            return Some(Self::V4(v4, bits as u8));
        }
        let v6 = Ipv6::parse(a)?;
        if bits > 128 {
            return None;
        }
        Some(Self::V6(v6, bits as u8))
    }

    /// Does this block contain the IPv4 address?
    pub fn contains_v4(self, a: Ipv4) -> bool {
        match self {
            Self::V4(base, bits) => {
                if bits == 0 {
                    return true;
                }
                let mask = u32::MAX << (32 - bits);
                (a.0 & mask) == (base.0 & mask)
            }
            Self::V6(_, _) => false,
        }
    }

    /// Does this block contain the IPv6 address?
    pub fn contains_v6(self, a: Ipv6) -> bool {
        match self {
            Self::V6(base, bits) => {
                let bits = bits as u32;
                for i in 0..8usize {
                    let lo = (i as u32) * 16;
                    let hi = lo + 16;
                    if bits >= hi {
                        if a.0[i] != base.0[i] {
                            return false;
                        }
                    } else if bits > lo {
                        let keep = bits - lo;
                        let mask = u16::MAX << (16 - keep);
                        return (a.0[i] & mask) == (base.0[i] & mask);
                    } else {
                        return true;
                    }
                }
                true
            }
            Self::V4(_, _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_parse_and_classify() {
        let a = Ipv4::parse("192.168.1.7").unwrap();
        assert_eq!(a.octets(), [192, 168, 1, 7]);
        assert_eq!(a.format(), "192.168.1.7");
        assert!(a.is_private());
        assert!(Ipv4::parse("127.0.0.1").unwrap().is_loopback());
        assert!(Ipv4::parse("169.254.0.1").unwrap().is_link_local());
        assert!(!Ipv4::parse("8.8.8.8").unwrap().is_private());
    }

    #[test]
    fn v4_rejects_malformed() {
        for s in [
            "",
            "1.2.3",
            "1.2.3.4.5",
            "256.0.0.1",
            "01.2.3.4",
            "1..2.3",
            "a.b.c.d",
        ] {
            assert!(Ipv4::parse(s).is_none(), "{s}");
        }
    }

    #[test]
    fn v6_full_and_compressed() {
        let a = Ipv6::parse("2001:0db8:0000:0000:0000:ff00:0042:8329").unwrap();
        assert_eq!(a.format(), "2001:db8::ff00:42:8329");
        assert_eq!(Ipv6::parse("::1").unwrap().format(), "::1");
        assert_eq!(Ipv6::parse("::").unwrap().format(), "::");
        let m = Ipv6::parse("::ffff:192.168.1.7").unwrap();
        assert_eq!(m.to_ipv4_mapped().unwrap().format(), "192.168.1.7");
        // Longest zero run wins; ties take the first.
        assert_eq!(
            Ipv6::parse("2001:0:0:1:0:0:0:1").unwrap().format(),
            "2001:0:0:1::1"
        );
    }

    #[test]
    fn v6_rejects_malformed() {
        for s in [
            "",
            "1:2:3:4:5:6:7",
            "1:2:3:4:5:6:7:8:9",
            "a::b::c",
            "gggg::1",
            "1:2:3:4:5:6:7:8::",
            ":1:2:3:4:5:6:7:8",
        ] {
            assert!(Ipv6::parse(s).is_none(), "{s}");
        }
    }

    #[test]
    fn cidr_contains() {
        let c = Cidr::parse("192.168.0.0/16").unwrap();
        assert!(c.contains_v4(Ipv4::parse("192.168.99.99").unwrap()));
        assert!(!c.contains_v4(Ipv4::parse("192.169.0.1").unwrap()));
        assert!(Cidr::parse("0.0.0.0/0")
            .unwrap()
            .contains_v4(Ipv4::parse("8.8.8.8").unwrap()));
        let v6 = Cidr::parse("2001:db8::/32").unwrap();
        assert!(v6.contains_v6(Ipv6::parse("2001:db8:ffff::1").unwrap()));
        assert!(!v6.contains_v6(Ipv6::parse("2001:db9::1").unwrap()));
        assert!(Cidr::parse("::/0")
            .unwrap()
            .contains_v6(Ipv6::parse("fe80::1").unwrap()));
    }

    #[test]
    fn cidr_rejects() {
        for s in ["1.2.3.4/33", "::/129", "1.2.3.4/", "nope/8", "1.2.3.4/-1"] {
            assert!(Cidr::parse(s).is_none(), "{s}");
        }
    }
}
