use crate::protocols::sdp::sdp_consts::error_consts::{
    INVALID_SDP_FORMAT, INVALID_SDP_LENGTH_ERROR, INVALID_SDP_TIME_FORMAT,
    INVALID_SDP_VERSION_FORMAT, SDP_ERROR,
};
use crate::protocols::sdp::sdp_error::attribute_error::AttributeError;
use crate::protocols::sdp::sdp_error::media_description_error::MediaDescriptionError;
use crate::protocols::sdp::sdp_error::origin_error::OriginError;
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum SdpError {
    InvalidParseIntSdp(ParsingError),
    OriginCreationError(OriginError),
    MediaDescriptionCreationError(MediaDescriptionError),
    AttributeCreationError(AttributeError),
    InvalidSdpVersionFormat(String),
    InvalidSdpFormatLength(usize),
    InvalidSdpTimeFormat(String),
    InvalidSdpFormat(String),
}
impl fmt::Display for SdpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SdpError::InvalidParseIntSdp(err) => write!(f, "{}", err),
            SdpError::OriginCreationError(err) => write!(f, "{}", err),
            SdpError::MediaDescriptionCreationError(err) => write!(f, "{}", err),
            SdpError::AttributeCreationError(err) => write!(f, "{}", err),
            SdpError::InvalidSdpVersionFormat(s) => {
                writeln!(f, "{}: \"{}\" {}", SDP_ERROR, s, INVALID_SDP_VERSION_FORMAT)
            }
            SdpError::InvalidSdpFormatLength(int) => {
                writeln!(f, "{}: \"{}\" {}", SDP_ERROR, int, INVALID_SDP_LENGTH_ERROR)
            }
            SdpError::InvalidSdpTimeFormat(str) => {
                writeln!(f, "{}: \"{}\" {}", SDP_ERROR, str, INVALID_SDP_TIME_FORMAT)
            }
            SdpError::InvalidSdpFormat(string) => {
                writeln!(f, "{}: \"{}\" {}", SDP_ERROR, string, INVALID_SDP_FORMAT)
            }
        }
    }
}
impl From<ParsingError> for SdpError {
    fn from(err: ParsingError) -> Self {
        SdpError::InvalidParseIntSdp(err)
    }
}

impl std::error::Error for SdpError {}
