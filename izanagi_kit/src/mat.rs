//! MATLAB Level-4 `.mat` variables.
//!
//! A Level-4 file is a flat sequence of variable blocks — no global
//! header. Each block is a little-endian 20-byte preamble
//! (`type`, `mrows`, `ncols`, `imagf`, `namelen`), then `namelen`
//! bytes of NUL-terminated name, then `mrows * ncols` real samples
//! (plus an equal run of imaginary samples when `imagf != 0`). The
//! `type` word encodes endianness `M`, element precision `P`
//! (0 = f64, 1 = f32, 2 = i32, 3 = i16, 4 = u16, 5 = u8) and matrix
//! class `T` (0 = numeric, 1 = text, 2 = sparse) as `M*1000 +
//! P*10 + T`.
//!
//! ```
//! use izanagi_kit::mat::{parse, Prec};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(&0u32.to_le_bytes());  // MOPT: LE, f64, numeric
//! d.extend_from_slice(&2u32.to_le_bytes());  // mrows
//! d.extend_from_slice(&2u32.to_le_bytes());  // ncols
//! d.extend_from_slice(&0u32.to_le_bytes());  // real only
//! d.extend_from_slice(&2u32.to_le_bytes());  // "x\0"
//! d.extend_from_slice(b"x\0");
//! for v in [1.0f64, 2.0, 3.0, 4.0] { d.extend_from_slice(&v.to_bits().to_le_bytes()); }
//! let m = parse(&d).unwrap();
//! assert_eq!(m.vars.len(), 1);
//! assert_eq!(m.vars[0].name, "x");
//! assert_eq!(m.vars[0].prec, Prec::F64);
//! assert_eq!(m.vars[0].real(&d).unwrap().len(), 32);
//! ```

use std::string::String;
use std::vec::Vec;

/// Element precision (`P` field of `MOPT`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prec {
    /// `0` — IEEE-754 double (8 bytes).
    F64,
    /// `1` — single (4 bytes).
    F32,
    /// `2` — int32.
    I32,
    /// `3` — int16.
    I16,
    /// `4` — uint16.
    U16,
    /// `5` — uint8.
    U8,
    /// Any other precision code.
    Other(u32),
}

impl Prec {
    fn of(v: u32) -> Self {
        match v {
            0 => Prec::F64,
            1 => Prec::F32,
            2 => Prec::I32,
            3 => Prec::I16,
            4 => Prec::U16,
            5 => Prec::U8,
            v => Prec::Other(v),
        }
    }

    /// Bytes per element (`0` when unknown).
    pub fn bytes(&self) -> usize {
        match self {
            Prec::F64 => 8,
            Prec::F32 | Prec::I32 => 4,
            Prec::I16 | Prec::U16 => 2,
            Prec::U8 => 1,
            Prec::Other(_) => 0,
        }
    }
}

/// Matrix class (`T` field of `MOPT`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    /// `0` — full numeric matrix.
    Numeric,
    /// `1` — text matrix.
    Text,
    /// `2` — sparse matrix.
    Sparse,
    /// Any other class code.
    Other(u32),
}

/// One variable block.
#[derive(Clone, Debug, PartialEq)]
pub struct Var {
    /// Raw `MOPT` word.
    pub mopt: u32,
    /// Endianness field (0 = little-endian IEEE).
    pub endian: u32,
    /// Element precision.
    pub prec: Prec,
    /// Matrix class.
    pub class: Class,
    /// Row count.
    pub rows: u32,
    /// Column count.
    pub cols: u32,
    /// Nonzero when an imaginary part follows the real data.
    pub imag: u32,
    /// Variable name (NUL trimmed).
    pub name: String,
    /// Byte offset of the real data inside the file.
    pub data_at: usize,
    /// Byte length of one data part (real or imaginary).
    pub data_len: usize,
}

impl Var {
    /// The real part bytes inside `d`.
    pub fn real<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(self.data_at..self.data_at + self.data_len)
    }

    /// The imaginary part bytes inside `d`, when `imag` is set.
    pub fn imag_part<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        if self.imag == 0 {
            return None;
        }
        d.get(self.data_at + self.data_len..self.data_at + self.data_len * 2)
    }

    /// Next variable starts here (past both data parts).
    pub fn end_at(&self) -> usize {
        let parts = if self.imag == 0 { 1 } else { 2 };
        self.data_at
            .saturating_add(self.data_len.saturating_mul(parts))
    }
}

/// A parsed Level-4 file: all variable blocks in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Mat {
    /// Variables found.
    pub vars: Vec<Var>,
}

fn le32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// Parse a Level-4 `.mat` file. Returns `None` when a block is
/// truncated or its name/data region overruns the buffer. An empty
/// input yields an empty variable list (valid Level-4).
pub fn parse(d: &[u8]) -> Option<Mat> {
    let mut vars = Vec::new();
    let mut at = 0usize;
    while at < d.len() {
        let mopt = le32(d, at)?;
        let rows = le32(d, at + 4)?;
        let cols = le32(d, at + 8)?;
        let imag = le32(d, at + 12)?;
        let namelen = usize::try_from(le32(d, at + 16)?).ok()?;
        let nat = at + 20;
        let raw = d.get(nat..nat + namelen)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let name = String::from_utf8_lossy(raw.get(..end).unwrap_or(&[])).into_owned();
        let prec = Prec::of(mopt / 10 % 10);
        let elems = usize::try_from(rows)
            .ok()?
            .checked_mul(usize::try_from(cols).ok()?)?;
        let data_len = elems.checked_mul(prec.bytes())?;
        let data_at = nat + namelen;
        let parts = if imag == 0 { 1 } else { 2 };
        let data_end = data_at + data_len.saturating_mul(parts);
        let var = Var {
            mopt,
            endian: mopt / 1000 % 10,
            prec,
            class: match mopt % 10 {
                0 => Class::Numeric,
                1 => Class::Text,
                2 => Class::Sparse,
                v => Class::Other(v),
            },
            rows,
            cols,
            imag,
            name,
            data_at,
            data_len,
        };
        if data_end > d.len() || data_len == 0 && prec.bytes() == 0 {
            return None;
        }
        vars.push(var);
        at = data_end;
    }
    Some(Mat { vars })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn var(name: &[u8], mopt: u32, rows: u32, cols: u32, imag: u32, data: &[u8]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&mopt.to_le_bytes());
        d.extend_from_slice(&rows.to_le_bytes());
        d.extend_from_slice(&cols.to_le_bytes());
        d.extend_from_slice(&imag.to_le_bytes());
        d.extend_from_slice(&(name.len() as u32).to_le_bytes());
        d.extend_from_slice(name);
        d.extend_from_slice(data);
        d
    }

    #[test]
    fn numeric_f64() {
        let mut d = var(b"x\0", 0, 2, 2, 0, &[]);
        for v in [1.0f64, 2.0, 3.0, 4.0] {
            d.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        let m = parse(&d).unwrap();
        assert_eq!(m.vars.len(), 1);
        let v = &m.vars[0];
        assert_eq!(v.name, "x");
        assert_eq!(v.prec, Prec::F64);
        assert_eq!(v.class, Class::Numeric);
        assert_eq!(v.endian, 0);
        assert_eq!((v.rows, v.cols), (2, 2));
        assert_eq!(v.data_len, 32);
        assert_eq!(v.real(&d).unwrap().len(), 32);
        assert_eq!(v.imag_part(&d), None);
        assert_eq!(v.end_at(), d.len());
    }

    #[test]
    fn imaginary_and_multi_vars() {
        // u8 precision (P=5), text class (T=1): MOPT = 51
        let mut d = var(b"s\0", 51, 1, 4, 1, &[]);
        d.extend_from_slice(b"abcd");
        d.extend_from_slice(b"ABCD"); // imaginary part
        let mut d2 = var(b"y\0", 0, 1, 1, 0, &[]);
        d2.extend_from_slice(&42.0f64.to_bits().to_le_bytes());
        d.extend_from_slice(&d2);
        let m = parse(&d).unwrap();
        assert_eq!(m.vars.len(), 2);
        assert_eq!(m.vars[0].prec, Prec::U8);
        assert_eq!(m.vars[0].class, Class::Text);
        assert_eq!(m.vars[0].real(&d), Some(b"abcd".as_ref()));
        assert_eq!(m.vars[0].imag_part(&d), Some(b"ABCD".as_ref()));
        assert_eq!(m.vars[1].name, "y");
    }

    #[test]
    fn mopt_decodes_sparse_and_big_endian() {
        // M=1 (VAX/Big endian host marker), P=2 (i32), T=2 (sparse)
        let mut d = var(b"z\0", 1022, 1, 2, 0, &[]);
        d.extend_from_slice(&[0u8; 8]);
        let m = parse(&d).unwrap();
        assert_eq!(m.vars[0].endian, 1);
        assert_eq!(m.vars[0].prec, Prec::I32);
        assert_eq!(m.vars[0].class, Class::Sparse);
    }

    #[test]
    fn prec_bytes() {
        assert_eq!(Prec::F64.bytes(), 8);
        assert_eq!(Prec::F32.bytes(), 4);
        assert_eq!(Prec::I16.bytes(), 2);
        assert_eq!(Prec::U8.bytes(), 1);
        assert_eq!(Prec::Other(9).bytes(), 0);
    }

    #[test]
    fn rejects_and_empty() {
        assert_eq!(parse(&[]).unwrap().vars.len(), 0);
        assert_eq!(parse(&[0u8; 19]), None); // short preamble
        let mut d = var(b"q\0", 0, 4, 4, 0, &[]);
        d.extend_from_slice(&[0u8; 16]); // needs 128B, has 16
        assert_eq!(parse(&d), None);
    }
}
