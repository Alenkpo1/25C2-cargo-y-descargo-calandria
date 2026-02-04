use crate::protocols::sdp::sdp_consts::error_consts::{INVALID_UINT_ERROR, PARSING_ERROR};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ParsingError {
    InvalidUint(String),
}
impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParsingError::InvalidUint(str) => {
                writeln!(f, "{}: {} \"{}\"", PARSING_ERROR, INVALID_UINT_ERROR, str)
            }
        }
    }
}
