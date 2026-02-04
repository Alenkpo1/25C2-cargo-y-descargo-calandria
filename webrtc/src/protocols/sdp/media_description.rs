use crate::protocols::sdp::media_type::MediaType;

use crate::protocols::sdp::sdp_consts::general_consts::{EQUAL_SYMBOL, MEDIA_DESCRIPTION_KEY};
use crate::protocols::sdp::sdp_error::media_description_error::MediaDescriptionError;

use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use crate::protocols::sdp::transport_protocol::TransportProtocol;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct MediaDescription {
    media_type: MediaType,
    port: u32,
    transport: TransportProtocol,
    fmt: Vec<u8>,
}
impl MediaDescription {
    pub fn new(
        media_type: MediaType,
        port: u32,
        transport: TransportProtocol,
        fmt: Vec<u8>,
    ) -> Self {
        MediaDescription {
            media_type,
            port,
            transport,
            fmt,
        }
    }
}

impl fmt::Display for MediaDescription {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fmt_str: Vec<String> = self.fmt.iter().map(|num| num.to_string()).collect();
        let fmt_joined = fmt_str.join(" ");
        writeln!(
            f,
            "{}{}{} {} {} {}",
            MEDIA_DESCRIPTION_KEY,
            EQUAL_SYMBOL,
            self.media_type,
            self.port,
            self.transport,
            fmt_joined,
        )
    }
}

impl FromStr for MediaDescription {
    type Err = MediaDescriptionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec_media_description: Vec<&str> = s.split_whitespace().collect();
        if vec_media_description.len() < 4 || s.len() < 2 {
            return Err(MediaDescriptionError::InvalidMediaDescriptionLength(
                vec_media_description.len(),
            ));
        }
        if s[0..2] != format!("{}{}", MEDIA_DESCRIPTION_KEY, EQUAL_SYMBOL) {
            return Err(MediaDescriptionError::InvalidMediaDescriptionKey(
                s[0..2].to_string(),
            ));
        }
        let media_type_str = &vec_media_description[0][2..];
        let media_type = MediaType::from_str(media_type_str)
            .map_err(MediaDescriptionError::MediaDescritpionMediaTypeError)?;
        let port = vec_media_description[1]
            .parse::<u32>()
            .map_err(|_| ParsingError::InvalidUint(vec_media_description[1].to_string()))?;
        let transport = TransportProtocol::from_str(vec_media_description[2])
            .map_err(MediaDescriptionError::MediaDescriptionTransportProtocolError)?;
        let mut fmt = Vec::new();
        for value in &vec_media_description[3..] {
            match value.parse::<u8>() {
                Ok(num) => fmt.push(num),
                Err(_) => {
                    return Err(MediaDescriptionError::MediaDescriptionParseUIntError(
                        ParsingError::InvalidUint(value.to_string()),
                    ));
                }
            };
        }
        Ok(MediaDescription {
            media_type,
            port,
            transport,
            fmt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::general_consts::MEDIA_DESCRIPTION_KEY;

    #[test]
    fn test_media_description_display() {
        let media_type_value = MediaType::Video;
        let port_value = 4000;
        let transport_protocol_value = TransportProtocol::RtpAvp;
        let fmt_value1 = 50;
        let fmt_value2 = 60;
        let mut fmt: Vec<u8> = Vec::new();
        fmt.push(fmt_value1);
        fmt.push(fmt_value2);
        let media_description =
            MediaDescription::new(media_type_value, port_value, TransportProtocol::RtpAvp, fmt);
        let media_description_str = format!("{}", media_description);
        assert_eq!(
            format!("{}", media_description_str),
            format!(
                "{}{}{} {} {} {} {}\n",
                MEDIA_DESCRIPTION_KEY,
                EQUAL_SYMBOL,
                MediaType::Video,
                port_value,
                transport_protocol_value,
                fmt_value1,
                fmt_value2
            )
        );
    }

    #[test]
    fn test_from_str_media_descritpion_ok() -> Result<(), MediaDescriptionError> {
        let media_type_value = MediaType::Video;
        let port_value = 4000;
        let transport_protocol_value = TransportProtocol::RtpAvp;
        let fmt_value1 = 50;
        let fmt_value2 = 60;
        let mut fmt: Vec<u8> = Vec::new();
        fmt.push(fmt_value1);
        fmt.push(fmt_value2);
        let value = format!(
            "{}{}{} {} {} {} {}",
            MEDIA_DESCRIPTION_KEY,
            EQUAL_SYMBOL,
            media_type_value,
            port_value,
            transport_protocol_value,
            fmt_value1,
            fmt_value2
        );
        let media_description = MediaDescription::from_str(&value)?;
        assert_eq!(media_description.media_type, MediaType::Video);
        assert_eq!(media_description.port, port_value);
        assert_eq!(media_description.transport, TransportProtocol::RtpAvp);
        assert_eq!(media_description.fmt[0], fmt_value1);
        assert_eq!(media_description.fmt[1], fmt_value2);
        Ok(())
    }
    #[test]
    fn test_from_str_media_description_invalid_length() {
        let media_type_value = MediaType::Video;
        let value = format!("{}", media_type_value);
        let value_vec_len = value.split_whitespace().count();
        let media_description = MediaDescription::from_str(&value);
        assert!(matches!(
            media_description,
            Err(MediaDescriptionError::InvalidMediaDescriptionLength(len)) if len == value_vec_len
        ));
    }
}
