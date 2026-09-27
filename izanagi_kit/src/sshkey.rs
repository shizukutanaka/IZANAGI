//! OpenSSH private-key blob (`openssh-key-v1\0` preamble inside the
//! `OPENSSH PRIVATE KEY` PEM): big-endian length-prefixed strings for
//! ciphername, kdfname, kdfoptions, public-key list, and the
//! (possibly encrypted) private section.
//!
//! ```
//! use izanagi_kit::sshkey::parse;
//!
//! let mut d = b"openssh-key-v1\x00".to_vec();
//! for s in [&b"none"[..], &b"none"[..], &b""[..]] {
//!     d.extend_from_slice(&(s.len() as u32).to_be_bytes());
//!     d.extend_from_slice(s);
//! }
//! d.extend_from_slice(&1u32.to_be_bytes()); // one public key
//! let pk = &[0, 0, 0, 7, b's', b's', b'h', b'-', b'r', b's', b'a'];
//! d.extend_from_slice(&(pk.len() as u32).to_be_bytes());
//! d.extend_from_slice(pk);
//! d.extend_from_slice(&[0xAA; 8]); // private section
//! let k = parse(&d).unwrap();
//! assert_eq!(k.cipher, "none");
//! assert_eq!(k.nkeys, 1);
//! assert!(k.is_encrypted() == false);
//! ```

use std::string::String;
use std::vec::Vec;

/// `openssh-key-v1\0` preamble.
pub const MAGIC: &[u8] = b"openssh-key-v1\x00";

fn be32(d: &[u8], at: usize) -> Option<u32> {
    let b = d.get(at..at + 4)?;
    Some(u32::from(b[0]) << 24 | u32::from(b[1]) << 16 | u32::from(b[2]) << 8 | u32::from(b[3]))
}

fn string(d: &[u8], at: usize) -> Option<(&[u8], usize)> {
    let n = be32(d, at)? as usize;
    let start = at + 4;
    let end = start.checked_add(n)?;
    Some((d.get(start..end)?, end))
}

/// Parsed `openssh-key-v1` outer fields (private section left opaque).
#[derive(Debug, Clone)]
pub struct SshKey {
    /// `ciphername` (`"none"` when unencrypted).
    pub cipher: String,
    /// `kdfname` (`"none"` / `"bcrypt"`).
    pub kdf: String,
    /// Raw `kdfoptions` string bytes.
    pub kdf_options: Vec<u8>,
    /// Number of public keys embedded.
    pub nkeys: u32,
    /// Public-key blob(s), one `string` each.
    pub public_keys: Vec<Vec<u8>>,
    /// Byte offset of the encrypted/plaintext private section.
    pub private_at: usize,
}

/// Parse the blob; `None` on wrong magic or truncation.
pub fn parse(d: &[u8]) -> Option<SshKey> {
    let rest = d.strip_prefix(MAGIC)?;
    let mut at = d.len() - rest.len();
    let (cipher, n) = string(d, at)?;
    at = n;
    let (kdf, n) = string(d, at)?;
    at = n;
    let (kdf_options, n) = string(d, at)?;
    at = n;
    let nkeys = be32(d, at)?;
    at += 4;
    let mut public_keys = Vec::new();
    for _ in 0..nkeys {
        let (pk, n) = string(d, at)?;
        public_keys.push(pk.to_vec());
        at = n;
    }
    if at >= d.len() {
        return None;
    }
    Some(SshKey {
        cipher: String::from_utf8_lossy(cipher).into_owned(),
        kdf: String::from_utf8_lossy(kdf).into_owned(),
        kdf_options: kdf_options.to_vec(),
        nkeys,
        public_keys,
        private_at: at,
    })
}

impl SshKey {
    /// `cipher != "none"` — the private section is still encrypted.
    pub fn is_encrypted(&self) -> bool {
        self.cipher != "none"
    }

    /// Private-section bytes (decrypt per `cipher`/`kdf` externally).
    pub fn private_section<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(self.private_at..)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(cipher: &str) -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        for s in [cipher.as_bytes(), b"none".as_slice(), b"".as_slice()] {
            d.extend_from_slice(&(s.len() as u32).to_be_bytes());
            d.extend_from_slice(s);
        }
        d.extend_from_slice(&1u32.to_be_bytes());
        let pk = b"\x00\x00\x00\x07ssh-rsa";
        d.extend_from_slice(&(pk.len() as u32).to_be_bytes());
        d.extend_from_slice(pk);
        d.extend_from_slice(&[0xAA; 16]); // private section
        d
    }

    #[test]
    fn fields() {
        let d = fixture("none");
        let k = parse(&d).unwrap();
        assert_eq!(k.cipher, "none");
        assert!(!k.is_encrypted());
        assert_eq!(k.nkeys, 1);
        assert_eq!(k.public_keys[0].len(), 11);
        assert_eq!(k.private_section(&d).unwrap().len(), 16);

        let enc = parse(&fixture("aes256-ctr")).unwrap();
        assert!(enc.is_encrypted());
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"openssh-key-v1\x00").is_none()); // no ciphername
                                                         // truncation inside a string field fails
        let mut d = fixture("none");
        d.truncate(MAGIC.len() + 2);
        assert!(parse(&d).is_none());
    }
}
