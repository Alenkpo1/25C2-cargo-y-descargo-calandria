use crate::protocols::sdp::sdp_consts::error_consts::{
    INVALID_ORIGIN_KEY_ERROR, INVALID_ORIGIN_LENGTH_ERROR, ORIGIN_ERROR,
};
use crate::protocols::sdp::sdp_consts::general_consts::{EQUAL_SYMBOL, ORIGIN_KEY};
use crate::protocols::sdp::sdp_error::address_type_error::AddressTypeError;
use crate::protocols::sdp::sdp_error::net_type_error::NetTypeError;
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use std::fmt;
#[derive(Debug, PartialEq)]
pub enum OriginError {
    InvalidOriginLength(usize),
    InvalidOriginKey(String),
    OriginParseError(ParsingError),
    OriginNetTypeError(NetTypeError),
    OriginAddressTypeError(AddressTypeError),
}
impl fmt::Display for OriginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OriginError::InvalidOriginLength(n) => writeln!(
                f,
                "{}: {} \"{}\"",
                ORIGIN_ERROR, INVALID_ORIGIN_LENGTH_ERROR, n
            ),
            OriginError::InvalidOriginKey(str) => writeln!(
                f,
                "{}: {} \"{}{}\" \"{}\"",
                ORIGIN_ERROR, INVALID_ORIGIN_KEY_ERROR, ORIGIN_KEY, EQUAL_SYMBOL, str
            ),
            OriginError::OriginParseError(parsing_error) => write!(f, "{}", parsing_error),
            OriginError::OriginNetTypeError(net) => write!(f, "{}", net),
            OriginError::OriginAddressTypeError(addr_type) => write!(f, "{}", addr_type),
        }
    }
}
impl From<ParsingError> for OriginError {
    fn from(err: ParsingError) -> Self {
        OriginError::OriginParseError(err)
    }
}
