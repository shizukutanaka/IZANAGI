//! LLVM bitcode container sniffing.
//!
//! Two on-disk forms: the raw bitcode stream starting `BC\xC0\xDE`,
//! and the wrapper used by Apple/Xcode (`0x0B17C0DE` u32LE + u32
//! version + u32 offset + u32 size + u32 cpuType) that embeds a raw
//! stream at `offset`. Bitcode blocks are left opaque — this module
//! only validates the envelope and locates the raw stream.
//!
//! ```
//! use izanagi_kit::llvmbc;
//! let raw = b"BC\xC0\xDErest...";
//! let l = llvmbc::parse(raw).unwrap();
//! assert_eq!(l.form, llvmbc::Form::Raw);
//! assert_eq!(l.stream_offset, 0);
//! ```

/// Raw bitcode magic `BC\xC0\xDE`.
pub const RAW_MAGIC: &[u8; 4] = b"BC\xC0\xDE";

/// Wrapper magic (little-endian `0x0B17C0DE`).
pub const WRAPPER_MAGIC: u32 = 0x0B17_C0DE;

/// Which container form was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Bare `BC\xC0\xDE` bitcode stream.
    Raw,
    /// Apple wrapper — raw stream at `offset`.
    Wrapped,
}

/// A parsed bitcode container.
#[derive(Clone, Debug, PartialEq)]
pub struct LlvmBc {
    /// Raw or wrapped.
    pub form: Form,
    /// Offset of the `BC\xC0\xDE` stream within the input.
    pub stream_offset: usize,
    /// Wrapper only: declared `version`.
    pub wrapper_version: u32,
    /// Wrapper only: declared `cpuType`.
    pub cpu_type: u32,
    /// Wrapper only: declared embedded stream size (≤ input span).
    pub stream_size: usize,
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a bitcode file: raw magic at 0, or the 20-byte wrapper
/// whose `offset`/`size` must locate a `BC\xC0\xDE` stream.
pub fn parse(d: &[u8]) -> Option<LlvmBc> {
    if d.len() < 4 {
        return None;
    }
    if d.get(..4)? == RAW_MAGIC {
        return Some(LlvmBc {
            form: Form::Raw,
            stream_offset: 0,
            wrapper_version: 0,
            cpu_type: 0,
            stream_size: d.len(),
        });
    }
    if u32le(d, 0)? == WRAPPER_MAGIC {
        let version = u32le(d, 4)?;
        let offset = u32le(d, 8)? as usize;
        let size = u32le(d, 12)? as usize;
        let cpu = u32le(d, 16)?;
        let end = offset.checked_add(size)?;
        if end > d.len() || size < 4 {
            return None;
        }
        if d.get(offset..offset + 4)? != RAW_MAGIC {
            return None;
        }
        return Some(LlvmBc {
            form: Form::Wrapped,
            stream_offset: offset,
            wrapper_version: version,
            cpu_type: cpu,
            stream_size: size,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn parses_raw() {
        let l = parse(b"BC\xC0\xDE\x01\x02").unwrap();
        assert_eq!(l.form, Form::Raw);
        assert_eq!(l.stream_offset, 0);
    }

    #[test]
    fn parses_wrapper() {
        let mut d = Vec::new();
        d.extend_from_slice(&WRAPPER_MAGIC.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes()); // version
        d.extend_from_slice(&20u32.to_le_bytes()); // offset
        d.extend_from_slice(&8u32.to_le_bytes()); // size
        d.extend_from_slice(&12u32.to_le_bytes()); // cpuType
        d.extend_from_slice(b"BC\xC0\xDEABCD");
        let l = parse(&d).unwrap();
        assert_eq!(l.form, Form::Wrapped);
        assert_eq!(l.stream_offset, 20);
        assert_eq!(l.stream_size, 8);
        assert_eq!(l.cpu_type, 12);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 3]).is_none());
        assert!(parse(b"ELF\x7F").is_none());
        // wrapper pointing at garbage
        let mut d = Vec::new();
        d.extend_from_slice(&WRAPPER_MAGIC.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&20u32.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(b"NOPEjunk");
        assert!(parse(&d).is_none());
    }
}
