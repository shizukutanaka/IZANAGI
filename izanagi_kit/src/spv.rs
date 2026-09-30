//! SPIR-V binary module: 4-byte magic `0x0723_0203` (little-endian
//! `\x03\x02\x23\x07`), then `version`, `generator`, `bound`,
//! `reserved`, and a stream of `word_count:u16 | opcode:u16`
//! instructions.
//!
//! ```
//! let mut d = vec![0x03, 0x02, 0x23, 0x07];
//! d.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]); // v1.0
//! d.extend_from_slice(&[0; 8]); // generator, bound
//! d.extend_from_slice(&[0; 4]); // reserved
//! d.extend_from_slice(&[0x11, 0x00, 0x02, 0x00]); // OpMemoryModel
//! d.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]);
//! let p = izanagi_kit::spv::parse(&d).unwrap();
//! assert_eq!(p.version_major, 1);
//! assert_eq!(p.instructions, 1);
//! assert!(izanagi_kit::spv::detect(&d));
//! ```

/// Census of a SPIR-V binary module.
#[derive(Debug, Clone, PartialEq)]
pub struct Spv {
    /// Magic `0x0723_0203` seen (always true on success).
    pub magic_ok: bool,
    /// Version major (`(word >> 16) & 0xFF`).
    pub version_major: u8,
    /// Version minor (`(word >> 8) & 0xFF`).
    pub version_minor: u8,
    /// Generator magic word (tool id / version).
    pub generator: u32,
    /// ID bound — upper limit of result ids.
    pub bound: u32,
    /// Reserved word (schema, must be 0).
    pub reserved: u32,
    /// Instructions walked.
    pub instructions: u32,
    /// Distinct opcodes seen (bucketed count of unique low-16 opcodes).
    pub distinct_opcodes: u32,
    /// `OpEntryPoint` instructions.
    pub entry_points: u32,
    /// `OpName` debug instructions.
    pub names: u32,
    /// `OpCapability` instructions.
    pub capabilities: u32,
    /// `OpExtension`/`OpExtInstImport` instructions.
    pub extensions: u32,
    /// An instruction claimed a word count past the end.
    pub truncated: bool,
}

fn le32(b: &[u8], i: usize) -> u32 {
    b[i] as u32 | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

/// `true` on the SPIR-V magic in either byte order.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 20 && (b[..4] == [0x03, 0x02, 0x23, 0x07] || b[..4] == [0x07, 0x23, 0x02, 0x03])
}

/// Census; `None` without the magic. Handles little-endian layout
/// (the byte order the magic selects in practice).
#[must_use]
pub fn parse(b: &[u8]) -> Option<Spv> {
    if !detect(b) || b[..4] != [0x03, 0x02, 0x23, 0x07] {
        return None;
    }
    let version = le32(b, 4);
    let mut s = Spv {
        magic_ok: true,
        version_major: ((version >> 16) & 0xFF) as u8,
        version_minor: ((version >> 8) & 0xFF) as u8,
        generator: le32(b, 8),
        bound: le32(b, 12),
        reserved: le32(b, 16),
        instructions: 0,
        distinct_opcodes: 0,
        entry_points: 0,
        names: 0,
        capabilities: 0,
        extensions: 0,
        truncated: false,
    };
    // Opcode presence bitset is too large for stack arrays; count
    // distinct opcodes via a small sorted insertion instead.
    let mut seen: Vec<u16> = Vec::new();
    let mut i = 20usize;
    while i + 4 <= b.len() {
        // First word: [ wc:u16 ][ opcode:u16 ] packed as u32 le.
        let first = le32(b, i);
        let opcode = (first & 0xFFFF) as u16;
        let words = (first >> 16) as usize;
        if words == 0 {
            s.truncated = true;
            break;
        }
        let bytes = words * 4;
        if i + bytes > b.len() {
            s.truncated = true;
            break;
        }
        match opcode {
            15 => s.entry_points += 1,    // OpEntryPoint
            5 => s.names += 1,            // OpName
            17 => s.capabilities += 1,    // OpCapability
            10 | 11 => s.extensions += 1, // OpExtension / OpExtInstImport
            _ => {}
        }
        if !seen.contains(&opcode) {
            seen.push(opcode);
        }
        s.instructions += 1;
        i += bytes;
    }
    s.distinct_opcodes = seen.len() as u32;
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0x03, 0x02, 0x23, 0x07];
        d.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]);
        d.extend_from_slice(&[0x0B, 0x00, 0x08, 0x00]); // generator
        d.extend_from_slice(&[0x40, 0x00, 0x00, 0x00]); // bound 64
        d.extend_from_slice(&[0; 4]);
        // OpCapability Shader (opcode 17, wc 2)
        d.extend_from_slice(&[0x11, 0x00, 0x02, 0x00, 1, 0, 0, 0]);
        // OpMemoryModel (opcode 14, wc 3)
        d.extend_from_slice(&[0x0E, 0x00, 0x03, 0x00, 0, 0, 0, 0, 1, 0, 0, 0]);
        // OpEntryPoint (opcode 15, wc 3)
        d.extend_from_slice(&[0x0F, 0x00, 0x03, 0x00, 5, 0, 0, 0, 1, 0, 0, 0]);
        // OpName (opcode 5, wc 3)
        d.extend_from_slice(&[0x05, 0x00, 0x03, 0x00, 1, 0, 0, 0, 0x6D, 0x61, 0x69, 0x6E]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"SPV module"));
        assert!(!detect(&fixture()[..10]));
    }

    #[test]
    fn parses_header_and_instructions() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.version_major, 1);
        assert_eq!(p.version_minor, 0);
        assert_eq!(p.bound, 64);
        assert_eq!(p.instructions, 4);
        assert_eq!(p.entry_points, 1);
        assert_eq!(p.names, 1);
        assert_eq!(p.capabilities, 1);
        assert!(p.distinct_opcodes >= 4);
        assert!(!p.truncated);
    }

    #[test]
    fn truncated_instruction_flagged() {
        let mut d = fixture();
        d.extend_from_slice(&[0x05, 0x00, 0x7F, 0x00]); // claims 127 words
        let p = parse(&d).unwrap();
        assert!(p.truncated);
    }

    #[test]
    fn rejects_non_magic() {
        assert!(parse(b"spir-v text").is_none());
    }
}
