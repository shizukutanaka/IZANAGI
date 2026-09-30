//! ASF (Advanced Systems Format — `.asf`/`.wmv`/`.wma`). Every
//! object is a 16-byte little-endian GUID followed by a `u64le`
//! size; the Header Object opens the file and indexes the rest.
//!
//! ```
//! use izanagi_kit::asf::{detect, parse};
//!
//! // Header GUID + 94B header object with 2 sub-objects.
//! let mut d = Vec::new();
//! d.extend_from_slice(&[
//!     0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, //
//!     0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
//! ]);
//! d.extend_from_slice(&[212, 0, 0, 0, 0, 0, 0, 0]); // object size
//! d.extend_from_slice(&[2, 0, 0, 0]); // object count
//! d.extend_from_slice(&[1, 2]); // reserved
//! // File Properties object (GUID + size 104).
//! d.extend_from_slice(&[
//!     0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, //
//!     0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
//! ]);
//! d.extend_from_slice(&[104, 0, 0, 0, 0, 0, 0, 0]);
//! d.extend_from_slice(&[0u8; 80]); // object payload
//! // Stream Properties object with an audio stream-type GUID.
//! d.extend_from_slice(&[
//!     0x91, 0x07, 0xDC, 0xB7, 0xB7, 0xA9, 0xCF, 0x11, //
//!     0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
//! ]);
//! d.extend_from_slice(&[78, 0, 0, 0, 0, 0, 0, 0]);
//! d.extend_from_slice(&[
//!     0x40, 0x9E, 0x69, 0xF8, 0x4D, 0x5B, 0xCF, 0x11, // audio stream type
//!     0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B,
//! ]);
//! d.extend_from_slice(&[0u8; 38]);
//! assert!(detect(&d));
//! let a = parse(&d).unwrap();
//! assert_eq!(a.objects, 2);
//! assert_eq!(a.audio_streams, 1);
//! ```

fn le64(b: &[u8]) -> u64 {
    let mut v = 0u64;
    for (i, &x) in b.iter().take(8).enumerate() {
        v |= u64::from(x) << (8 * i);
    }
    v
}

const HEADER: [u8; 16] = [
    0x30, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
];
const FILE_PROPS: [u8; 16] = [
    0xA1, 0xDC, 0xAB, 0x8C, 0x47, 0xA9, 0xCF, 0x11, 0x8E, 0xE4, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const STREAM_PROPS: [u8; 16] = [
    0x91, 0x07, 0xDC, 0xB7, 0xB7, 0xA9, 0xCF, 0x11, 0x8E, 0xE6, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const CONTENT_DESC: [u8; 16] = [
    0x33, 0x26, 0xB2, 0x75, 0x8E, 0x66, 0xCF, 0x11, 0xA6, 0xD9, 0x00, 0xAA, 0x00, 0x62, 0xCE, 0x6C,
];
const HEADER_EXT: [u8; 16] = [
    0xB5, 0x03, 0xBF, 0x5F, 0x2E, 0xA9, 0xCF, 0x11, 0x8E, 0xE3, 0x00, 0xC0, 0x0C, 0x20, 0x53, 0x65,
];
const AUDIO_STREAM: [u8; 16] = [
    0x40, 0x9E, 0x69, 0xF8, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B,
];
const VIDEO_STREAM: [u8; 16] = [
    0xC0, 0xEF, 0x19, 0xBC, 0x4D, 0x5B, 0xCF, 0x11, 0xA8, 0xFD, 0x00, 0x80, 0x5F, 0x5C, 0x44, 0x2B,
];

/// Parsed ASF header object.
#[derive(Debug, Clone, PartialEq)]
pub struct Asf {
    /// Declared number of sub-objects in the header object.
    pub declared_objects: u32,
    /// Header object size field (`u64le` at offset 16).
    pub header_size: u64,
    /// Sub-objects actually walked within the header.
    pub objects: u32,
    /// Number of Stream Properties objects.
    pub streams: u32,
    /// Streams whose stream-type GUID is the audio GUID.
    pub audio_streams: u32,
    /// Streams whose stream-type GUID is the video GUID.
    pub video_streams: u32,
    /// A File Properties object was present.
    pub has_file_properties: bool,
    /// A Content Description object was present.
    pub has_content_description: bool,
    /// A Header Extension object was present.
    pub has_header_extension: bool,
    /// Play duration in 100-ns units, when a File Properties object
    /// carried one (offsets 40..48 of its payload).
    pub play_duration_100ns: Option<u64>,
}

/// `true` when the buffer opens with the ASF Header Object GUID.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 30 && b[..16] == HEADER
}

/// Parses the header object; `None` without the GUID or room for it.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Asf> {
    if !detect(b) {
        return None;
    }
    let header_size = le64(&b[16..24]);
    let declared_objects =
        u32::from(b[24]) | u32::from(b[25]) << 8 | u32::from(b[26]) << 16 | u32::from(b[27]) << 24;
    let mut a = Asf {
        declared_objects,
        header_size,
        objects: 0,
        streams: 0,
        audio_streams: 0,
        video_streams: 0,
        has_file_properties: false,
        has_content_description: false,
        has_header_extension: false,
        play_duration_100ns: None,
    };
    let end = usize::try_from(header_size).unwrap_or(b.len()).min(b.len());
    let mut off = 30usize;
    while off + 24 <= end {
        let guid: [u8; 16] = b[off..off + 16].try_into().ok()?;
        let size = match usize::try_from(le64(&b[off + 16..off + 24])) {
            Ok(v) => v,
            Err(_) => break,
        };
        if size < 24 || size > b.len() - off {
            break;
        }
        a.objects += 1;
        let body_end = (off + size).min(b.len());
        if guid == FILE_PROPS {
            a.has_file_properties = true;
            if off + 72 <= body_end {
                a.play_duration_100ns = Some(le64(&b[off + 64..off + 72]));
            }
        } else if guid == STREAM_PROPS {
            a.streams += 1;
            if off + 40 <= body_end {
                let st: [u8; 16] = b[off + 24..off + 40].try_into().ok()?;
                if st == AUDIO_STREAM {
                    a.audio_streams += 1;
                } else if st == VIDEO_STREAM {
                    a.video_streams += 1;
                }
            }
        } else if guid == CONTENT_DESC {
            a.has_content_description = true;
        } else if guid == HEADER_EXT {
            a.has_header_extension = true;
        }
        off += size;
    }
    Some(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&HEADER);
        d.extend_from_slice(&[212, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&[2, 0, 0, 0]);
        d.extend_from_slice(&[1, 2]);
        d.extend_from_slice(&FILE_PROPS);
        d.extend_from_slice(&[104, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&[0u8; 80]);
        d.extend_from_slice(&STREAM_PROPS);
        d.extend_from_slice(&[78, 0, 0, 0, 0, 0, 0, 0]);
        d.extend_from_slice(&AUDIO_STREAM);
        d.extend_from_slice(&[0u8; 38]);
        d
    }

    #[test]
    fn detect_works() {
        assert!(detect(&fixture()));
        assert!(!detect(b"RIFF....WAVE"));
        assert!(!detect(&fixture()[..20]));
    }

    #[test]
    fn parses() {
        let a = parse(&fixture()).unwrap();
        assert_eq!(a.declared_objects, 2);
        assert_eq!(a.header_size, 212);
        assert_eq!(a.objects, 2);
        assert_eq!(a.streams, 1);
        assert_eq!(a.audio_streams, 1);
        assert_eq!(a.video_streams, 0);
        assert!(a.has_file_properties);
        assert!(!a.has_content_description);
        assert!(!a.has_header_extension);
        assert_eq!(a.play_duration_100ns, Some(0));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"short").is_none());
        assert!(parse(b"X.............................").is_none());
    }
}
