use crate::protocols::rtp::constants::rtp_err_const::{H264_TYPE_ERROR, INVALID_H264_TYPE_ERROR};
use std::fmt;

#[derive(Debug)]
pub enum H26VideoTypeErr {
    InvalidNalPayloadType(u8),
}
impl fmt::Display for H26VideoTypeErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            H26VideoTypeErr::InvalidNalPayloadType(number) => writeln!(
                f,
                "{}: \"{}\" {}",
                H264_TYPE_ERROR, INVALID_H264_TYPE_ERROR, number
            ),
        }
    }
}
