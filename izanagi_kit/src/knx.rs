//! KNXnet/IP frame header parsing (KNXnet/IP Core).
//!
//! Header: `header_len(1)=0x06 proto_ver(1)=0x10 service(2,BE) total_len(2,BE)`.
//! The declared total length must equal the packet size.
//!
//! ```
//! use izanagi_kit::knx::{parse, Service};
//!
//! // TUNNELING_REQUEST carrying a small cEMI
//! let f = [0x06u8, 0x10, 0x04, 0x20, 0, 9, 0x29, 0x00, 0xBC];
//! let k = parse(&f).unwrap();
//! assert_eq!(k.service, Service::TunnellingRequest);
//! assert_eq!(k.body_at, 6);
//! ```

/// Known KNXnet/IP service identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    /// 0x0201 — search request.
    SearchRequest,
    /// 0x0202 — search response.
    SearchResponse,
    /// 0x0203 — description request.
    DescriptionRequest,
    /// 0x0204 — description response.
    DescriptionResponse,
    /// 0x0205 — connect request.
    ConnectRequest,
    /// 0x0206 — connect response.
    ConnectResponse,
    /// 0x0209 — disconnect request.
    DisconnectRequest,
    /// 0x020A — disconnect response.
    DisconnectResponse,
    /// 0x0310 — device configuration request.
    DeviceConfigRequest,
    /// 0x0420 — tunnelling request.
    TunnellingRequest,
    /// 0x0421 — tunnelling ACK.
    TunnellingAck,
    /// 0x0530 — routing indication.
    RoutingIndication,
    /// Any other service id.
    Other(u16),
}

impl Service {
    /// Maps a raw BE service id.
    pub fn from_u16(v: u16) -> Service {
        match v {
            0x0201 => Service::SearchRequest,
            0x0202 => Service::SearchResponse,
            0x0203 => Service::DescriptionRequest,
            0x0204 => Service::DescriptionResponse,
            0x0205 => Service::ConnectRequest,
            0x0206 => Service::ConnectResponse,
            0x0209 => Service::DisconnectRequest,
            0x020A => Service::DisconnectResponse,
            0x0310 => Service::DeviceConfigRequest,
            0x0420 => Service::TunnellingRequest,
            0x0421 => Service::TunnellingAck,
            0x0530 => Service::RoutingIndication,
            o => Service::Other(o),
        }
    }
}

/// Parsed KNXnet/IP header.
#[derive(Debug)]
pub struct KnxIp {
    /// Service identifier (classified).
    pub service: Service,
    /// Raw service id.
    pub service_id: u16,
    /// Declared total frame length (equals packet size).
    pub total_len: usize,
    /// Offset of the body (always 6).
    pub body_at: usize,
}

/// Parses a KNXnet/IP frame. Requires `header_len==0x06`, `proto==0x10`,
/// and the declared length equal to `d.len()`.
pub fn parse(d: &[u8]) -> Option<KnxIp> {
    if d.len() < 6 || d[0] != 0x06 || d[1] != 0x10 {
        return None;
    }
    let service_id = ((d[2] as u16) << 8) | d[3] as u16;
    let total = (((d[4] as u16) << 8) | d[5] as u16) as usize;
    if total != d.len() {
        return None;
    }
    Some(KnxIp {
        service: Service::from_u16(service_id),
        service_id,
        total_len: total,
        body_at: 6,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let k = parse(&[0x06, 0x10, 0x02, 0x01, 0, 6]).unwrap();
        assert_eq!(k.service, Service::SearchRequest);
        assert_eq!(k.total_len, 6);
        let k = parse(&[0x06, 0x10, 0x05, 0x30, 0, 8, 0x11, 0x22]).unwrap();
        assert_eq!(k.service, Service::RoutingIndication);
        let k = parse(&[0x06, 0x10, 0x09, 0x99, 0, 6]).unwrap();
        assert_eq!(k.service, Service::Other(0x0999));
    }

    #[test]
    fn service_map() {
        assert_eq!(Service::from_u16(0x0420), Service::TunnellingRequest);
        assert_eq!(Service::from_u16(0xFFFF), Service::Other(0xFFFF));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0x05, 0x10, 0x02, 0x01, 0, 6]).is_none()); // bad header len
        assert!(parse(&[0x06, 0x20, 0x02, 0x01, 0, 6]).is_none()); // bad proto
        assert!(parse(&[0x06, 0x10, 0x02, 0x01, 0, 7]).is_none()); // len mismatch
        assert!(parse(&[0x06]).is_none());
    }
}
