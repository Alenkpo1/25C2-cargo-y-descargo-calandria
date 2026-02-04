use crate::protocols::sdp::sdp_consts::error_consts::{
    ATTRIBUTE_ERROR, INVALID_ATTRIBUTE_FORMAT_ERROR, INVALID_KEY_ATTRIBUTE_ERROR,
    INVALID_KEY_VALUE_FORMAT_ERROR, INVALID_VALUE_FORMAT_ERROR,
};
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum AttributeError {
    InvalidKeyValueFormat(String),
    InvalidValueFormat(String),
    AttributeParseError(ParsingError),
    InvalidKeyAttribute(String),
    InvalidAttributeFormat(String),
}
impl fmt::Display for AttributeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AttributeError::InvalidKeyValueFormat(value) => writeln!(
                f,
                "{}: \"{}\" {}",
                ATTRIBUTE_ERROR, value, INVALID_KEY_VALUE_FORMAT_ERROR
            ),
            AttributeError::InvalidValueFormat(value) => writeln!(
                f,
                "{}: \"{}\" {}",
                ATTRIBUTE_ERROR, value, INVALID_VALUE_FORMAT_ERROR
            ),
            AttributeError::AttributeParseError(err) => write!(f, "{}", err),
            AttributeError::InvalidKeyAttribute(value) => writeln!(
                f,
                "{}: \"{}\" {}",
                ATTRIBUTE_ERROR, value, INVALID_KEY_ATTRIBUTE_ERROR
            ),
            AttributeError::InvalidAttributeFormat(value) => writeln!(
                f,
                "{}: {} \"{}\"",
                ATTRIBUTE_ERROR, INVALID_ATTRIBUTE_FORMAT_ERROR, value
            ),
        }
    }
}
impl From<ParsingError> for AttributeError {
    fn from(err: ParsingError) -> Self {
        AttributeError::AttributeParseError(err)
    }
}
