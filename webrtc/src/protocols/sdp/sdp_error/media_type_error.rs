use crate::protocols::sdp::sdp_consts::error_consts::{INVALID_MEDIA_TYPE_ERROR, MEDIA_TYPE_ERROR};
use std::fmt;

#[derive(PartialEq, Debug)]
pub enum MediaTypeError {
    InvalidMediaType(String),
}
impl fmt::Display for MediaTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MediaTypeError::InvalidMediaType(str) => writeln!(
                f,
                "{}: \"{}\" {}",
                MEDIA_TYPE_ERROR, str, INVALID_MEDIA_TYPE_ERROR
            ),
        }
    }
}
