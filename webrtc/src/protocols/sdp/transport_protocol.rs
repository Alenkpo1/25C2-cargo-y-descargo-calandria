use crate::protocols::sdp::sdp_consts::general_consts::{RTP_AVP, RTP_SAVP, UDP};
use crate::protocols::sdp::sdp_error::transport_protocol_error::TransportProtocolError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum TransportProtocol {
    Udp,
    RtpAvp,
    RtpSavp,
}
impl FromStr for TransportProtocol {
    type Err = TransportProtocolError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            UDP => Ok(TransportProtocol::Udp),
            RTP_AVP => Ok(TransportProtocol::RtpAvp),
            RTP_SAVP => Ok(TransportProtocol::RtpSavp),
            not_found => Err(TransportProtocolError::InvalidTransportProtocol(
                not_found.to_string(),
            )),
        }
    }
}

impl fmt::Display for TransportProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportProtocol::Udp => write!(f, "{}", UDP),
            TransportProtocol::RtpAvp => write!(f, "{}", RTP_AVP),
            TransportProtocol::RtpSavp => write!(f, "{}", RTP_SAVP),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        INVALID_TRANSPORT_PROTOCOL_ERROR, TRANSPORT_PROTOCOL_ERROR,
    };
    #[test]
    fn test_from_str_all_ok() {
        let udp_protocol = TransportProtocol::from_str(UDP).unwrap();
        let rtp_avp = TransportProtocol::from_str(RTP_AVP).unwrap();
        let rtp_savp = TransportProtocol::from_str(RTP_SAVP).unwrap();

        assert_eq!(udp_protocol, TransportProtocol::Udp);
        assert_eq!(rtp_avp, TransportProtocol::RtpAvp);
        assert_eq!(rtp_savp, TransportProtocol::RtpSavp);
    }
    #[test]
    fn test_from_str_transport_protocol_err() {
        let value = "h2";
        let transport_protocol_err = TransportProtocol::from_str(value).unwrap_err();
        assert_eq!(
            TransportProtocolError::InvalidTransportProtocol(value.to_string()),
            transport_protocol_err
        );
        assert_eq!(
            format!("{}", transport_protocol_err),
            format!(
                "{}: \"{}\" {}\n",
                TRANSPORT_PROTOCOL_ERROR, value, INVALID_TRANSPORT_PROTOCOL_ERROR
            )
        );
    }
    #[test]
    fn test_display_all() {
        let udp_protocol = TransportProtocol::Udp;
        let rtp_avp = TransportProtocol::RtpAvp;
        let rtp_savp = TransportProtocol::RtpSavp;
        assert_eq!(UDP, udp_protocol.to_string());
        assert_eq!(RTP_AVP, rtp_avp.to_string());
        assert_eq!(RTP_SAVP, rtp_savp.to_string());
    }
}
