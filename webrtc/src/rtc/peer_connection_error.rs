//! Error types for RTC peer connection.

use std::fmt;

use super::socket::peer_socket_err::PeerSocketErr;

/// Errors that can occur during peer connection operations.
#[derive(Debug)]
pub enum PeerConnectionError {
    /// Error propagated from the UDP socket layer.
    Socket(PeerSocketErr),
    /// Input/output error returned by the operating system.
    Io(std::io::Error),
    /// Error converting or interpreting SDP descriptions.
    Sdp(String),
    /// Error originating from ICE agent.
    Ice(String),
    /// The peer role does not allow the requested operation.
    InvalidRole(&'static str),
    /// Error in DTLS handshake or configuration.
    Dtls(String),
}

impl fmt::Display for PeerConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerConnectionError::Socket(err) => write!(f, "Peer socket error: {}", err),
            PeerConnectionError::Io(err) => write!(f, "IO error: {}", err),
            PeerConnectionError::Sdp(err) => write!(f, "SDP error: {}", err),
            PeerConnectionError::Ice(err) => write!(f, "ICE error: {}", err),
            PeerConnectionError::InvalidRole(msg) => write!(f, "Invalid role: {}", msg),
            PeerConnectionError::Dtls(msg) => write!(f, "DTLS error: {}", msg),
        }
    }
}

impl std::error::Error for PeerConnectionError {}

impl From<PeerSocketErr> for PeerConnectionError {
    fn from(value: PeerSocketErr) -> Self {
        PeerConnectionError::Socket(value)
    }
}

impl From<std::io::Error> for PeerConnectionError {
    fn from(value: std::io::Error) -> Self {
        PeerConnectionError::Io(value)
    }
}
