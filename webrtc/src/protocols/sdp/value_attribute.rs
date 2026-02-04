use crate::protocols::sdp::sdp_consts::general_consts::{
    CANDIDATE, CAT, FINGERPRINT, GROUP, ICE_PWD, ICE_UFRAG, MAXPTIME, MSID_SEMANTIC, PTIME, RTPMAP,
};
use crate::protocols::sdp::sdp_error::attribute_error::AttributeError;
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use std::str::FromStr;
use std::{fmt, str};

#[derive(Debug)]
pub enum ValueAttribute {
    RtpMap {
        payload_type: u64,
        encoding_name: String,
        clock_rate: u64,
    },
    PTime(u64),
    MaxPtime(u64),
    Cat(String),

    IceUfrag(String),
    IcePwd(String),
    Candidate {
        foundation: u32,
        component: u32,
        protocol: String,
        priority: u32,
        address: String,
        port: u32,
        typ: String,
    },
    Fingerprint(String, String), // Acá le pongo (hash function, fp)
    Group(String),
    MsidSemantic,
}

impl FromStr for ValueAttribute {
    type Err = AttributeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s
            .split_once(':')
            .ok_or(AttributeError::InvalidKeyValueFormat(s.to_string()))?;
        match key {
            RTPMAP => from_str_rtpmap(value),
            PTIME => {
                let time = value
                    .parse::<u64>()
                    .map_err(|_| ParsingError::InvalidUint(value.to_string()))?;
                Ok(ValueAttribute::PTime(time))
            }
            MAXPTIME => {
                let max_time = value
                    .parse::<u64>()
                    .map_err(|_| ParsingError::InvalidUint(value.to_string()))?;
                Ok(ValueAttribute::MaxPtime(max_time))
            }
            CAT => Ok(ValueAttribute::Cat(value.to_string())),

            ICE_UFRAG => Ok(ValueAttribute::IceUfrag(value.to_string())),

            ICE_PWD => Ok(ValueAttribute::IcePwd(value.to_string())),

            CANDIDATE => from_str_candidate(value),

            FINGERPRINT => from_str_fingerprint(value),

            GROUP => Ok(ValueAttribute::Group(value.to_string())),

            MSID_SEMANTIC => {
                // El valor "WMS" es estándar, así que no necesitamos almacenarlo.
                Ok(ValueAttribute::MsidSemantic)
            }

            _ => Err(AttributeError::InvalidKeyAttribute(key.to_string())),
        }
    }
}

impl fmt::Display for ValueAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueAttribute::RtpMap {
                payload_type,
                encoding_name,
                clock_rate,
            } => write!(
                f,
                "{}:{} {}/{}",
                RTPMAP, payload_type, encoding_name, clock_rate
            ),
            ValueAttribute::PTime(time) => write!(f, "{}:{}", PTIME, time),
            ValueAttribute::MaxPtime(time) => write!(f, "{}:{}", MAXPTIME, time),
            ValueAttribute::Cat(value) => write!(f, "{}:{}", CAT, value),

            ValueAttribute::IceUfrag(ufrag) => write!(f, "{}:{}", ICE_UFRAG, ufrag),
            ValueAttribute::IcePwd(pwd) => write!(f, "{}:{}", ICE_PWD, pwd),
            ValueAttribute::Candidate {
                foundation,
                component,
                protocol,
                priority,
                address,
                port,
                typ,
            } => write!(
                f,
                "{}:{} {} {} {} {} {} typ {}",
                CANDIDATE, foundation, component, protocol, priority, address, port, typ
            ),
            ValueAttribute::Fingerprint(hash_func, hash_value) => {
                write!(f, "{}:{} {}",FINGERPRINT, hash_func, hash_value)
            }
            ValueAttribute::Group(value) => write!(f, "{}:{}", GROUP, value),
            // WMS is the default value
            ValueAttribute::MsidSemantic => write!(f, "{}:WMS", MSID_SEMANTIC),
        }
    }
}

fn from_str_candidate(value: &str) -> Result<ValueAttribute, AttributeError> {
    // format: 1 1 UDP 2130706431 192.168.1.100 50000 typ host
    let parts: Vec<&str> = value.split_whitespace().collect();

    if parts.len() < 8 {
        return Err(AttributeError::InvalidValueFormat(value.to_string()));
    }

    let foundation = parts[0]
        .parse::<u32>()
        .map_err(|_| ParsingError::InvalidUint(parts[0].to_string()))?;

    let component = parts[1]
        .parse::<u32>()
        .map_err(|_| ParsingError::InvalidUint(parts[1].to_string()))?;

    let protocol = parts[2].to_string();

    let priority = parts[3]
        .parse::<u32>()
        .map_err(|_| ParsingError::InvalidUint(parts[3].to_string()))?;

    let address = parts[4].to_string();

    let port = parts[5]
        .parse::<u32>()
        .map_err(|_| ParsingError::InvalidUint(parts[5].to_string()))?;

    if parts[6] != "typ" {
        return Err(AttributeError::InvalidValueFormat(value.to_string()));
    }

    let typ = parts[7].to_string();

    Ok(ValueAttribute::Candidate {
        foundation,
        component,
        protocol,
        priority,
        address,
        port,
        typ,
    })
}

fn from_str_rtpmap(value: &str) -> Result<ValueAttribute, AttributeError> {
    let vec_value: Vec<&str> = value.split_whitespace().collect();
    if vec_value.len() != 2 {
        return Err(AttributeError::InvalidValueFormat(value.to_string()));
    }
    let payload_type = vec_value[0]
        .parse::<u64>()
        .map_err(|_| ParsingError::InvalidUint(vec_value[0].to_string()))?;
    let encoding_vector: Vec<&str> = vec_value[1].split('/').collect();
    if encoding_vector.len() != 2 {
        return Err(AttributeError::InvalidValueFormat(value.to_string()));
    }
    let encoding_name = encoding_vector[0].to_string();
    let clock_rate = encoding_vector[1]
        .parse::<u64>()
        .map_err(|_| ParsingError::InvalidUint(encoding_vector[1].to_string()))?;
    Ok(ValueAttribute::RtpMap {
        payload_type,
        encoding_name,
        clock_rate,
    })
}

fn from_str_fingerprint(value: &str) -> Result<ValueAttribute, AttributeError> {
    // El valor viene así: "sha-256 AA:BB:CC..."

    let parts: Vec<&str> = value.split_whitespace().collect();

    if parts.len() != 2 {
        return Err(AttributeError::InvalidValueFormat(value.to_string()));
    }
    // Separar los dos
    let hash_func = parts[0].to_string(); // "sha-256"
    let fingerprint = parts[1].to_string(); // "AA:BB:CC"

    Ok(ValueAttribute::Fingerprint(hash_func, fingerprint))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        ATTRIBUTE_ERROR, INVALID_KEY_VALUE_FORMAT_ERROR, INVALID_UINT_ERROR,
        INVALID_VALUE_FORMAT_ERROR, PARSING_ERROR,
    };
    #[test]
    fn test_from_str_rtpmap_ok() {
        let string_value = format!("{}:96 L8/8000", RTPMAP);
        let rtpmap_value = ValueAttribute::from_str(&string_value).unwrap();
        assert_eq!(rtpmap_value.to_string(), string_value);
    }
    #[test]
    fn test_display_rtpmap() {
        let display = ValueAttribute::RtpMap {
            payload_type: 96,
            encoding_name: "L8".to_string(),
            clock_rate: 8000,
        };
        assert_eq!(display.to_string(), format!("{}:96 L8/8000", RTPMAP));
    }
    #[test]
    fn test_from_str_rtpmap_invalid_value_format_length_err() {
        let string_value = format!("{}:96", RTPMAP);
        let rtpmap_err = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::InvalidValueFormat("96".to_string()),
            rtpmap_err
        );
        assert_eq!(
            format!("{}", rtpmap_err),
            format!(
                "{}: \"96\" {}\n",
                ATTRIBUTE_ERROR, INVALID_VALUE_FORMAT_ERROR
            )
        );
    }
    #[test]
    fn test_from_str_rtpmap_invalid_parse_payload_err() {
        let string_value = format!("{}:as2 L8/8000", RTPMAP);
        let rtpmap_vec: Vec<&str> = string_value.split(':').collect();
        let value_vec: Vec<&str> = rtpmap_vec[1].split_whitespace().collect();
        let payload = value_vec[0];
        let rtpmap_err = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::AttributeParseError(ParsingError::InvalidUint(payload.to_string())),
            rtpmap_err
        );
        assert_eq!(
            format!("{}", rtpmap_err),
            format!(
                "{}: {} \"{}\"\n",
                PARSING_ERROR, INVALID_UINT_ERROR, payload
            )
        );
    }
    #[test]
    fn test_from_str_rtpmap_invalid_value_format_encoding_err() {
        let string_value = format!("{}:96 ls", RTPMAP);
        let rtpmap_err = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::InvalidValueFormat("96 ls".to_string()),
            rtpmap_err
        );
        assert_eq!(
            format!("{}", rtpmap_err),
            format!(
                "{}: \"96 ls\" {}\n",
                ATTRIBUTE_ERROR, INVALID_VALUE_FORMAT_ERROR
            )
        );
    }
    #[test]
    fn test_from_str_rtpmap_invalid_parse_clockrate_err() {
        let string_value = format!("{}:22 L8/800d0", RTPMAP);
        let rtpmap_vec: Vec<&str> = string_value.split(':').collect();
        let value_vec: Vec<&str> = rtpmap_vec[1].split_whitespace().collect();
        let _payload = value_vec[0];
        let rest: Vec<&str> = rtpmap_vec[1].split("/").collect();
        let clock_rate = rest[1];
        let rtpmap_err = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::AttributeParseError(ParsingError::InvalidUint(clock_rate.to_string())),
            rtpmap_err
        );
        assert_eq!(
            format!("{}", rtpmap_err),
            format!(
                "{}: {} \"{}\"\n",
                PARSING_ERROR, INVALID_UINT_ERROR, clock_rate
            )
        );
    }
    #[test]
    fn test_from_str_p_time_ok() {
        let value = 100;
        let string_value = format!("{}:{}", PTIME, value);
        let ptime_value = ValueAttribute::from_str(&string_value).unwrap();
        assert_eq!(ptime_value.to_string(), string_value);
    }
    #[test]
    fn test_display_p_time() {
        let value_time = 100;
        let string_value = format!("{}:{}", PTIME, value_time);
        let display = ValueAttribute::PTime(value_time);
        assert_eq!(display.to_string(), string_value);
    }
    #[test]
    fn test_from_str_p_time_error() {
        let value = "10aa0";
        let string_value = format!("{}:{}", PTIME, value.to_string());
        let ptime_error = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::AttributeParseError(ParsingError::InvalidUint(value.to_string())),
            ptime_error
        );
        assert_eq!(
            format!("{}", ptime_error),
            format!("{}: {} \"{}\"\n", PARSING_ERROR, INVALID_UINT_ERROR, value)
        );
    }
    #[test]
    fn test_from_str_max_p_time_ok() {
        let value = 200;
        let string_value = format!("{}:{}", MAXPTIME, value.to_string());
        let max_ptime_value = ValueAttribute::from_str(&string_value).unwrap();
        assert_eq!(max_ptime_value.to_string(), string_value);
    }
    #[test]
    fn test_display_max_p_time() {
        let value_time = 200;
        let string_value = format!("{}:{}", MAXPTIME, value_time);
        let display = ValueAttribute::MaxPtime(value_time);
        assert_eq!(display.to_string(), string_value);
    }
    #[test]
    fn test_from_str_max_p_time_error() {
        let value = "20as02";
        let string_value = format!("{}:{}", MAXPTIME, value.to_string());
        let max_ptime_error = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::AttributeParseError(ParsingError::InvalidUint(value.to_string())),
            max_ptime_error
        );
        assert_eq!(
            format!("{}", max_ptime_error),
            format!("{}: {} \"{}\"\n", PARSING_ERROR, INVALID_UINT_ERROR, value)
        );
    }
    #[test]
    fn test_from_str_cat_ok() {
        let value = "hello";
        let string_value = format!("{}:{}", CAT, value.to_string());
        let cat_value = ValueAttribute::from_str(&string_value).unwrap();
        assert_eq!(cat_value.to_string(), string_value);
    }
    #[test]
    fn test_display_cat() {
        let value = "hello";
        let string_value = format!("{}:{}", CAT, value);
        let display = ValueAttribute::Cat(value.to_string());
        assert_eq!(display.to_string(), string_value);
    }
    #[test]
    fn test_from_str_invalid_key_attribute_error() {
        let key = "top";
        let value = "hello";
        let string_value = format!("{}:{}", key, value);
        let key_error = ValueAttribute::from_str(&string_value).unwrap_err();
        assert_eq!(
            AttributeError::InvalidKeyAttribute(key.to_string()),
            key_error
        );
    }
    #[test]
    fn test_from_str_invalid_key_value_format_error() {
        let key = "top";
        let key_value_err = ValueAttribute::from_str(key).unwrap_err();
        assert_eq!(
            AttributeError::InvalidKeyValueFormat(key.to_string()),
            key_value_err
        );
        assert_eq!(
            format!("{}", key_value_err),
            format!(
                "{}: \"{}\" {}\n",
                ATTRIBUTE_ERROR, key, INVALID_KEY_VALUE_FORMAT_ERROR
            )
        );
    }
}
