use crate::protocols::sdp::sdp_consts::general_consts::{EQUAL_SYMBOL, SDP_VERSION_KEY};
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use crate::protocols::sdp::sdp_error::sdp_error::SdpError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct SdpVersion {
    version_int: u64,
}
impl SdpVersion {
    pub fn new(version_int: u64) -> SdpVersion {
        SdpVersion { version_int }
    }
}

impl fmt::Display for SdpVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}{}{}", SDP_VERSION_KEY, EQUAL_SYMBOL, self.version_int)
    }
}
impl FromStr for SdpVersion {
    type Err = SdpError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec_version: Vec<&str> = s.split_whitespace().collect();
        if vec_version.len() != 1 || s.len() < 2 {
            return Err(SdpError::InvalidSdpVersionFormat(s.to_string()));
        }
        if s[0..2] != format!("{}{}", SDP_VERSION_KEY, EQUAL_SYMBOL) {
            return Err(SdpError::InvalidSdpVersionFormat(s.to_string()));
        }
        let version = vec_version[0][2..]
            .parse()
            .map_err(|_| ParsingError::InvalidUint(vec_version[0][2..].to_string()))?;
        Ok(SdpVersion {
            version_int: version,
        })
    }
}
