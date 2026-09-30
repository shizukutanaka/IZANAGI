//! SMB2 packet header (MS-SMB2): `\xFESMB` signature + 64-byte fixed
//! header — `structure_size`(must be 64), `credit_charge`, `status`/
//! `channel_seq`, `command`, `credits`, `flags`, `next_command`,
//! `message_id`, `tree_id`/`async_id`, `session_id`, `signature`.
//!
//! ```
//! let mut d = vec![0u8; 64];
//! d[..4].copy_from_slice(b"\xfeSMB");
//! d[4..6].copy_from_slice(&[64, 0]); // structure_size
//! d[12..14].copy_from_slice(&[0, 0]); // NEGOTIATE
//! d[16..20].copy_from_slice(&[0, 0, 0, 0]); // flags: request
//! d[24..32].copy_from_slice(&[7, 0, 0, 0, 0, 0, 0, 0]); // msg id 7
//! let s = izanagi_kit::smb2::parse(&d).unwrap();
//! assert_eq!(s.command, izanagi_kit::smb2::Command::Negotiate);
//! assert_eq!(s.message_id, 7);
//! assert!(!s.is_response);
//! ```

/// SMB2 command (MS-SMB2 §2.2.1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `0` — negotiate dialect
    Negotiate,
    /// `1` — session setup
    SessionSetup,
    /// `2` — logoff
    Logoff,
    /// `3` — tree connect
    TreeConnect,
    /// `4` — tree disconnect
    TreeDisconnect,
    /// `5` — create/open
    Create,
    /// `6` — close
    Close,
    /// `7` — flush
    Flush,
    /// `8` — read
    Read,
    /// `9` — write
    Write,
    /// `10` — lock
    Lock,
    /// `11` — ioctl
    Ioctl,
    /// `12` — cancel
    Cancel,
    /// `13` — echo/keepalive
    Echo,
    /// `14` — query directory
    QueryDirectory,
    /// `15` — change notify
    ChangeNotify,
    /// `16` — query info
    QueryInfo,
    /// `17` — set info
    SetInfo,
    /// `18` — oplock break notification
    OplockBreak,
    /// Any other command word.
    Other(u16),
}

impl Command {
    /// Map the wire command word to a `Command`.
    pub fn from_u16(v: u16) -> Command {
        match v {
            0 => Command::Negotiate,
            1 => Command::SessionSetup,
            2 => Command::Logoff,
            3 => Command::TreeConnect,
            4 => Command::TreeDisconnect,
            5 => Command::Create,
            6 => Command::Close,
            7 => Command::Flush,
            8 => Command::Read,
            9 => Command::Write,
            10 => Command::Lock,
            11 => Command::Ioctl,
            12 => Command::Cancel,
            13 => Command::Echo,
            14 => Command::QueryDirectory,
            15 => Command::ChangeNotify,
            16 => Command::QueryInfo,
            17 => Command::SetInfo,
            18 => Command::OplockBreak,
            other => Command::Other(other),
        }
    }
}

/// A parsed SMB2 packet header.
#[derive(Clone, Debug)]
pub struct Smb2 {
    /// Command word decoded.
    pub command: Command,
    /// `credit_charge` field.
    pub credit_charge: u16,
    /// `status` (in a response) or `channel_seq` (in a request).
    pub status: u32,
    /// `credits` granted/requested.
    pub credits: u16,
    /// `flags` word — `SMB2_FLAGS_SERVER_TO_REDIR` (bit 0) means this
    /// is a response; `SMB2_FLAGS_ASYNC_COMMAND` (bit 1) async.
    pub flags: u32,
    /// `next_command` offset for compounded packets (0 = none).
    pub next_command: u32,
    /// `message_id`.
    pub message_id: u64,
    /// `tree_id` (or `reserved` when async).
    pub tree_id: u32,
    /// `session_id`.
    pub session_id: u64,
    /// 16-byte signature.
    pub signature: [u8; 16],
    /// True when flag bit 0 (`SERVER_TO_REDIR`) is set.
    pub is_response: bool,
    /// Byte offset of the command payload (always 64 for a valid
    /// header; compounding adds bodies after `next_command`).
    pub body_offset: usize,
}

fn u16l(d: &[u8], i: usize) -> Option<u16> {
    Some((u16::from(*d.get(i + 1)?) << 8) | u16::from(*d.get(i)?))
}

fn u32l(d: &[u8], i: usize) -> Option<u32> {
    Some(
        (u32::from(*d.get(i + 3)?) << 24)
            | (u32::from(*d.get(i + 2)?) << 16)
            | (u32::from(*d.get(i + 1)?) << 8)
            | u32::from(*d.get(i)?),
    )
}

fn u64l(d: &[u8], i: usize) -> Option<u64> {
    let lo = u64::from(u32l(d, i)?);
    let hi = u64::from(u32l(d, i + 4)?);
    Some((hi << 32) | lo)
}

/// Parse an SMB2 header: `\xFESMB` + `structure_size == 64` + at
/// least 64 bytes. SMB3 shares this header (the transform header adds
/// a `\xFDSMB` layer outside, which fails the signature check).
pub fn parse(d: &[u8]) -> Option<Smb2> {
    if d.len() < 64 {
        return None;
    }
    if !d.starts_with(b"\xfeSMB") {
        return None;
    }
    if u16l(d, 4)? != 64 {
        return None;
    }
    let flags = u32l(d, 16)?;
    let mut signature = [0u8; 16];
    signature.copy_from_slice(d.get(48..64)?);
    Some(Smb2 {
        command: Command::from_u16(u16l(d, 12)?),
        credit_charge: u16l(d, 6)?,
        status: u32l(d, 8)?,
        credits: u16l(d, 14)?,
        flags,
        next_command: u32l(d, 20)?,
        message_id: u64l(d, 24)?,
        tree_id: u32l(d, 36)?,
        session_id: u64l(d, 40)?,
        signature,
        is_response: flags & 1 != 0,
        body_offset: 64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> Vec<u8> {
        let mut d = vec![0u8; 64];
        d[..4].copy_from_slice(b"\xfeSMB");
        d[4] = 64;
        d
    }

    #[test]
    fn negotiate_request() {
        let mut d = hdr();
        d[12] = 0; // NEGOTIATE
        d[14] = 1; // credits
        d[24] = 5; // message_id
        d[40] = 0x99; // session_id low
        let s = parse(&d).unwrap();
        assert_eq!(s.command, Command::Negotiate);
        assert_eq!(s.message_id, 5);
        assert_eq!(s.session_id, 0x99);
        assert!(!s.is_response);
    }

    #[test]
    fn response_flag() {
        let mut d = hdr();
        d[16] = 1; // SERVER_TO_REDIR
        d[8] = 0x16;
        d[9] = 0x00;
        d[10] = 0x00;
        d[11] = 0xc0; // status 0xC0000016-ish
        let s = parse(&d).unwrap();
        assert!(s.is_response);
        assert_eq!(s.status, 0xc0000016);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut d = hdr();
        d[0] = 0xff; // bad magic
        assert!(parse(&d).is_none());
        d[0] = 0xfe;
        d[4] = 65; // wrong structure_size
        assert!(parse(&d).is_none());
        assert!(parse(&d[..32]).is_none()); // truncated
        assert_eq!(Command::from_u16(18), Command::OplockBreak);
        assert_eq!(Command::from_u16(99), Command::Other(99));
    }
}
