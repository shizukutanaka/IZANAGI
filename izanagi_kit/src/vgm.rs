//! VGM — the Video Game Music log format. A 0x100-byte header of
//! little-endian u32 fields (version is packed BCD, so `0x171` means
//! v1.71), then the data block of chip-write / wait commands and an
//! optional `Gd3 ` tag holding UTF-16LE metadata strings.
//!
//! [`parse`] validates the magic, the EOF-length marker
//! (`eof = file_len - 4`), and locates the data block
//! (`0x40` for versions before `0x150`; after that the relative
//! offset at `0x34` applies, a `0` meaning "at `0x40`").
//! [`commands`] walks the stream into [`Cmd`]s and accumulates wait
//! time in 44100 Hz samples.
//!
//! ```
//! use izanagi_kit::vgm::{parse, Cmd};
//!
//! let mut d = b"Vgm ".to_vec();
//! let put = |off: usize, v: u32, d: &mut Vec<u8>| {
//!     d.resize(off, 0);
//!     d.extend_from_slice(&v.to_le_bytes());
//! };
//! put(0x04, 0, &mut d); put(0x08, 0x150, &mut d);   // eof, version
//! put(0x0C, 3579545, &mut d);                       // SN76489 clock
//! put(0x18, 44100, &mut d);                         // total samples
//! put(0x34, 0x0C, &mut d);                          // data at 0x40
//! d.resize(0x40, 0);
//! d.extend_from_slice(&[0x62, 0x66]);               // wait 735, end
//! d[4] = (d.len() - 4) as u8;
//! let v = parse(&d).unwrap();
//! let cmds: Vec<Cmd> = izanagi_kit::vgm::commands(&d, v.data_at).collect();
//! assert_eq!(cmds, [Cmd::Wait735, Cmd::End]);
//! ```

use std::string::String;
use std::vec::Vec;

/// `Vgm ` file magic.
pub const MAGIC: &[u8] = b"Vgm ";
/// `Gd3 ` tag magic.
pub const GD3_MAGIC: &[u8] = b"Gd3 ";
/// Samples per second the wait commands count in.
pub const RATE: u32 = 44_100;

/// A parsed VGM header.
#[derive(Clone, Debug, PartialEq)]
pub struct Vgm {
    /// Packed-BCD version (`0x171` = v1.71).
    pub version: u32,
    /// SN76489 clock in Hz (0 = absent).
    pub sn76489_clk: u32,
    /// YM2413 clock (bit 30 = dual-chip marker on any clock field).
    pub ym2413_clk: u32,
    /// Total playback length in 44100 Hz samples.
    pub total_samples: u32,
    /// Loop start offset into the file, `0` = no loop.
    pub loop_at: u32,
    /// Loop length in samples.
    pub loop_samples: u32,
    /// Recording rate in Hz (v1.01+; 0 for older).
    pub rate: u32,
    /// YM2612 clock.
    pub ym2612_clk: u32,
    /// YM2151 clock.
    pub ym2151_clk: u32,
    /// File offset of the command data block.
    pub data_at: usize,
    /// File offset of the GD3 tag, `0` = none.
    pub gd3_at: usize,
}

/// One stream command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    /// `0x61` — wait *n* samples.
    WaitN(u16),
    /// `0x62` — wait 735 samples (60 Hz frame).
    Wait735,
    /// `0x63` — wait 882 samples (50 Hz frame).
    Wait882,
    /// `0x70-0x7F` — wait *n+1* samples.
    Wait(u8),
    /// `0x80-0x8F` — YM2612 port-0 write plus wait *n+1*.
    WaitYm2612(u8),
    /// `0x66` — end of data.
    End,
    /// A chip write or anything else; argument is the operand size in
    /// bytes consumed after the opcode.
    Other(u8),
}

fn u32le(d: &[u8], at: usize) -> u32 {
    u32::from(d[at])
        | u32::from(d[at + 1]) << 8
        | u32::from(d[at + 2]) << 16
        | u32::from(d[at + 3]) << 24
}

/// Parse the header; `None` on bad magic, short file, or an EOF
/// marker that disagrees with the actual length.
pub fn parse(d: &[u8]) -> Option<Vgm> {
    if d.len() < 0x40 || &d[..4] != MAGIC {
        return None;
    }
    let eof = u32le(d, 4) as usize;
    if eof + 4 != d.len() {
        return None;
    }
    let version = u32le(d, 8);
    let gd3_off = u32le(d, 0x14);
    let data_off = if version >= 0x150 { u32le(d, 0x34) } else { 0 };
    Some(Vgm {
        version,
        sn76489_clk: u32le(d, 0x0C),
        ym2413_clk: u32le(d, 0x10),
        total_samples: u32le(d, 0x18),
        loop_at: if u32le(d, 0x1C) == 0 {
            0
        } else {
            0x1C + u32le(d, 0x1C)
        },
        loop_samples: u32le(d, 0x20),
        rate: u32le(d, 0x24),
        ym2612_clk: u32le(d, 0x2C),
        ym2151_clk: u32le(d, 0x30),
        data_at: if version < 0x150 || data_off == 0 {
            0x40
        } else {
            0x34 + data_off as usize
        },
        gd3_at: if gd3_off == 0 {
            0
        } else {
            0x14 + gd3_off as usize
        },
    })
}

/// Iterate the data block. Yields [`Cmd::End`] once and stops;
/// unknown opcodes are skipped by their operand-size table.
pub fn commands<'a>(d: &'a [u8], at: usize) -> impl Iterator<Item = Cmd> + 'a {
    let mut at = at;
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        let op = *d.get(at)?;
        if op == 0x66 {
            done = true;
        }
        let (cmd, operand_len) = match op {
            0x66 => (Cmd::End, 0),
            0x61 => {
                let lo = u16::from(*d.get(at + 1)?) | u16::from(*d.get(at + 2)?) << 8;
                (Cmd::WaitN(lo), 2)
            }
            0x62 => (Cmd::Wait735, 0),
            0x63 => (Cmd::Wait882, 0),
            0x70..=0x7F => (Cmd::Wait((op & 0x0F) + 1), 0),
            0x80..=0x8F => (Cmd::WaitYm2612((op & 0x0F) + 1), 0),
            // documented operand sizes (vgm spec command table)
            0x4F | 0x50..=0x56 | 0x59..=0x5B | 0x5D..=0x5F | 0xA0..=0xBF | 0xC8 => {
                (Cmd::Other(op), 2)
            }
            0x67 => {
                // data block: 0x67 0x66 type len32 data[len]
                let len = if d.get(at + 1) == Some(&0x66) {
                    d.get(at + 3..at + 7)
                        .map(|b| u32le(b, 0) as usize)
                        .unwrap_or(0)
                } else {
                    0
                };
                (Cmd::Other(op), 6 + len)
            }
            0x68 => (Cmd::Other(op), 11),
            0x30..=0x3F
            | 0x40..=0x4E
            | 0x57
            | 0x58
            | 0x5C
            | 0x90..=0x95
            | 0xC0..=0xC7
            | 0xD0..=0xD6 => (Cmd::Other(op), 1),
            0xC9..=0xCF | 0xD7..=0xDF | 0xE0..=0xFF => (Cmd::Other(op), 3),
            _ => (Cmd::Other(op), 0),
        };
        at = at.checked_add(1 + operand_len)?;
        if at > d.len() {
            return None;
        }
        Some(cmd)
    })
}

/// Parse the `Gd3 ` metadata block into its NUL-separated UTF-16LE
/// fields (order: track EN/JA, game EN/JA, system EN/JA, author
/// EN/JA, release date, creator, notes).
pub fn gd3(d: &[u8], at: usize) -> Option<Vec<String>> {
    if d.get(at..at + 4)? != GD3_MAGIC {
        return None;
    }
    let len = u32le(d, at + 8) as usize;
    let body = d.get(at + 12..at + 12 + len)?;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    while i + 1 < body.len() {
        let u = u16::from(body[i]) | u16::from(body[i + 1]) << 8;
        i += 2;
        if u == 0 {
            out.push(std::mem::take(&mut cur));
        } else if let Some(c) = char::from_u32(u32::from(u)) {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vgm150() -> Vec<u8> {
        let mut d = b"Vgm ".to_vec();
        let put = |off: usize, v: u32, d: &mut Vec<u8>| {
            d.resize(off, 0);
            d.extend_from_slice(&v.to_le_bytes());
        };
        put(0x08, 0x150, &mut d);
        put(0x0C, 3_579_545, &mut d);
        put(0x10, 3_579_545 | 0x4000_0000, &mut d); // dual YM2413
        put(0x18, 44_100, &mut d);
        put(0x1C, 0x2A, &mut d); // loop at 0x46
        put(0x20, 22_050, &mut d);
        put(0x34, 0x0C, &mut d);
        d.resize(0x40, 0);
        d.extend_from_slice(&[0x62, 0x63, 0x70, 0x80, 0x61, 0xE8, 0x03, 0x66]);
        let len = (d.len() - 4) as u32;
        d[4..8].copy_from_slice(&len.to_le_bytes());
        d
    }

    #[test]
    fn header_and_stream() {
        let d = vgm150();
        let v = parse(&d).unwrap();
        assert_eq!(v.version, 0x150);
        assert_eq!(v.sn76489_clk, 3_579_545);
        assert_eq!(v.total_samples, 44_100);
        assert_eq!(v.loop_at, 0x46);
        assert_eq!(v.loop_samples, 22_050);
        assert_eq!(v.data_at, 0x40);
        assert_eq!(v.gd3_at, 0);
        let cmds: Vec<Cmd> = commands(&d, v.data_at).collect();
        assert_eq!(
            cmds,
            [
                Cmd::Wait735,
                Cmd::Wait882,
                Cmd::Wait(1),
                Cmd::WaitYm2612(1),
                Cmd::WaitN(1000),
                Cmd::End
            ]
        );
    }

    #[test]
    fn gd3_tag() {
        let d = vgm150();
        let mut t = d.clone();
        let at = t.len();
        let mut body = Vec::new();
        for s in ["Track", "曲", "Game", "", "System", "", "Author"] {
            for c in s.encode_utf16() {
                body.extend_from_slice(&c.to_le_bytes());
            }
            body.extend_from_slice(&0u16.to_le_bytes());
        }
        t.extend_from_slice(GD3_MAGIC);
        t.extend_from_slice(&0x100u32.to_le_bytes());
        t.extend_from_slice(&(body.len() as u32).to_le_bytes());
        t.extend_from_slice(&body);
        let len = (t.len() - 4) as u32;
        t[4..8].copy_from_slice(&len.to_le_bytes());
        // gd3 offset field = at - 0x14
        t[0x14..0x18].copy_from_slice(&((at - 0x14) as u32).to_le_bytes());
        let v = parse(&t).unwrap();
        assert_eq!(v.gd3_at, at);
        let fields = gd3(&t, v.gd3_at).unwrap();
        assert_eq!(fields[0], "Track");
        assert_eq!(fields[1], "曲");
        assert_eq!(fields[6], "Author");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"Vgm x").is_none());
        let mut d = vgm150();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d = vgm150();
        d.truncate(d.len() - 1); // eof marker now wrong
        assert!(parse(&d).is_none());
    }
}
