use crate::rtc::socket::peer_socket_err::PeerSocketErr;
use std::fmt;

pub enum RtcError {
    RtcPeerError(PeerSocketErr),
}
impl fmt::Display for RtcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RtcError::RtcPeerError(err) => writeln!(f, "{}", err),
        }
    }
}
