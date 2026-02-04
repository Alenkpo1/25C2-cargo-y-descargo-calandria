use crate::codec::h264::h264_const::encoder_const::{
    CREATE_ENCODER_ERROR, ENCODE_FAILED, ENCODER_ERROR,
};
use openh264::Error;
use std::fmt;

#[derive(Debug)]
pub enum EncoderError {
    CreateEncoderErr(Error),
    EncodeError(Error),
}
impl fmt::Display for EncoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncoderError::CreateEncoderErr(err) => {
                writeln!(f, "{}: \"{}\" {}", ENCODER_ERROR, CREATE_ENCODER_ERROR, err)
            }
            EncoderError::EncodeError(err) => {
                writeln!(f, "{}: \"{}\" {}", ENCODER_ERROR, ENCODE_FAILED, err)
            }
        }
    }
}
