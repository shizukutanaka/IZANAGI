//! FastTracker II XM module (`.xm`).
//!
//! `Extended Module: ` + `name[20]` + `0x1A` + `tracker[20]` +
//! `version u16 | header_size u32 | song_len | restart | channels |
//! patterns | instruments | flags | tempo | bpm | orders[256]`.
//! `header_size` counts from the size field itself, so patterns start
//! at `60 + header_size`. Each pattern is
//! `len u32 | packing u8 | rows u16 | packed_size u16 | data`, and a
//! cell is five fields (note/instrument/volume/effect/param) either
//! verbatim or as a `0x80`-flagged bitmask.
//!
//! ```
//! use izanagi_kit::xm::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 9 + 2];
//! d[..17].copy_from_slice(b"Extended Module: ");
//! d[17..22].copy_from_slice(b"SONG!");
//! d[37] = 0x1A;
//! d[58..60].copy_from_slice(&0x0104u16.to_le_bytes());
//! d[60..64].copy_from_slice(&276u32.to_le_bytes());
//! d[64..66].copy_from_slice(&1u16.to_le_bytes());  // song len
//! d[68..70].copy_from_slice(&2u16.to_le_bytes());  // channels
//! d[70..72].copy_from_slice(&1u16.to_le_bytes());  // patterns
//! d[72..74].copy_from_slice(&0u16.to_le_bytes());  // instruments
//! d[76..78].copy_from_slice(&6u16.to_le_bytes());  // tempo
//! d[78..80].copy_from_slice(&125u16.to_le_bytes());// bpm
//! // pattern 0: 9-byte header, 1 row, 2 bytes packed (one full cell)
//! let p = HEADER;
//! d[p..p + 4].copy_from_slice(&9u32.to_le_bytes());
//! d[p + 4] = 0;
//! d[p + 5..p + 7].copy_from_slice(&1u16.to_le_bytes()); // rows
//! d[p + 7..p + 9].copy_from_slice(&2u16.to_le_bytes()); // packed size
//! let x = parse(&d).unwrap();
//! assert_eq!(x.title, "SONG!");
//! assert_eq!(x.channels, 2);
//! let pat = x.pattern(&d, 0).unwrap();
//! assert_eq!(pat.rows, 1);
//! ```

use std::string::String;
use std::vec::Vec;

/// Main header length: `60 + header_size` for the canonical 276-byte
/// pattern-order region.
pub const HEADER: usize = 336;
/// Standard order-table cell count.
pub const ORDERS: usize = 256;

/// Playback frequency table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    /// Amiga periods.
    Amiga,
    /// Linear periods (bit 0 of flags set).
    Linear,
}

/// One decoded pattern cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    /// Note number (0 = none, 1..96 = C-0..B-7, 97 = key off).
    pub note: u8,
    /// Instrument number.
    pub instrument: u8,
    /// Volume column byte.
    pub volume: u8,
    /// Effect command.
    pub effect: u8,
    /// Effect parameter.
    pub effect_param: u8,
}

/// A pattern: row count plus its packed data region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    /// Rows (1..=256).
    pub rows: u16,
    /// Byte offset of the packed cell data.
    pub data_at: usize,
    /// Packed data length.
    pub data_len: usize,
}

impl Pattern {
    /// Decode every cell row-major. Truncated data stops the walk.
    pub fn cells<'a>(&self, d: &'a [u8], channels: usize) -> Cells<'a> {
        let end = self.data_at.saturating_add(self.data_len).min(d.len());
        Cells {
            d: &d[self.data_at.min(d.len())..end],
            at: 0,
            n: 0,
            total: usize::from(self.rows) * channels,
        }
    }
}

/// Decoded-cell iterator.
pub struct Cells<'a> {
    d: &'a [u8],
    at: usize,
    n: usize,
    total: usize,
}

impl<'a> Iterator for Cells<'a> {
    type Item = Cell;
    fn next(&mut self) -> Option<Cell> {
        if self.n >= self.total {
            return None;
        }
        let first = *self.d.get(self.at)?;
        self.at += 1;
        let mut c = Cell::default();
        if first & 0x80 != 0 {
            if first & 0x01 != 0 {
                c.note = *self.d.get(self.at)?;
                self.at += 1;
            }
            if first & 0x02 != 0 {
                c.instrument = *self.d.get(self.at)?;
                self.at += 1;
            }
            if first & 0x04 != 0 {
                c.volume = *self.d.get(self.at)?;
                self.at += 1;
            }
            if first & 0x08 != 0 {
                c.effect = *self.d.get(self.at)?;
                self.at += 1;
            }
            if first & 0x10 != 0 {
                c.effect_param = *self.d.get(self.at)?;
                self.at += 1;
            }
        } else {
            c.note = first;
            c.instrument = *self.d.get(self.at)?;
            c.volume = *self.d.get(self.at + 1)?;
            c.effect = *self.d.get(self.at + 2)?;
            c.effect_param = *self.d.get(self.at + 3)?;
            self.at += 4;
        }
        self.n += 1;
        Some(c)
    }
}

/// One instrument's header (sample headers may follow).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instrument {
    /// Name (22 bytes, trimmed).
    pub name: String,
    /// Number of sample slots.
    pub samples: u16,
    /// Byte offset of the instrument's sample-header table.
    pub at: usize,
    /// Instrument header size (the field's own value).
    pub header_size: u32,
    /// Bytes per sample header (the field's own value, usually 40).
    pub sample_header_size: u32,
}

/// One XM sample header (40 bytes: lengths, loop, volume, name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// Sample length in bytes.
    pub len: u32,
    /// Loop start (bytes) — `0` when `kind` has no loop bit.
    pub loop_start: u32,
    /// Loop length (bytes).
    pub loop_len: u32,
    /// Volume 0..64.
    pub volume: u8,
    /// Finetune (signed nibble semantics).
    pub finetune: i8,
    /// Type byte: low 2 bits loop kind, bit 4 16-bit.
    pub kind: u8,
    /// Panning 0..255.
    pub pan: u8,
    /// Relative note (signed).
    pub rel_note: i8,
    /// Name (22 bytes, trimmed).
    pub name: String,
    /// Sample data offset.
    pub data_at: usize,
}

/// A parsed XM module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xm {
    /// Module title.
    pub title: String,
    /// Tracker name.
    pub tracker: String,
    /// File version (e.g. `0x0104`).
    pub version: u16,
    /// Song length in orders.
    pub song_len: u16,
    /// Restart position.
    pub restart: u16,
    /// Channel count.
    pub channels: u16,
    /// Pattern count.
    pub patterns: u16,
    /// Instrument count.
    pub instruments: u16,
    /// Frequency table selection.
    pub freq: Freq,
    /// Tempo (ticks per row).
    pub tempo: u16,
    /// BPM.
    pub bpm: u16,
    /// Order table.
    pub orders: Vec<u8>,
    /// Absolute offset where pattern headers begin.
    pub patterns_at: usize,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

fn txt(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).trim_end().to_string()
}

/// Parse the XM header, or `None` on bad magic, short file or a
/// degenerate channel/pattern count.
pub fn parse(d: &[u8]) -> Option<Xm> {
    if d.get(..17)? != b"Extended Module: " || *d.get(37)? != 0x1A {
        return None;
    }
    let hsize = u32le(d, 60)? as usize;
    if !(20..=276 + 32 * 1024).contains(&hsize) {
        return None;
    }
    let channels = u16le(d, 68)?;
    if channels == 0 || channels > 64 {
        return None;
    }
    let mut orders = vec![0u8; ORDERS];
    orders.copy_from_slice(d.get(80..336)?);
    Some(Xm {
        title: txt(&d[17..37]),
        tracker: txt(&d[38..58]),
        version: u16le(d, 58)?,
        song_len: u16le(d, 64)?,
        restart: u16le(d, 66)?,
        channels,
        patterns: u16le(d, 70)?,
        instruments: u16le(d, 72)?,
        freq: if d[74] & 1 != 0 {
            Freq::Linear
        } else {
            Freq::Amiga
        },
        tempo: u16le(d, 76)?,
        bpm: u16le(d, 78)?,
        orders,
        patterns_at: 60 + hsize,
    })
}

impl Xm {
    /// Pattern `i`'s header + packed data location.
    pub fn pattern(&self, d: &[u8], i: usize) -> Option<Pattern> {
        if i >= usize::from(self.patterns) {
            return None;
        }
        let mut at = self.patterns_at;
        for _ in 0..i {
            let hlen = u32le(d, at)? as usize;
            let packed = u16le(d, at + 7)? as usize;
            at = at.checked_add(hlen)?.checked_add(packed)?;
        }
        let hlen = u32le(d, at)? as usize;
        let rows = u16le(d, at + 5)?;
        let packed = u16le(d, at + 7)? as usize;
        let data_at = at.checked_add(hlen)?;
        d.get(data_at..data_at + packed)?;
        Some(Pattern {
            rows,
            data_at,
            data_len: packed,
        })
    }

    /// Decode pattern `i`'s cells.
    pub fn pattern_cells<'a>(&self, d: &'a [u8], i: usize) -> Option<Cells<'a>> {
        let p = self.pattern(d, i)?;
        Some(p.cells(d, usize::from(self.channels)))
    }

    /// Instrument `i`'s header. Instruments follow all patterns.
    pub fn instrument(&self, d: &[u8], i: usize) -> Option<Instrument> {
        if i >= usize::from(self.instruments) {
            return None;
        }
        let mut at = self.patterns_at;
        for _ in 0..self.patterns {
            let hlen = u32le(d, at)? as usize;
            let packed = u16le(d, at + 7)? as usize;
            at = at.checked_add(hlen)?.checked_add(packed)?;
        }
        for _ in 0..i {
            let size = u32le(d, at)? as usize;
            let ns = u16le(d, at + 27)?;
            let shdr = if ns > 0 {
                u32le(d, at + 29)? as usize
            } else {
                0
            };
            // skip instrument header + sample headers + sample data
            let p = at.checked_add(size)?;
            let shdr_table = p;
            let mut data = shdr_table.checked_add(shdr.checked_mul(usize::from(ns))?)?;
            for s in 0..usize::from(ns) {
                let _ = s;
                let slen = u32le(d, shdr_table + s * 40)? as usize;
                data = data.checked_add(slen)?;
            }
            at = data;
        }
        let size = u32le(d, at)?;
        let ns = u16le(d, at + 27)?;
        let mut name = [0u8; 22];
        name.copy_from_slice(d.get(at + 4..at + 26)?);
        let shdr = if ns > 0 { u32le(d, at + 29)? } else { 0 };
        Some(Instrument {
            name: txt(&name),
            samples: ns,
            at: at + usize::try_from(size).ok()?,
            header_size: size,
            sample_header_size: shdr,
        })
    }

    /// Sample `s` of instrument `i`.
    pub fn sample(&self, d: &[u8], i: usize, s: usize) -> Option<Sample> {
        let ins = self.instrument(d, i)?;
        if s >= usize::from(ins.samples) || ins.sample_header_size == 0 {
            return None;
        }
        let stride = usize::try_from(ins.sample_header_size).ok()?;
        let h = ins.at.checked_add(s.checked_mul(stride)?)?;
        let base = u32le(d, h)?;
        let mut name = [0u8; 22];
        name.copy_from_slice(d.get(h + 18..h + 40)?);
        // sample data starts after the whole header table + earlier samples
        let mut data = ins
            .at
            .checked_add(usize::from(ins.samples).checked_mul(stride)?)?;
        for k in 0..s {
            data = data.checked_add(u32le(d, ins.at + k * stride)? as usize)?;
        }
        Some(Sample {
            len: base,
            loop_start: u32le(d, h + 4)?,
            loop_len: u32le(d, h + 8)?,
            volume: *d.get(h + 12)?,
            finetune: *d.get(h + 13)? as i8,
            kind: *d.get(h + 14)?,
            pan: *d.get(h + 15)?,
            rel_note: *d.get(h + 16)? as i8,
            name: txt(&name),
            data_at: data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn module(pat_rows: u16, pat_data: &[u8]) -> Vec<u8> {
        let mut d = vec![0u8; HEADER + 9 + pat_data.len()];
        d[..17].copy_from_slice(b"Extended Module: ");
        d[17..25].copy_from_slice(b"TEST SNG");
        d[37] = 0x1A;
        d[38..45].copy_from_slice(b"FastTrk");
        d[58..60].copy_from_slice(&0x0104u16.to_le_bytes());
        d[60..64].copy_from_slice(&276u32.to_le_bytes());
        d[64..66].copy_from_slice(&1u16.to_le_bytes());
        d[68..70].copy_from_slice(&2u16.to_le_bytes());
        d[70..72].copy_from_slice(&1u16.to_le_bytes());
        d[74..76].copy_from_slice(&1u16.to_le_bytes());
        d[76..78].copy_from_slice(&6u16.to_le_bytes());
        d[78..80].copy_from_slice(&125u16.to_le_bytes());
        let p = HEADER;
        d[p..p + 4].copy_from_slice(&9u32.to_le_bytes());
        d[p + 5..p + 7].copy_from_slice(&pat_rows.to_le_bytes());
        d[p + 7..p + 9].copy_from_slice(&(pat_data.len() as u16).to_le_bytes());
        d[p + 9..p + 9 + pat_data.len()].copy_from_slice(pat_data);
        d
    }

    #[test]
    fn header_fields() {
        let d = module(64, &[]);
        let x = parse(&d).unwrap();
        assert_eq!(x.title, "TEST SNG");
        assert_eq!(x.tracker, "FastTrk");
        assert_eq!(x.version, 0x0104);
        assert_eq!(x.song_len, 1);
        assert_eq!(x.channels, 2);
        assert_eq!(x.patterns, 1);
        assert_eq!(x.freq, Freq::Linear);
        assert_eq!(x.tempo, 6);
        assert_eq!(x.bpm, 125);
        assert_eq!(x.orders[0], 0);
        assert_eq!(x.patterns_at, 60 + 276);
    }

    #[test]
    fn pattern_header() {
        let d = module(64, &[0x80, 0x80]);
        let x = parse(&d).unwrap();
        let p = x.pattern(&d, 0).unwrap();
        assert_eq!(p.rows, 64);
        assert_eq!(p.data_len, 2);
        assert!(x.pattern(&d, 1).is_none());
    }

    #[test]
    fn unpack_full_and_masked_cells() {
        // 2 channels, 1 row: full cell then masked (note only)
        let data = [0x32, 0x02, 0x40, 0x0F, 0x55, 0x81, 0x40];
        let d = module(1, &data);
        let x = parse(&d).unwrap();
        let cells: Vec<_> = x.pattern_cells(&d, 0).unwrap().collect();
        assert_eq!(cells.len(), 2);
        assert_eq!(
            cells[0],
            Cell {
                note: 0x32,
                instrument: 2,
                volume: 0x40,
                effect: 0x0F,
                effect_param: 0x55
            }
        );
        assert_eq!(cells[1].note, 0x40);
        assert_eq!(cells[1].effect_param, 0);
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        let mut d = module(64, &[]);
        d[0] = b'X';
        assert_eq!(parse(&d), None);
        let mut d = module(64, &[]);
        d[37] = 0;
        assert_eq!(parse(&d), None);
        let mut d = module(64, &[]);
        d[68] = 0;
        assert_eq!(parse(&d), None); // zero channels
    }

    #[test]
    fn empty_instrument_list() {
        let d = module(1, &[0x80]);
        let x = parse(&d).unwrap();
        assert_eq!(x.instruments, 0);
        assert!(x.instrument(&d, 0).is_none());
    }
}
