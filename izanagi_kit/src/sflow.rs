//! sFlow v5 datagram (RFC 3176): `version` u32 = 5, `agent_address_type`
//! (1 = IPv4, 2 = IPv6) + agent address, `sub_agent`, `seq`, `uptime`,
//! `num_samples` — then `num_samples` records of `[format u32][len u32]`
//! whose data is padded to a multiple of 4.
//!
//! ```
//! let mut d = Vec::new();
//! d.extend_from_slice(&[0,0,0,5, 0,0,0,1, 10,0,0,1]); // v5 + IPv4 agent
//! d.extend_from_slice(&[0,0,0,0, 0,0,0,7, 0,0,0,9, 0,0,0,1]); // 1 sample
//! d.extend_from_slice(&[0,0,0,1, 0,0,0,4, 0xaa,0xbb,0xcc,0xdd]); // sample
//! let s = izanagi_kit::sflow::parse(&d).unwrap();
//! assert_eq!(s.agent, [10, 0, 0, 1]);
//! assert_eq!(s.samples.len(), 1);
//! ```

use std::vec::Vec;

/// One sFlow sample record.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Sample format (1 = flow sample, 2 = counter sample, …;
    /// enterprise bits live in the upper bits).
    pub format: u32,
    /// Byte length of the sample payload.
    pub len: u32,
    /// Offset of the sample body inside the input.
    pub body_offset: usize,
}

/// A parsed sFlow v5 datagram.
#[derive(Clone, Debug)]
pub struct Sflow {
    /// Agent IP address (4 or 16 bytes).
    pub agent: Vec<u8>,
    /// Sub-agent ID.
    pub sub_agent: u32,
    /// Datagram sequence number.
    pub sequence: u32,
    /// `sys_uptime` ms.
    pub uptime: u32,
    /// Sample records.
    pub samples: Vec<Sample>,
}

fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let d = d.get(o..o + 4)?;
    Some(
        (u32::from(d[0]) << 24)
            | (u32::from(d[1]) << 16)
            | (u32::from(d[2]) << 8)
            | u32::from(d[3]),
    )
}

/// Parse an sFlow datagram; `None` unless version 5, a known agent
/// address family, and all `num_samples` records fitting the buffer.
pub fn parse(d: &[u8]) -> Option<Sflow> {
    if u32be(d, 0)? != 5 {
        return None;
    }
    let addr_len = match u32be(d, 4)? {
        1 => 4usize,
        2 => 16usize,
        _ => return None,
    };
    let agent = d.get(8..8 + addr_len)?.to_vec();
    let mut off = 8 + addr_len;
    let sub_agent = u32be(d, off)?;
    let sequence = u32be(d, off + 4)?;
    let uptime = u32be(d, off + 8)?;
    let count = u32be(d, off + 12)? as usize;
    off += 16;
    let mut samples = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let format = u32be(d, off)?;
        let len = u32be(d, off + 4)? as usize;
        let data_off = off + 8;
        if len > d.len().saturating_sub(data_off) {
            return None;
        }
        samples.push(Sample {
            format,
            len: len as u32,
            body_offset: data_off,
        });
        off = data_off + len.div_ceil(4) * 4;
        if off > d.len() {
            return None;
        }
    }
    Some(Sflow {
        agent,
        sub_agent,
        sequence,
        uptime,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(count: u32) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&[0, 0, 0, 5, 0, 0, 0, 1, 192, 168, 0, 1]);
        v.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 5]);
        v.extend_from_slice(&[
            (count >> 24) as u8,
            (count >> 16) as u8,
            (count >> 8) as u8,
            count as u8,
        ]);
        v
    }

    #[test]
    fn two_samples() {
        let mut d = hdr(2);
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 5]); // fmt 1, len 5
        d.extend_from_slice(&[1, 2, 3, 4, 5, 0, 0, 0]); // 5B + 3 pad
        d.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 4]); // fmt 2, len 4
        d.extend_from_slice(&[9, 8, 7, 6]);
        let s = parse(&d).unwrap();
        assert_eq!(s.agent, [192, 168, 0, 1]);
        assert_eq!(s.samples.len(), 2);
        assert_eq!(s.samples[0].len, 5);
        assert_eq!(s.samples[1].format, 2);
        assert_eq!(s.uptime, 5);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut d = hdr(0);
        d[3] = 4; // version 4
        assert!(parse(&d).is_none());
        let mut d = hdr(0);
        d[7] = 3; // bad addr type
        assert!(parse(&d).is_none());
        let mut d = hdr(1);
        d.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 100]); // len 100 > buf
        assert!(parse(&d).is_none());
    }
}
