use crate::protocols::sdp::sdp_consts::error_consts::{
    INVALID_MEDIA_DESCRIPTION_KEY_ERROR, INVALID_MEDIA_DESCRITPION_LENGTH_ERROR,
    MEDIA_DESCRIPTION_ERROR,
};
use crate::protocols::sdp::sdp_error::media_type_error::MediaTypeError;
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use crate::protocols::sdp::sdp_error::transport_protocol_error::TransportProtocolError;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum MediaDescriptionError {
    InvalidMediaDescriptionLength(usize),
    InvalidMediaDescriptionKey(String),
    MediaDescritpionMediaTypeError(MediaTypeError),
    MediaDescriptionParseUIntError(ParsingError),
    MediaDescriptionTransportProtocolError(TransportProtocolError),
}
impl fmt::Display for MediaDescriptionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MediaDescriptionError::InvalidMediaDescriptionLength(value) => writeln!(
                f,
                "{}: \"{}\" {} ",
                MEDIA_DESCRIPTION_ERROR, value, INVALID_MEDIA_DESCRITPION_LENGTH_ERROR
            ),
            MediaDescriptionError::InvalidMediaDescriptionKey(value) => writeln!(
                f,
                "{}: \"{}\" {}",
                MEDIA_DESCRIPTION_ERROR, value, INVALID_MEDIA_DESCRIPTION_KEY_ERROR
            ),
            MediaDescriptionError::MediaDescritpionMediaTypeError(err) => write!(f, "{}", err),
            MediaDescriptionError::MediaDescriptionParseUIntError(err) => write!(f, "{}", err),
            MediaDescriptionError::MediaDescriptionTransportProtocolError(err) => {
                write!(f, "{}", err)
            }
        }
    }
}
impl From<ParsingError> for MediaDescriptionError {
    fn from(err: ParsingError) -> Self {
        MediaDescriptionError::MediaDescriptionParseUIntError(err)
    }
}
