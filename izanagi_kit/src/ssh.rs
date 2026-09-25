//! OpenSSH public-key wire format (RFC 4253 §6.6 + `authorized_keys`
//! layout): length-prefixed `string`/`mpint` fields, base64 key blobs,
//! and the `SHA256:` fingerprint every `ssh-keygen -l` prints. Parsers
//! are total; a truncated blob or mismatched algorithm yields `None`.
//!
//! `pem` covers the ASCII armor layer; `ssh` covers the key blob itself.
//!
//! ```
//! use izanagi_kit::ssh;
//! // ssh-ed25519 blob: string "ssh-ed25519" + string <32-byte key>
//! let mut blob = Vec::new();
//! blob.extend_from_slice(&[0, 0, 0, 11]);
//! blob.extend_from_slice(b"ssh-ed25519");
//! blob.extend_from_slice(&[0, 0, 0, 32]);
//! blob.extend_from_slice(&[7u8; 32]);
//! let f = ssh::fields(&blob).unwrap();
//! assert!(f.len() == 2 && f[1] == vec![7u8; 32]);
//! ```

use crate::base64;
use crate::sha256::sha256;

/// Read one SSH `string` (4-byte big-endian length + payload);
/// returns `(payload, next offset)`, `None` on truncation.
pub fn string(d: &[u8], at: usize) -> Option<(&[u8], usize)> {
    let n = u32::try_from(
        d.get(at..at + 4)?
            .iter()
            .fold(0u64, |a, &b| (a << 8) | b as u64),
    )
    .ok()? as usize;
    let end = at.checked_add(4)?.checked_add(n)?;
    Some((d.get(at + 4..end)?, end))
}

/// Every length-prefixed field of a blob, in order (field 0 is the
/// algorithm name). `None` if any field overruns the buffer or bytes
/// trail the last field.
pub fn fields(blob: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < blob.len() {
        let (f, next) = string(blob, at)?;
        out.push(f.to_vec());
        at = next;
    }
    Some(out)
}

/// Build a wire blob from fields (`string` each).
pub fn blob(fs: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for f in fs {
        out.extend_from_slice(&[
            (f.len() >> 24) as u8,
            (f.len() >> 16) as u8,
            (f.len() >> 8) as u8,
            f.len() as u8,
        ]);
        out.extend_from_slice(f);
    }
    out
}

/// One authorized key: algorithm + blob + optional comment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PubKey {
    /// `ssh-ed25519`, `ssh-rsa`, `ecdsa-sha2-nistp256`, …
    pub algo: String,
    /// Raw wire blob (what the base64 decoded).
    pub blob: Vec<u8>,
    /// Trailing comment (may be empty).
    pub comment: String,
}

fn is_algo_token(t: &str) -> bool {
    t.starts_with("ssh-") || t.starts_with("ecdsa-") || t.starts_with("sk-")
}

/// Parse one `authorized_keys` line: `[options] algo base64 [comment]`.
/// Options before the algorithm are skipped. `None` when the base64
/// fails, the blob's first field disagrees with the algorithm token,
/// or no algorithm token exists.
pub fn parse_line(line: &str) -> Option<PubKey> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    let i = toks.iter().position(|t| is_algo_token(t))?;
    let algo = toks[i];
    let b64 = toks.get(i + 1)?;
    let blob = base64::decode(b64)?;
    let fs = fields(&blob)?;
    if fs.first().map(|v| v.as_slice()) != Some(algo.as_bytes()) {
        return None;
    }
    Some(PubKey {
        algo: algo.to_string(),
        blob,
        comment: toks[i + 2..].join(" "),
    })
}

/// Parse a whole `authorized_keys` file; blank lines and `#` comments
/// are skipped, malformed lines dropped.
pub fn authorized_keys(text: &str) -> Vec<PubKey> {
    text.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .filter_map(parse_line)
        .collect()
}

/// OpenSSH fingerprint: `SHA256:` + base64 of `sha256(blob)` with the
/// `=` padding stripped (the `ssh-keygen -l` form).
pub fn fingerprint(k: &PubKey) -> String {
    let mut s = String::from("SHA256:");
    s.push_str(base64::encode(&sha256(&k.blob)).trim_end_matches('='));
    s
}

/// Emit the authorized_keys line (`algo base64 comment`).
pub fn emit_line(k: &PubKey) -> String {
    let mut s = format!("{} {}", k.algo, base64::encode(&k.blob));
    if !k.comment.is_empty() {
        s.push(' ');
        s.push_str(&k.comment);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed25519_blob() -> Vec<u8> {
        blob(&[b"ssh-ed25519", &[0xAB; 32]])
    }

    #[test]
    fn string_walk() {
        let b = blob(&[b"algo", b"field2", b""]);
        assert_eq!(
            fields(&b).unwrap(),
            vec![b"algo".to_vec(), b"field2".to_vec(), Vec::new()]
        );
        assert!(fields(&b[..b.len() - 1]).is_none());
        assert!(fields(&[0, 0, 0, 9, b'x']).is_none()); // overruns
        assert_eq!(fields(&[]), Some(vec![]));
    }

    #[test]
    fn authorized_keys_flow() {
        let key = PubKey {
            algo: "ssh-ed25519".into(),
            blob: ed25519_blob(),
            comment: "dev@box".into(),
        };
        let line = emit_line(&key);
        let k2 = parse_line(&line).unwrap();
        assert_eq!(k2, key);
        // with options + comments
        let line2 = format!("restrict,pty {}", line);
        assert_eq!(parse_line(&line2).unwrap().algo, "ssh-ed25519");
        let file = format!("# comment\n\n{}\nbogus line\n", line);
        assert_eq!(authorized_keys(&file).len(), 1);
        // algo/blob mismatch → None
        let bad = format!("ssh-rsa {}", base64::encode(&ed25519_blob()));
        assert!(parse_line(&bad).is_none());
        assert!(parse_line("ssh-ed25519 !!!").is_none());
        assert!(parse_line("no-key-here").is_none());
    }

    #[test]
    fn fingerprint_shape() {
        // real ed25519 test key blob (RFC 8032 test seed pubkey)
        let mut raw = [0u8; 32];
        for (i, v) in raw.iter_mut().enumerate() {
            *v = i as u8;
        }
        let k = PubKey {
            algo: "ssh-ed25519".into(),
            blob: blob(&[b"ssh-ed25519", &raw]),
            comment: String::new(),
        };
        let f = fingerprint(&k);
        assert!(f.starts_with("SHA256:"));
        assert_eq!(f.len(), 7 + 43); // unpadded b64 of 32 bytes
        assert!(!f.contains('='));
        // determinism
        assert_eq!(fingerprint(&k), f);
    }
}
