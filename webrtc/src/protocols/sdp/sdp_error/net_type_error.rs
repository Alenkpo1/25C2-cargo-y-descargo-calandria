use crate::protocols::sdp::sdp_consts::error_consts::{INVALID_NET_TYPE_ERROR, NET_TYPE_ERROR};
use std::fmt;

#[derive(PartialEq, Debug)]
pub enum NetTypeError {
    InvalidNetType(String),
}
impl fmt::Display for NetTypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetTypeError::InvalidNetType(str) => writeln!(
                f,
                "{}: \"{}\" {}",
                NET_TYPE_ERROR, str, INVALID_NET_TYPE_ERROR
            ),
        }
    }
}
