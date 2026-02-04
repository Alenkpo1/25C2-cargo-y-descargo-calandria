use crate::camera::camera_err::CameraError;
use crate::codec::h264::h264_err::encoder_err::EncoderError;
use opencv::Error;
use std::fmt;

#[derive(Debug)]
pub enum WorkerError {
    SendError,
    CaptureFrameError(CameraError),
    ConvertRgbFrame(CameraError),
    ConvertToYuvError(Error),
    InvalidEncoding(EncoderError),
}
impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorkerError::SendError => writeln!(f, "close thread"),
            WorkerError::CaptureFrameError(err) => writeln!(f, "{}", err),
            WorkerError::ConvertRgbFrame(err) => writeln!(f, "{}", err),
            WorkerError::ConvertToYuvError(err) => writeln!(f, "{}", err),
            WorkerError::InvalidEncoding(err) => writeln!(f, "{}", err),
        }
    }
}
