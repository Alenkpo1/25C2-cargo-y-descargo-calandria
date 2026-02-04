use crate::protocols::sdp::sdp_consts::general_consts::{EQUAL_SYMBOL, TIME_KEY};
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use crate::protocols::sdp::sdp_error::sdp_error::SdpError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct Time {
    time: u64,
}
impl Time {
    pub fn new(time: u64) -> Time {
        Time { time }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}{}{}", TIME_KEY, EQUAL_SYMBOL, self.time)
    }
}

impl FromStr for Time {
    type Err = SdpError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec_time: Vec<&str> = s.split_whitespace().collect();
        if vec_time.len() != 1 || s.len() < 2 {
            return Err(SdpError::InvalidSdpVersionFormat(s.to_string()));
        }
        if s[0..2] != format!("{}{}", TIME_KEY, EQUAL_SYMBOL) {
            return Err(SdpError::InvalidSdpVersionFormat(s.to_string()));
        }
        let time = vec_time[0][2..]
            .parse()
            .map_err(|_| ParsingError::InvalidUint(vec_time[0][2..].to_string()))?;
        Ok(Time { time })
    }
}
