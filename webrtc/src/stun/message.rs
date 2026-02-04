//! Construction and parsing of STUN Binding messages.

use super::MAGIC_COOKIE;
use super::attributes::XorMappedAddress;
use std::net::{IpAddr, SocketAddr};

/// Message types supported by the STUN implementation.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    BindingRequest,
    BindingResponse,
    BindingErrorResponse,
    Unknown(u16),
}

impl MessageType {
    /// Creates a message type based on the numeric value of the header.
    pub fn from_u16(value: u16) -> Self {
        match value {
            0x0001 => MessageType::BindingRequest,
            0x0101 => MessageType::BindingResponse,
            0x0111 => MessageType::BindingErrorResponse,
            other => MessageType::Unknown(other),
        }
    }

    /// Converts a message type to the value used in the STUN header.
    pub fn to_u16(&self) -> u16 {
        match self {
            MessageType::BindingRequest => 0x0001,
            MessageType::BindingResponse => 0x0101,
            MessageType::BindingErrorResponse => 0x0111,
            MessageType::Unknown(val) => *val,
        }
    }
}

/// STUN message along with the discovered address, when applicable.
#[derive(Debug)]
pub struct StunMessage {
    pub message_type: MessageType,
    pub length: u16,
    pub transaction_id: [u8; 12],
    pub xor_mapped_address: Option<SocketAddr>,
}

impl StunMessage {
    /// Build a new Binding Request with a random identifier.
    pub fn create_binding_request() -> Vec<u8> {
        Self::create_binding_request_with_transaction().0
    }

    /// Same as [`Self::create_binding_request`] but returns the transaction ID used.
    pub fn create_binding_request_with_transaction() -> (Vec<u8>, [u8; 12]) {
        let mut msg = Vec::with_capacity(20);

        // Type: Binding Request (0x0001)
        msg.extend_from_slice(&MessageType::BindingRequest.to_u16().to_be_bytes());

        // Length: 0 (no atributes)
        msg.extend_from_slice(&0x0000u16.to_be_bytes());

        // magic cookie
        msg.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());

        let transaction_id = Self::generate_transaction_id();
        msg.extend_from_slice(&transaction_id);

        (msg, transaction_id)
    }

    /// Build a Binding Success Response with address XOR-MAPPED-ADDRESS.
    pub fn create_binding_success(transaction_id: [u8; 12], addr: SocketAddr) -> Vec<u8> {
        let mut msg = Vec::with_capacity(20 + 12);

        msg.extend_from_slice(&MessageType::BindingResponse.to_u16().to_be_bytes());

        // Attribute length (XOR-MAPPED-ADDRESS 12 bytes total)
        msg.extend_from_slice(&12u16.to_be_bytes());

        msg.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        msg.extend_from_slice(&transaction_id);

        if let IpAddr::V4(ipv4) = addr.ip() {
            // Attribute header
            msg.extend_from_slice(&0x0020u16.to_be_bytes());
            msg.extend_from_slice(&0x0008u16.to_be_bytes());

            // Reserved + family (IPv4)
            msg.push(0x00);
            msg.push(0x01);

            let xor_port = addr.port() ^ ((MAGIC_COOKIE >> 16) as u16);
            msg.extend_from_slice(&xor_port.to_be_bytes());

            let xor_ip = u32::from_be_bytes(ipv4.octets()) ^ MAGIC_COOKIE;
            msg.extend_from_slice(&xor_ip.to_be_bytes());
        }

        msg
    }

    /// Analyzes a STUN message and returns the structured representation.
    pub fn parse(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() < 20 {
            return Err("STUN message too short".into());
        }

        // header parsing
        let message_type = MessageType::from_u16(u16::from_be_bytes([data[0], data[1]]));
        let length = u16::from_be_bytes([data[2], data[3]]);

        // magic cookie check
        let magic = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if magic != MAGIC_COOKIE {
            return Err("invalid Magic Cookie".into());
        }

        // Transaction ID
        let mut transaction_id = [0u8; 12];
        transaction_id.copy_from_slice(&data[8..20]);

        // atribute parsing
        let xor_mapped_address = if data.len() > 20 {
            XorMappedAddress::parse(&data[20..], &transaction_id)?
        } else {
            None
        };

        Ok(StunMessage {
            message_type,
            length,
            transaction_id,
            xor_mapped_address,
        })
    }

    /// Generates a pseudo-random identifier for STUN transactions.
    fn generate_transaction_id() -> [u8; 12] {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => dur.as_nanos(),
            Err(err) => {
                eprintln!("STUN txid clock error, using 0: {}", err);
                0
            }
        };

        let mut id = [0u8; 12];
        let bytes = now.to_be_bytes();
        id[..8].copy_from_slice(&bytes[8..16]);
        id[8..12].copy_from_slice(&bytes[0..4]);

        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_binding_request() {
        let request = StunMessage::create_binding_request();

        // size check (20 bytes header)
        assert_eq!(request.len(), 20);

        // message type check (Binding Request = 0x0001)
        let msg_type = u16::from_be_bytes([request[0], request[1]]);
        assert_eq!(msg_type, 0x0001);

        // magic cookie check
        let magic = u32::from_be_bytes([request[4], request[5], request[6], request[7]]);
        assert_eq!(magic, MAGIC_COOKIE);
    }

    #[test]
    fn test_message_type_conversion() {
        assert_eq!(MessageType::from_u16(0x0001), MessageType::BindingRequest);
        assert_eq!(MessageType::from_u16(0x0101), MessageType::BindingResponse);
        assert_eq!(
            MessageType::from_u16(0x0111),
            MessageType::BindingErrorResponse
        );

        assert_eq!(MessageType::BindingRequest.to_u16(), 0x0001);
        assert_eq!(MessageType::BindingResponse.to_u16(), 0x0101);
    }

    #[test]
    fn test_parse_invalid_message() {
        let short_msg = vec![0u8; 10]; // Mensaje muy corto
        let result = StunMessage::parse(&short_msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_magic_cookie() {
        let mut msg = vec![0u8; 20];
        // valid message type
        msg[0] = 0x01;
        msg[1] = 0x01;
        // invalid Magic cookie
        msg[4] = 0xFF;
        msg[5] = 0xFF;
        msg[6] = 0xFF;
        msg[7] = 0xFF;

        let result = StunMessage::parse(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_id_is_unique() {
        let request1 = StunMessage::create_binding_request();
        let request2 = StunMessage::create_binding_request();

        // Transaction IDs must be differents
        assert_ne!(&request1[8..20], &request2[8..20]);
    }
}
