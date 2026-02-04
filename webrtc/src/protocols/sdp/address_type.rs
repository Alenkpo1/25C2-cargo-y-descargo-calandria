use crate::protocols::sdp::sdp_consts::general_consts::{IP4_STR, IP6_STR};
use crate::protocols::sdp::sdp_error::address_type_error::AddressTypeError;
use std::fmt;
use std::str::FromStr;
#[derive(Debug, PartialEq)]
pub enum AddressType {
    IP4,
    IP6,
}
impl FromStr for AddressType {
    type Err = AddressTypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            IP4_STR => Ok(AddressType::IP4),
            IP6_STR => Ok(AddressType::IP6),
            not_found => Err(AddressTypeError::InvalidAddrType(not_found.to_string())),
        }
    }
}

impl fmt::Display for AddressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressType::IP4 => write!(f, "{}", IP4_STR),
            AddressType::IP6 => write!(f, "{}", IP6_STR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        ADDRESS_TYPE_ERROR, INVALID_ADDRESS_TYPE_ERROR,
    };

    #[test]
    fn test_valid_from_str_ip4() {
        let addr_type = AddressType::from_str(IP4_STR).unwrap();
        assert_eq!(addr_type, AddressType::IP4);
    }

    #[test]
    fn test_valid_from_str_ip6() {
        let addr_type = AddressType::from_str(IP6_STR).unwrap();
        assert_eq!(addr_type, AddressType::IP6);
    }

    #[test]
    fn test_valid_to_string_ip4() {
        let addr_type = AddressType::IP4;
        assert_eq!(addr_type.to_string(), IP4_STR);
    }
    #[test]
    fn test_valid_to_string_ip6() {
        let addr_type = AddressType::IP6;
        assert_eq!(addr_type.to_string(), IP6_STR);
    }

    #[test]
    fn test_invalid_addr_type() {
        let addr_type = AddressType::from_str("IP10").unwrap_err();
        assert_eq!(
            AddressTypeError::InvalidAddrType("IP10".to_string()),
            addr_type
        );
        assert_eq!(
            format!("{}", addr_type),
            format!(
                "{}: \"IP10\" {}\n",
                ADDRESS_TYPE_ERROR, INVALID_ADDRESS_TYPE_ERROR
            )
        );
    }
}
