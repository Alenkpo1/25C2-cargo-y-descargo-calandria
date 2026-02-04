use crate::protocols::sdp::sdp_consts::general_consts::IN_STR;
use crate::protocols::sdp::sdp_error::net_type_error::NetTypeError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum NetType {
    In,
}
impl FromStr for NetType {
    type Err = NetTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            IN_STR => Ok(NetType::In),
            not_found => Err(NetTypeError::InvalidNetType(not_found.to_string())),
        }
    }
}

impl fmt::Display for NetType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetType::In => write!(f, "{}", IN_STR),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{INVALID_NET_TYPE_ERROR, NET_TYPE_ERROR};
    #[test]
    fn test_valid_from_str_net_type_in() {
        let net_type = NetType::from_str(IN_STR).unwrap();
        assert_eq!(net_type, NetType::In);
    }
    #[test]
    fn test_valid_to_string_net_type_in() {
        let net_type = NetType::In;
        assert_eq!(IN_STR, net_type.to_string());
    }
    #[test]
    fn test_invalid_net_type() {
        let net_type = NetType::from_str("JJ").unwrap_err();
        assert_eq!(NetTypeError::InvalidNetType("JJ".to_string()), net_type);
        assert_eq!(
            format!("{}", net_type),
            format!("{}: \"JJ\" {}\n", NET_TYPE_ERROR, INVALID_NET_TYPE_ERROR)
        );
    }
}
