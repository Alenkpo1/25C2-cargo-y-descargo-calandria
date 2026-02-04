use crate::camera::camera_const::{
    BGR_TO_RGB_ERROR, CAMERA_ERROR, CREATE_CAMERA_ERROR, FRAME_EMPTY_ERROR, NOT_OPEN_CAMERA,
    NOT_OPEN_CAMERA_ERROR_MSG, OPEN_CAMERA_ERROR, READ_FRAME_ERROR,
};
use std::fmt;

#[derive(Debug)]
pub enum CameraError {
    CameraCreationError(String),
    CameraOpenError(String),
    ReadFrameError(String),
    FrameEmpty,
    BgrToRgbError(String),
    NotOpenCamera,
}

impl From<opencv::Error> for CameraError {
    fn from(e: opencv::Error) -> Self {
        CameraError::ReadFrameError(format!("opencv error: code={} msg={}", e.code, e.message))
    }
}
impl From<&str> for CameraError {
    fn from(s: &str) -> Self {
        CameraError::CameraCreationError(s.to_string())
    }
}

impl From<String> for CameraError {
    fn from(s: String) -> Self {
        CameraError::CameraCreationError(s)
    }
}
impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CameraError::CameraCreationError(err) => {
                writeln!(f, "{}: \"{}\" {}", CAMERA_ERROR, CREATE_CAMERA_ERROR, err)
            }
            CameraError::CameraOpenError(err) => {
                writeln!(f, "{}: \"{}\" {}", CAMERA_ERROR, OPEN_CAMERA_ERROR, err)
            }
            CameraError::NotOpenCamera => writeln!(
                f,
                "{}: \"{}\" {}",
                CAMERA_ERROR, NOT_OPEN_CAMERA, NOT_OPEN_CAMERA_ERROR_MSG
            ),
            CameraError::ReadFrameError(err) => {
                writeln!(f, "{}: \"{}\" {}", CAMERA_ERROR, READ_FRAME_ERROR, err)
            }
            CameraError::FrameEmpty => writeln!(f, "{}: \"{}\"", CAMERA_ERROR, FRAME_EMPTY_ERROR),
            CameraError::BgrToRgbError(err) => {
                writeln!(f, "{}: \"{}\" {}", CAMERA_ERROR, BGR_TO_RGB_ERROR, err)
            }
        }
    }
}
