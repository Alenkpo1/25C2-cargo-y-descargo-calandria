use crate::protocols::rtp::constants::rtp_err_const::{INVALID_RTP_PAYLOAD_TYPE_ERROR, RTP_ERROR};
use crate::protocols::rtp::rtp_err::h26_video_type_err::H26VideoTypeErr;
use std::fmt;

#[derive(Debug)]
pub enum RtpError {
    InvalidH264(H26VideoTypeErr),
    InvalidRtpPayloadType(u8),
}
impl fmt::Display for RtpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RtpError::InvalidH264(err) => write!(f, "{}", err),
            RtpError::InvalidRtpPayloadType(number) => write!(
                f,
                "{}: \"{}\" {}",
                RTP_ERROR, number, INVALID_RTP_PAYLOAD_TYPE_ERROR
            ),
        }
    }
}
