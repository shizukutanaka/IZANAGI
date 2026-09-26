//! DirectDraw Surface (`.dds`) — Microsoft's texture container for
//! DirectX: a `'DDS '` DWORD magic, the 124-byte `DDS_HEADER`, a
//! 32-byte `DDS_PIXELFORMAT` inside it, an optional 20-byte
//! `DDS_HEADER_DXT10` (present iff the pixel format's FourCC reads
//! `DX10`), then the mip chain. All integers are little-endian.
//!
//! [`parse`] validates the two magic/`dwSize` fields the spec pins
//! (header 124, pixel format 32), extracts geometry + mip count +
//! cubemap flags, and locates the first mip level's data. Block
//! compression format names (`DXT1`/`BC4`…) surface through
//! [`PixelFormat::fourcc`]; DXGI formats through
//! [`Dds::dx10`].
//!
//! ```
//! use izanagi_kit::dds::parse;
//!
//! // minimal uncompressed 4x4 RGB texture, mips=1
//! let mut d = b"DDS ".to_vec();
//! let mut h = [0u8; 124];
//! h[..4].copy_from_slice(&124u32.to_le_bytes()); // dwSize
//! h[4..8].copy_from_slice(&0x2100Fu32.to_le_bytes()); // CAPS|HEIGHT|WIDTH|PITCH|PIXELFORMAT
//! h[8..12].copy_from_slice(&4u32.to_le_bytes()); // height
//! h[12..16].copy_from_slice(&4u32.to_le_bytes()); // width
//! h[16..20].copy_from_slice(&16u32.to_le_bytes()); // pitch
//! h[24..28].copy_from_slice(&1u32.to_le_bytes()); // mipmaps
//! h[72..76].copy_from_slice(&32u32.to_le_bytes()); // ddspf.dwSize
//! h[76..80].copy_from_slice(&0x41u32.to_le_bytes()); // DDPF_RGB|ALPHAPIXELS
//! h[84..88].copy_from_slice(&32u32.to_le_bytes()); // RGBBitCount
//! h[88..92].copy_from_slice(&0x00FF0000u32.to_le_bytes()); // R mask
//! h[92..96].copy_from_slice(&0x0000FF00u32.to_le_bytes());
//! h[96..100].copy_from_slice(&0x000000FFu32.to_le_bytes());
//! h[100..104].copy_from_slice(&0xFF000000u32.to_le_bytes());
//! h[104..108].copy_from_slice(&0x1000u32.to_le_bytes()); // DDSCAPS_TEXTURE
//! d.extend_from_slice(&h);
//! d.extend_from_slice(&[0xAB; 64]);
//! let t = parse(&d).unwrap();
//! assert_eq!((t.width, t.height, t.mipmaps), (4, 4, 1));
//! assert!(!t.is_compressed());
//! assert_eq!(t.data(&d).unwrap().len(), 64);
//! ```

/// `DDPF_ALPHAPIXELS` — pixel format has an alpha channel.
pub const DDPF_ALPHAPIXELS: u32 = 0x1;
/// `DDPF_FOURCC` — `dwFourCC` holds a compression code.
pub const DDPF_FOURCC: u32 = 0x4;
/// `DDPF_RGB` — uncompressed RGB(A) data follows.
pub const DDPF_RGB: u32 = 0x40;
/// `DDPF_LUMINANCE` — luminance channel data.
pub const DDPF_LUMINANCE: u32 = 0x2_0000;
/// `DDSCAPS2_CUBEMAP` — the surface is a cubemap.
pub const DDSCAPS2_CUBEMAP: u32 = 0x200;
/// `DDSCAPS2_VOLUME` — the surface is a volume texture.
pub const DDSCAPS2_VOLUME: u32 = 0x20_0000;

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let b = d.get(at..at + 4)?;
    Some(u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16 | u32::from(b[3]) << 24)
}

/// The `DDS_PIXELFORMAT` sub-structure (offset 76 inside the header).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelFormat {
    /// Raw `dwFlags` (`DDPF_*`).
    pub flags: u32,
    /// `dwFourCC` bytes — `Some` iff `DDPF_FOURCC` is set.
    pub fourcc: Option<[u8; 4]>,
    /// `dwRGBBitCount` for uncompressed formats.
    pub rgb_bits: u32,
    /// Channel bit masks `[R, G, B, A]` for uncompressed formats.
    pub masks: [u32; 4],
}

/// `DDS_HEADER_DXT10` — appended when FourCC is `DX10` (Direct3D 10+).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dx10Header {
    /// `dxgiFormat` (a `DXGI_FORMAT` value).
    pub dxgi_format: u32,
    /// `resourceDimension` (a `D3D10_RESOURCE_DIMENSION` value).
    pub resource_dim: u32,
    /// `miscFlag` — bit 2 (0x4) marks a cubemap.
    pub misc: u32,
    /// `arraySize` — texture array element count.
    pub array_size: u32,
    /// `miscFlags2` — alpha mode in the low 4 bits.
    pub misc2: u32,
}

/// A parsed `.dds` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Dds {
    /// `dwWidth`.
    pub width: u32,
    /// `dwHeight`.
    pub height: u32,
    /// `dwDepth` (volumes only; 0/1 otherwise).
    pub depth: u32,
    /// `dwMipMapCount`.
    pub mipmaps: u32,
    /// `dwPitchOrLinearSize` — row pitch or top-level byte size.
    pub pitch_or_linear: u32,
    /// `dwCaps` + `dwCaps2` raw values.
    pub caps: u32,
    /// `dwCaps2`.
    pub caps2: u32,
    /// The pixel format sub-structure.
    pub pf: PixelFormat,
    /// The DX10 extension header, when the FourCC requests it.
    pub dx10: Option<Dx10Header>,
    /// Byte offset where the mip data starts (128 or 148).
    pub data_at: usize,
}

/// Parse the header. `None` on short input, bad magic, or a
/// `dwSize`/`ddspf.dwSize` that doesn't match the spec constants
/// (124 and 32). The mip payload itself is not required — query it
/// with [`Dds::data`].
pub fn parse(d: &[u8]) -> Option<Dds> {
    if d.len() < 128 || &d[..4] != b"DDS " {
        return None;
    }
    if u32le(d, 4)? != 124 {
        return None;
    }
    let height = u32le(d, 12)?;
    let width = u32le(d, 16)?;
    let pitch_or_linear = u32le(d, 20)?;
    let depth = u32le(d, 24)?;
    let mipmaps = u32le(d, 28)?;
    // DDS_PIXELFORMAT at header offset 72: dwSize@76, dwFlags@80,
    // dwFourCC@84, dwRGBBitCount@88, RGBA masks @92..104.
    let pf_at = 4 + 72;
    if u32le(d, pf_at)? != 32 {
        return None;
    }
    let flags = u32le(d, pf_at + 4)?;
    let fourcc_raw = d.get(pf_at + 8..pf_at + 12)?;
    let fourcc = if flags & DDPF_FOURCC != 0 {
        let mut f = [0u8; 4];
        f.copy_from_slice(fourcc_raw);
        Some(f)
    } else {
        None
    };
    let pf = PixelFormat {
        flags,
        fourcc,
        rgb_bits: u32le(d, pf_at + 12)?,
        masks: [
            u32le(d, pf_at + 16)?,
            u32le(d, pf_at + 20)?,
            u32le(d, pf_at + 24)?,
            u32le(d, pf_at + 28)?,
        ],
    };
    let caps = u32le(d, 108)?;
    let caps2 = u32le(d, 112)?;
    let mut data_at = 128usize;
    let dx10 = if fourcc == Some(*b"DX10") {
        if d.len() < 148 {
            return None;
        }
        data_at = 148;
        Some(Dx10Header {
            dxgi_format: u32le(d, 128)?,
            resource_dim: u32le(d, 132)?,
            misc: u32le(d, 136)?,
            array_size: u32le(d, 140)?,
            misc2: u32le(d, 144)?,
        })
    } else {
        None
    };
    Some(Dds {
        width,
        height,
        depth,
        mipmaps,
        pitch_or_linear,
        caps,
        caps2,
        pf,
        dx10,
        data_at,
    })
}

impl Dds {
    /// True when the pixel format declares a FourCC (compressed or
    /// DX10-extended surface).
    pub fn is_compressed(&self) -> bool {
        self.pf.fourcc.is_some()
    }

    /// Cubemap bit of `dwCaps2` — or the DX10 `misc` cubemap flag.
    pub fn is_cubemap(&self) -> bool {
        self.caps2 & DDSCAPS2_CUBEMAP != 0 || self.dx10.is_some_and(|x| x.misc & 0x4 != 0)
    }

    /// Volume-texture bit of `dwCaps2`.
    pub fn is_volume(&self) -> bool {
        self.caps2 & DDSCAPS2_VOLUME != 0
    }

    /// The mip/surface payload following the header(s).
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(self.data_at..)
    }

    /// Block width/height for the common BC FourCCs — `DXT1`/`BC4`/
    /// `ATI1` are 4x4 blocks of 8 bytes, `DXT3`/`DXT5`/`BC5`/`ATI2`
    /// are 4x4 blocks of 16 bytes. `None` for other formats.
    pub fn block(&self) -> Option<(u32, u32)> {
        match self.pf.fourcc.as_ref()? {
            b"DXT1" | b"BC4 " | b"ATI1" => Some((4, 8)),
            b"DXT3" | b"DXT5" | b"BC5 " | b"ATI2" => Some((4, 16)),
            _ => None,
        }
    }

    /// Byte size of mip level 0 — block-linear for the recognized
    /// BC formats, `width * height * (rgb_bits / 8)` otherwise.
    pub fn top_mip_size(&self) -> Option<u64> {
        if let Some((block_px, block_bytes)) = self.block() {
            let bw = u64::from(self.width).div_ceil(u64::from(block_px));
            let bh = u64::from(self.height).div_ceil(u64::from(block_px));
            Some(bw * bh * u64::from(block_bytes))
        } else {
            if self.pf.rgb_bits == 0 || self.pf.rgb_bits % 8 != 0 {
                return None;
            }
            Some(u64::from(self.width) * u64::from(self.height) * u64::from(self.pf.rgb_bits / 8))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(pf_flags: u32, fourcc: [u8; 4], rgb_bits: u32) -> Vec<u8> {
        let mut d = b"DDS ".to_vec();
        let mut h = [0u8; 124];
        h[..4].copy_from_slice(&124u32.to_le_bytes());
        h[4..8].copy_from_slice(&0x2100Fu32.to_le_bytes());
        h[8..12].copy_from_slice(&8u32.to_le_bytes()); // height
        h[12..16].copy_from_slice(&16u32.to_le_bytes()); // width
        h[24..28].copy_from_slice(&3u32.to_le_bytes()); // mips
        h[72..76].copy_from_slice(&32u32.to_le_bytes());
        h[76..80].copy_from_slice(&pf_flags.to_le_bytes());
        h[80..84].copy_from_slice(&fourcc);
        h[84..88].copy_from_slice(&rgb_bits.to_le_bytes());
        h[104..108].copy_from_slice(&0x1000u32.to_le_bytes());
        d.extend_from_slice(&h);
        d
    }

    #[test]
    fn fourcc_dxt1() {
        let mut d = hdr(DDPF_FOURCC, *b"DXT1", 0);
        d.extend_from_slice(&[0; 512]);
        let t = parse(&d).unwrap();
        assert_eq!((t.width, t.height, t.mipmaps), (16, 8, 3));
        assert!(t.is_compressed());
        assert!(!t.is_cubemap());
        assert_eq!(t.block(), Some((4, 8)));
        assert_eq!(t.top_mip_size(), Some(4 * 2 * 8)); // 16x8 @ 4x4x8B blocks
        assert_eq!(t.data(&d).unwrap().len(), 512);
    }

    #[test]
    fn dx10_extension() {
        let mut d = hdr(DDPF_FOURCC, *b"DX10", 0);
        for v in [71u32 /*DXGI_FORMAT_R8G8B8A8_UNORM*/, 3, 0x4, 6, 0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        let t = parse(&d).unwrap();
        let x = t.dx10.unwrap();
        assert_eq!(x.dxgi_format, 71);
        assert_eq!(x.array_size, 6);
        assert!(t.is_cubemap()); // misc flag bit 2
        assert_eq!(t.data_at, 148);
        assert_eq!(t.data(&d), Some(&[][..]));
        // truncated dx10 header
        let short = &hdr(DDPF_FOURCC, *b"DX10", 0)[..];
        assert!(parse(short).is_none());
    }

    #[test]
    fn uncompressed_masks() {
        let mut d = hdr(DDPF_RGB | DDPF_ALPHAPIXELS, [0; 4], 24);
        d.extend_from_slice(&[7; 16 * 8 * 3]);
        let t = parse(&d).unwrap();
        assert!(!t.is_compressed());
        assert_eq!(t.top_mip_size(), Some(16 * 8 * 3));
        assert_eq!(t.data(&d).unwrap()[0], 7);
    }

    #[test]
    fn volume_flag() {
        let mut d = hdr(DDPF_FOURCC, *b"DXT5", 0);
        // dwCaps2 at file offset 4 + 108.
        let caps2 = DDSCAPS2_VOLUME | 0xFE00;
        d[112..116].copy_from_slice(&caps2.to_le_bytes());
        let t = parse(&d).unwrap();
        assert!(t.is_volume());
        assert_eq!(t.block(), Some((4, 16)));
    }

    #[test]
    fn malformed() {
        assert!(parse(b"").is_none());
        assert!(parse(b"DDS!").is_none());
        // dwSize != 124
        let mut d = hdr(DDPF_RGB, [0; 4], 32);
        d[4] = 120;
        assert!(parse(&d).is_none());
        // ddspf.dwSize != 32
        let mut d2 = hdr(DDPF_RGB, [0; 4], 32);
        d2[76] = 0;
        assert!(parse(&d2).is_none());
    }
}
