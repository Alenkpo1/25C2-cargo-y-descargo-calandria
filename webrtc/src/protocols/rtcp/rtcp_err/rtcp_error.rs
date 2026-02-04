use crate::protocols::rtcp::rtcp_const::rtp_controller_const::{
    INVALID_PAYLOAD_RTCP_TYPE, INVALID_TYPE_SDES, RTCP_ERROR,
};
use std::fmt;

#[derive(Debug)]
pub enum RtcpError {
    SdesEnumReadError(u8),
    InvalidRtcpPayloadType(u8),
}
impl fmt::Display for RtcpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RtcpError::SdesEnumReadError(number) => {
                writeln!(f, "{}: \"{}\" {}", RTCP_ERROR, INVALID_TYPE_SDES, number)
            }
            RtcpError::InvalidRtcpPayloadType(number) => writeln!(
                f,
                "{}: \"{}\" {}",
                RTCP_ERROR, INVALID_PAYLOAD_RTCP_TYPE, number
            ),
        }
    }
}
