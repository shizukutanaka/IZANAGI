//! LDAP (RFC 4511) — `SEQUENCE { messageID INTEGER, protocolOp [APPLICATION n], controls [0] }`.
//!
//! Built on [`crate::der`]; the protocol-op TLV carries the LDAP operation
//! number in its application-class tag (BindRequest = 0, SearchRequest = 3, …).
//!
//! ```
//! // SEQUENCE { INTEGER 7, [APPLICATION 3] "dc=x" }
//! let d = [0x30, 0x09, 0x02, 0x01, 0x07, 0x63, 0x04, 0x64, 0x63, 0x3d, 0x78];
//! let l = izanagi_kit::ldap::parse(&d).unwrap();
//! assert_eq!(l.msg_id, 7);
//! assert_eq!(l.op_name(), "searchRequest");
//! assert!(!l.has_controls);
//! ```

/// Parsed LDAP message envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ldap {
    /// `messageID` (INTEGER).
    pub msg_id: u32,
    /// Application-class tag number of `protocolOp` (0 bind … 25 intermediateResponse).
    pub op: u8,
    /// True when the optional `controls [0]` field is present.
    pub has_controls: bool,
    /// Raw protocolOp content octets.
    pub op_content: Vec<u8>,
}

impl Ldap {
    /// RFC 4511 name of the operation tag.
    pub fn op_name(&self) -> &'static str {
        match self.op {
            0 => "bindRequest",
            1 => "bindResponse",
            2 => "unbindRequest",
            3 => "searchRequest",
            4 => "searchResEntry",
            5 => "searchResDone",
            6 => "modifyRequest",
            7 => "modifyResponse",
            8 => "addRequest",
            9 => "addResponse",
            10 => "delRequest",
            11 => "delResponse",
            12 => "modDNRequest",
            13 => "modDNResponse",
            14 => "compareRequest",
            15 => "compareResponse",
            16 => "abandonRequest",
            19 => "searchResRef",
            23 => "extendedRequest",
            24 => "extendedResponse",
            25 => "intermediateResponse",
            _ => "unknown",
        }
    }
}

/// Parse one LDAP message; `None` on non-DER input or a missing application op.
pub fn parse(d: &[u8]) -> Option<Ldap> {
    let top = crate::der::parse(d)?;
    let seq = top.first()?;
    if seq.cls != 0 || seq.tag != 16 || !seq.constructed {
        return None;
    }
    let kids = seq.children()?;
    let msg_id = kids.first()?.integer()? as u32;
    let op_tlv = kids.get(1)?;
    if op_tlv.cls != 1 || op_tlv.tag > 25 {
        return None;
    }
    Some(Ldap {
        msg_id,
        op: op_tlv.tag as u8,
        has_controls: kids.iter().skip(2).any(|t| t.cls == 2 && t.tag == 0),
        op_content: op_tlv.content.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_request() {
        // SEQUENCE { 02 01 01, 60 07 { 02 01 03 04 00 80 00 } , 80 00 controls? }
        let d = [
            0x30, 0x0C, 0x02, 0x01, 0x01, 0x60, 0x07, 0x02, 0x01, 0x03, 0x04, 0x00, 0x80, 0x00,
        ];
        let l = parse(&d).unwrap();
        assert_eq!(l.msg_id, 1);
        assert_eq!(l.op, 0);
        assert_eq!(l.op_name(), "bindRequest");
        assert!(!l.has_controls);
    }

    #[test]
    fn with_controls() {
        // SEQUENCE { INTEGER 2, [APPLICATION 5] "", [0] "" }
        let inner = vec![0x02, 0x01, 0x02, 0x65, 0x00, 0xA0, 0x00];
        let d = crate::der::encode(0x30, &inner);
        let l = parse(&d).unwrap();
        assert_eq!(l.msg_id, 2);
        assert_eq!(l.op, 5);
        assert!(l.has_controls);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x30, 0x03, 0x02, 0x01, 0x01]).is_none()); // no op
                                                                   // op is context class (2), not application (1) → reject
        assert!(parse(&[0x30, 0x05, 0x02, 0x01, 0x01, 0x80, 0x00]).is_none());
        assert_eq!(parse(&[0x30, 0x05, 0x02, 0x01, 0x01, 0x7F, 0x00]), None); // tag 127 > 25
    }
}
