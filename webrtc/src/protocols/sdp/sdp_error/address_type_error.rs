use crate::protocols::sdp::sdp_consts::error_consts::{
    ADDRESS_TYPE_ERROR, INVALID_ADDRESS_TYPE_ERROR,
};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum AddressTypeError {
    InvalidAddrType(String),
}
impl fmt::Display for AddressTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AddressTypeError::InvalidAddrType(str) => writeln!(
                f,
                "{}: \"{}\" {}",
                ADDRESS_TYPE_ERROR, str, INVALID_ADDRESS_TYPE_ERROR
            ),
        }
    }
}
