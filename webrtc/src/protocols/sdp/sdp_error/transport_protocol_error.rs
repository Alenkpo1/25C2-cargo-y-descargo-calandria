use crate::protocols::sdp::sdp_consts::error_consts::{
    INVALID_TRANSPORT_PROTOCOL_ERROR, TRANSPORT_PROTOCOL_ERROR,
};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum TransportProtocolError {
    InvalidTransportProtocol(String),
}
impl fmt::Display for TransportProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransportProtocolError::InvalidTransportProtocol(str) => writeln!(
                f,
                "{}: \"{}\" {}",
                TRANSPORT_PROTOCOL_ERROR, str, INVALID_TRANSPORT_PROTOCOL_ERROR
            ),
        }
    }
}
