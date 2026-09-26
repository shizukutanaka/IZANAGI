//! QCOW2/QCOW3 — QEMU's copy-on-write disk image. The header is
//! a fixed little struct (magic `QFI\xFB`, version 2/3) in
//! *big-endian*, followed by refcount/L1/snapshot tables this
//! module exposes by offset. Feature bits land only in v3.
//!
//! ```
//! use izanagi_kit::qcow2;
//! let mut h = vec![0u8; 104];
//! h[..4].copy_from_slice(b"QFI\xFB");
//! h[4..8].copy_from_slice(&3u32.to_be_bytes());   // version 3
//! h[20..24].copy_from_slice(&16u32.to_be_bytes()); // cluster_bits
//! h[24..32].copy_from_slice(&(1u64 << 32).to_be_bytes()); // size
//! h[36..40].copy_from_slice(&4u32.to_be_bytes());  // l1_size
//! h[40..48].copy_from_slice(&104u64.to_be_bytes()); // l1 offset
//! h.resize(104 + 32, 0);                             // room for L1
//! h[96..100].copy_from_slice(&4u32.to_be_bytes()); // refcount order
//! h[100..104].copy_from_slice(&104u32.to_be_bytes()); // header len
//! let q = qcow2::parse(&h).unwrap();
//! assert_eq!(q.version, 3);
//! assert_eq!(q.cluster_size(), 65536);
//! ```

fn rb32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        ((*d.get(at)? as u32) << 24)
            | ((*d.get(at + 1)? as u32) << 16)
            | ((*d.get(at + 2)? as u32) << 8)
            | *d.get(at + 3)? as u32,
    )
}
fn rb64(d: &[u8], at: usize) -> Option<u64> {
    Some((rb32(d, at)? as u64) << 32 | rb32(d, at + 4)? as u64)
}

/// Encryption method field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crypt {
    /// 0 — none.
    None,
    /// 1 — legacy AES (insecure, kept for reading).
    Aes,
    /// 2 — LUKS (payload is a LUKS container).
    Luks,
    /// Anything else.
    Other(u32),
}

/// Parsed header surface.
#[derive(Clone, Debug)]
pub struct Qcow {
    /// 2 or 3 (the "QCOW3" marketing name).
    pub version: u32,
    /// Backing-file path bytes when `backing_offset != 0`.
    pub backing: Option<Vec<u8>>,
    /// `1 << cluster_bits` bytes per cluster (bits 9..=21 valid).
    pub cluster_bits: u32,
    /// Virtual disk size in bytes.
    pub size: u64,
    /// L1 table entry count.
    pub l1_size: u32,
    /// L1 table file offset.
    pub l1_offset: u64,
    /// Encryption.
    pub crypt: Crypt,
    /// Refcount table offset.
    pub refcount_table_offset: u64,
    /// Refcount table cluster count.
    pub refcount_blocks: u32,
    /// Snapshot count.
    pub nb_snapshots: u32,
    /// Snapshot table offset.
    pub snapshots_offset: u64,
    /// v3 feature masks (0 on v2): incompatible.
    pub incompat: u64,
    /// Compatible.
    pub compat: u64,
    /// Auto-clear.
    pub autoclear: u64,
    /// Refcount entry order (v3; `4` when absent).
    pub refcount_order: u32,
    /// Declared header length (v3).
    pub header_len: u32,
}

impl Qcow {
    /// Bytes per cluster.
    pub fn cluster_size(&self) -> u64 {
        1u64 << self.cluster_bits
    }
    /// L2 entries per cluster (all entries are 8 bytes).
    pub fn l2_entries(&self) -> u64 {
        self.cluster_size() / 8
    }
    /// L1 entries needed to cover [`Qcow::size`] — i.e. how much
    /// of the L1 table is live.
    pub fn l1_needed(&self) -> u64 {
        let l2 = self.l2_entries();
        let clusters = self.size.div_ceil(self.cluster_size());
        clusters.div_ceil(l2)
    }
    /// `true` when bit `b` of the incompatible mask is set.
    pub fn incompat(&self, b: u32) -> bool {
        self.incompat & (1u64 << b) != 0
    }
}

/// Parses the header. Feature fields default to 0 on version 2;
/// unknown versions reject.
pub fn parse(d: &[u8]) -> Option<Qcow> {
    if d.get(0..4)? != b"QFI\xFB" {
        return None;
    }
    let version = rb32(d, 4)?;
    if !(2..=3).contains(&version) {
        return None;
    }
    let backing_offset = rb64(d, 8)?;
    let backing_size = rb32(d, 16)? as u64;
    let cluster_bits = rb32(d, 20)?;
    if !(9..=21).contains(&cluster_bits) {
        return None;
    }
    let size = rb64(d, 24)?;
    let crypt = match rb32(d, 32)? {
        0 => Crypt::None,
        1 => Crypt::Aes,
        2 => Crypt::Luks,
        c => Crypt::Other(c),
    };
    let backing = if backing_offset != 0 {
        if backing_offset.checked_add(backing_size)? > d.len() as u64 {
            return None;
        }
        Some(
            d.get(backing_offset as usize..(backing_offset + backing_size) as usize)?
                .to_vec(),
        )
    } else {
        None
    };
    let mut q = Qcow {
        version,
        backing,
        cluster_bits,
        size,
        l1_size: rb32(d, 36)?,
        l1_offset: rb64(d, 40)?,
        crypt,
        refcount_table_offset: rb64(d, 48)?,
        refcount_blocks: rb32(d, 56)?,
        nb_snapshots: rb32(d, 60)?,
        snapshots_offset: rb64(d, 64)?,
        incompat: 0,
        compat: 0,
        autoclear: 0,
        refcount_order: 4,
        header_len: 72,
    };
    if version == 3 {
        q.incompat = rb64(d, 72)?;
        q.compat = rb64(d, 80)?;
        q.autoclear = rb64(d, 88)?;
        q.refcount_order = rb32(d, 96)?;
        q.header_len = rb32(d, 100)?;
        if q.header_len < 104 || q.header_len as usize > d.len() {
            return None;
        }
    }
    // L1 table must fit in the image when present
    let l1_end = (q.l1_offset as usize).checked_add(8 * q.l1_size as usize)?;
    if l1_end > d.len() {
        return None;
    }
    Some(q)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be32(v: u32) -> [u8; 4] {
        [(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]
    }
    fn be64(v: u64) -> [u8; 8] {
        [
            (v >> 56) as u8,
            (v >> 48) as u8,
            (v >> 40) as u8,
            (v >> 32) as u8,
            (v >> 24) as u8,
            (v >> 16) as u8,
            (v >> 8) as u8,
            v as u8,
        ]
    }

    fn hdr(v: u32) -> Vec<u8> {
        let mut h = vec![0u8; 104];
        h[..4].copy_from_slice(b"QFI\xFB");
        h[4..8].copy_from_slice(&be32(v));
        h[20..24].copy_from_slice(&be32(16));
        h[24..32].copy_from_slice(&be64(1 << 30)); // 1 GiB
        h[36..40].copy_from_slice(&be32(4));
        h[40..48].copy_from_slice(&be64(200));
        h[100..104].copy_from_slice(&be32(104));
        h.resize(200 + 8 * 4, 0); // L1 fits
        h
    }

    #[test]
    fn parses_v2_and_v3() {
        let q2 = parse(&hdr(2)).unwrap();
        assert_eq!(q2.version, 2);
        assert_eq!(q2.cluster_size(), 65536);
        assert_eq!(q2.refcount_order, 4);
        let q3 = parse(&hdr(3)).unwrap();
        assert_eq!(q3.version, 3);
        assert_eq!(q3.l2_entries(), 8192);
        // 1 GiB / 64 KiB = 16384 clusters / 8192 per L2 → 2 L1 entries
        assert_eq!(q3.l1_needed(), 2);
    }

    #[test]
    fn backing_path_and_features() {
        let mut h = hdr(3);
        h[8..16].copy_from_slice(&be64(300));
        h[16..20].copy_from_slice(&be32(9));
        h.resize(309, 0);
        h[300..309].copy_from_slice(b"base.qcow");
        h[72..80].copy_from_slice(&be64(1)); // dirty bit incompat
        let q = parse(&h).unwrap();
        assert_eq!(q.backing.as_deref(), Some(b"base.qcow".as_ref()));
        assert!(q.incompat(0));
        assert!(!q.incompat(1));
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        let mut h = hdr(2);
        h[0] = b'X';
        assert!(parse(&h).is_none());
        let h2 = hdr(4);
        assert!(parse(&h2).is_none());
        let mut h3 = hdr(2);
        h3[20..24].copy_from_slice(&be32(40)); // cluster_bits too big
        assert!(parse(&h3).is_none());
        let mut h4 = hdr(2);
        h4[40..48].copy_from_slice(&be64(u64::MAX)); // L1 way past EOF
        assert!(parse(&h4).is_none());
    }
}
