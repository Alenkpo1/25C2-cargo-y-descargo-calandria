use crate::protocols::sdp::address_type::AddressType;
use crate::protocols::sdp::net_type::NetType;
use crate::protocols::sdp::sdp_consts::general_consts::{EQUAL_SYMBOL, ORIGIN_KEY};
use crate::protocols::sdp::sdp_error::origin_error::OriginError;
use crate::protocols::sdp::sdp_error::parse_error::ParsingError;
use std::fmt;
use std::str::FromStr;
#[derive(Debug)]
pub struct Origin {
    username: String,
    session_id: u32,
    session_version: u32,
    net_type: NetType,
    address_type: AddressType,
    address: String,
}
impl Origin {
    pub fn new(
        username: String,
        session_id: u32,
        session_version: u32,
        net_type: NetType,
        address_type: AddressType,
        address: String,
    ) -> Origin {
        Origin {
            username,
            session_id,
            session_version,
            net_type,
            address_type,
            address,
        }
    }
}
impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{}{}{} {} {} {} {} {}",
            ORIGIN_KEY,
            EQUAL_SYMBOL,
            self.username,
            self.session_id,
            self.session_version,
            self.net_type,
            self.address_type,
            self.address
        )
    }
}
impl FromStr for Origin {
    type Err = OriginError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec_origin: Vec<&str> = s.split_whitespace().collect();
        if vec_origin.len() != 6 || vec_origin[0].len() < 2 {
            return Err(OriginError::InvalidOriginLength(vec_origin.len()));
        }

        if s[0..2] != format!("{}{}", ORIGIN_KEY, EQUAL_SYMBOL) {
            return Err(OriginError::InvalidOriginKey(s[0..2].to_string()));
        }
        let username = vec_origin[0][2..].to_string();
        let session_id = vec_origin[1]
            .parse::<u32>()
            .map_err(|_| ParsingError::InvalidUint(vec_origin[1].to_string()))?;
        let session_version = vec_origin[2]
            .parse::<u32>()
            .map_err(|_| ParsingError::InvalidUint(vec_origin[2].to_string()))?;
        let net_type = NetType::from_str(vec_origin[3]).map_err(OriginError::OriginNetTypeError)?;
        let addr_type =
            AddressType::from_str(vec_origin[4]).map_err(OriginError::OriginAddressTypeError)?;
        let address: String = vec_origin[5].to_string();
        Ok(Origin::new(
            username,
            session_id,
            session_version,
            net_type,
            addr_type,
            address,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        ADDRESS_TYPE_ERROR, INVALID_ADDRESS_TYPE_ERROR, INVALID_NET_TYPE_ERROR,
        INVALID_ORIGIN_KEY_ERROR, INVALID_ORIGIN_LENGTH_ERROR, INVALID_UINT_ERROR, NET_TYPE_ERROR,
        ORIGIN_ERROR, PARSING_ERROR,
    };
    use crate::protocols::sdp::sdp_consts::general_consts::{IN_STR, IP4_STR};
    use crate::protocols::sdp::sdp_error::address_type_error::AddressTypeError;
    use crate::protocols::sdp::sdp_error::net_type_error::NetTypeError;
    #[test]
    fn test_convert_to_string() {
        let origin = Origin::new(
            "User1".to_string(),
            123,
            1,
            NetType::In,
            AddressType::IP4,
            "127.0.0.1".to_string(),
        );

        let sdp_line = format!("{}", origin);
        assert_eq!(
            sdp_line,
            format!(
                "{}{}User1 123 1 {} {} 127.0.0.1\n",
                ORIGIN_KEY, EQUAL_SYMBOL, IN_STR, IP4_STR
            )
        );
    }
    #[test]
    fn test_origin_from_str() {
        let input = "o=- 1234 5678 IN IP4 127.0.0.1";
        let origin = Origin::from_str(input).unwrap();

        assert_eq!(origin.username, "-");
        assert_eq!(origin.session_id, 1234);
        assert_eq!(origin.session_version, 5678);
        assert_eq!(origin.net_type, NetType::In);
        assert_eq!(origin.address_type, AddressType::IP4);
        assert_eq!(origin.address, "127.0.0.1");
    }
    #[test]
    fn test_from_str_length_error() {
        let origin_str = "o=- 1000 1 5678 IN IP4 157.2.2.1";
        let origin_vec: Vec<&str> = origin_str.split_whitespace().collect();
        let origin_err = Origin::from_str(origin_str).unwrap_err();
        assert_eq!(
            OriginError::InvalidOriginLength(origin_vec.len()),
            origin_err
        );
        assert_eq!(
            format!("{}", origin_err),
            format!(
                "{}: {} \"{}\"\n",
                ORIGIN_ERROR,
                INVALID_ORIGIN_LENGTH_ERROR,
                origin_vec.len()
            )
        );
    }
    #[test]
    fn test_from_str_key_error() {
        let origin_str = "P-=- 2234 1178 IN IP4 132.1.2.1";
        let _: Vec<&str> = origin_str.split_whitespace().collect();
        let origin_err = Origin::from_str(origin_str).unwrap_err();
        assert_eq!(
            OriginError::InvalidOriginKey(origin_str[0..2].to_string()),
            origin_err
        );
        assert_eq!(
            format!("{}", origin_err),
            format!(
                "{}: {} \"{}{}\" \"{}\"\n",
                ORIGIN_ERROR,
                INVALID_ORIGIN_KEY_ERROR,
                ORIGIN_KEY,
                EQUAL_SYMBOL,
                origin_str[0..2].to_string()
            )
        );
    }
    #[test]
    fn test_from_str_session_id_error() {
        let origin_str = "o=- as2 123 IN IP4 172.16.2.1";
        let origin_vec: Vec<&str> = origin_str.split_whitespace().collect();
        let origin_err = Origin::from_str(origin_str).unwrap_err();
        assert_eq!(
            OriginError::OriginParseError(ParsingError::InvalidUint(origin_vec[1].to_string())),
            origin_err
        );
        assert_eq!(
            format!("{}", origin_err),
            format!(
                "{}: {} \"{}\"\n",
                PARSING_ERROR,
                INVALID_UINT_ERROR,
                origin_vec[1].to_string()
            )
        );
    }
    #[test]
    fn test_from_str_session_version_error() {
        let origin_str = "o=- 1234 rock IN IP4 172.16.2.1";
        let origin_vec: Vec<&str> = origin_str.split_whitespace().collect();
        let origin_err = Origin::from_str(origin_str).unwrap_err();
        assert_eq!(
            OriginError::OriginParseError(ParsingError::InvalidUint(origin_vec[2].to_string())),
            origin_err
        );
        assert_eq!(
            format!("{}", origin_err),
            format!(
                "{}: {} \"{}\"\n",
                PARSING_ERROR,
                INVALID_UINT_ERROR,
                origin_vec[2].to_string()
            )
        );
    }
    #[test]
    fn test_from_str_net_type_error() {
        let origin_str = "o=- 1234 5678 TE IP4 172.16.2.1";
        let origin_vec: Vec<&str> = origin_str.split_whitespace().collect();
        let origin_err = Origin::from_str(origin_str).unwrap_err();
        assert_eq!(
            OriginError::OriginNetTypeError(NetTypeError::InvalidNetType(
                origin_vec[3].to_string()
            )),
            origin_err
        );
        assert_eq!(
            format!("{}", origin_err),
            format!(
                "{}: \"{}\" {}\n",
                NET_TYPE_ERROR,
                origin_vec[3].to_string(),
                INVALID_NET_TYPE_ERROR
            )
        )
    }
    #[test]
    fn test_from_str_address_error() {
        let origin_str = "o=- 1234 5678 IN IP2 172.16.2.1";
        let origin_vec: Vec<&str> = origin_str.split_whitespace().collect();
        let origin_err = Origin::from_str(origin_str).unwrap_err();
        assert_eq!(
            OriginError::OriginAddressTypeError(AddressTypeError::InvalidAddrType(
                origin_vec[4].to_string()
            )),
            origin_err
        );
        assert_eq!(
            format!("{}", origin_err),
            format!(
                "{}: \"{}\" {}\n",
                ADDRESS_TYPE_ERROR,
                origin_vec[4].to_string(),
                INVALID_ADDRESS_TYPE_ERROR
            )
        )
    }
}
