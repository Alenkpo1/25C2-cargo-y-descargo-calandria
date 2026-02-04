use std::fmt;
use std::str::FromStr;

use crate::protocols::sdp::sdp_consts::general_consts::VIDEO_STR;
use crate::protocols::sdp::sdp_error::media_type_error::MediaTypeError;

#[derive(Debug, PartialEq)]
pub enum MediaType {
    Video,
}
impl FromStr for MediaType {
    type Err = MediaTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            VIDEO_STR => Ok(MediaType::Video),
            not_found => Err(MediaTypeError::InvalidMediaType(not_found.to_string())),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MediaType::Video => write!(f, "{}", VIDEO_STR),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        INVALID_MEDIA_TYPE_ERROR, MEDIA_TYPE_ERROR,
    };
    #[test]
    fn test_media_type_from_str_video() {
        let video_type = MediaType::from_str(VIDEO_STR).unwrap();
        assert_eq!(video_type, MediaType::Video);
    }
    #[test]
    fn test_display_video() {
        let video_type = MediaType::Video;
        assert_eq!(VIDEO_STR, video_type.to_string());
    }
    #[test]
    fn test_from_str_media_type_err() {
        let value = "He";
        let media_type_err = MediaType::from_str(value).unwrap_err();
        assert_eq!(
            MediaTypeError::InvalidMediaType(value.to_string()),
            media_type_err
        );
        assert_eq!(
            format!("{}", media_type_err),
            format!(
                "{}: \"{}\" {}\n",
                MEDIA_TYPE_ERROR, value, INVALID_MEDIA_TYPE_ERROR
            )
        );
    }
}
