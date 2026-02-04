//! Implementation of STUN attributes relevant to binding responses.

use super::MAGIC_COOKIE;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Reader for the `XOR-MAPPED-ADDRESS` attribute.
pub struct XorMappedAddress;

impl XorMappedAddress {
    const ATTRIBUTE_TYPE: u16 = 0x0020;

    /// XOR-MAPPED-ADDRESS parsing
    pub fn parse(
        data: &[u8],
        transaction_id: &[u8; 12],
    ) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
        if data.len() < 12 {
            return Ok(None);
        }

        // search the XOR-MAPPED-ADDRESS attribute
        let attr_type = u16::from_be_bytes([data[0], data[1]]);
        if attr_type != Self::ATTRIBUTE_TYPE {
            return Ok(None);
        }

        let attr_length = u16::from_be_bytes([data[2], data[3]]);
        if attr_length < 8 {
            return Ok(None);
        }

        // parse ipv4 or ipv6
        let family = data[5];

        match family {
            0x01 => Self::parse_ipv4(&data[4..]),
            0x02 => Self::parse_ipv6(&data[4..], transaction_id),
            _ => Ok(None),
        }
    }

    /// Decodes the IPv4 address contained in the attribute.
    fn parse_ipv4(data: &[u8]) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
        if data.len() < 8 {
            return Ok(None);
        }

        // XOR port with the firsts 16 bits of the magic cookie
        let xor_port = u16::from_be_bytes([data[2], data[3]]);
        let port = xor_port ^ (MAGIC_COOKIE >> 16) as u16;

        // XOR IP with the full magic cookie
        let xor_ip = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ip = xor_ip ^ MAGIC_COOKIE;

        Ok(Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(ip)), port)))
    }

    /// decodes ipv6 soon
    fn parse_ipv6(
        _data: &[u8],
        _transaction_id: &[u8; 12],
    ) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_parse_xor_mapped_address_ipv4() {
        // make a simulated XOR-MAPPED-ADDRESS message
        let transaction_id: [u8; 12] = [0; 12];
        let data = vec![
            0x00, 0x20, // Attribute type: XOR-MAPPED-ADDRESS
            0x00, 0x08, // Length: 8 bytes
            0x00, // Reserved
            0x01, // Family: IPv4
            0x21, 0x12, // XOR'd port (must be 0x0000 post XOR)
            0x21, 0x12, 0xA4, 0x42, // XOR'd IP (será 0.0.0.0 después de XOR)
        ];

        let result = XorMappedAddress::parse(&data, &transaction_id);
        assert!(result.is_ok());

        let addr = result.unwrap();
        assert!(addr.is_some());

        let socket_addr = addr.unwrap();
        // XOR'd port: 0x2112 ^ 0x2112 = 0x0000
        assert_eq!(socket_addr.port(), 0);

        // IP XOR'd: 0x2112A442 ^ 0x2112A442 = 0x00000000 (0.0.0.0)
        match socket_addr.ip() {
            IpAddr::V4(ip) => assert_eq!(ip, Ipv4Addr::new(0, 0, 0, 0)),
            _ => panic!("Expected IPv4 address"),
        }
    }

    #[test]
    fn test_parse_wrong_attribute_type() {
        let transaction_id: [u8; 12] = [0; 12];
        let data = vec![
            0x00, 0x01, // Wrong attribute type
            0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let result = XorMappedAddress::parse(&data, &transaction_id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_short_data() {
        let transaction_id: [u8; 12] = [0; 12];
        let data = vec![0x00, 0x20, 0x00, 0x08]; //very short

        let result = XorMappedAddress::parse(&data, &transaction_id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
