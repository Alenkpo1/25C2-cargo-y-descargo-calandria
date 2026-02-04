use crate::protocols::sdp::sdp_consts::general_consts::{INACTIVE, RECVONLY, SENDONLY, SENDRECV};
use crate::protocols::sdp::sdp_error::attribute_error::AttributeError;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum PropertyAttribute {
    Recvonly,
    Sendrecv,
    SendOnly,
    Inactive,
}

impl FromStr for PropertyAttribute {
    type Err = AttributeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            RECVONLY => Ok(PropertyAttribute::Recvonly),
            SENDRECV => Ok(PropertyAttribute::Sendrecv),
            SENDONLY => Ok(PropertyAttribute::SendOnly),
            INACTIVE => Ok(PropertyAttribute::Inactive),
            not_found => Err(AttributeError::InvalidKeyAttribute(not_found.to_string())),
        }
    }
}

impl fmt::Display for PropertyAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyAttribute::Recvonly => write!(f, "{}", RECVONLY),
            PropertyAttribute::Sendrecv => write!(f, "{}", SENDRECV),
            PropertyAttribute::SendOnly => write!(f, "{}", SENDONLY),
            PropertyAttribute::Inactive => write!(f, "{}", INACTIVE),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        ATTRIBUTE_ERROR, INVALID_KEY_ATTRIBUTE_ERROR,
    };
    #[test]
    fn test_from_str_property_attribute_recvonly() {
        let property_attribute = PropertyAttribute::from_str(RECVONLY).unwrap();
        assert_eq!(property_attribute, PropertyAttribute::Recvonly);
        assert_eq!(PropertyAttribute::Recvonly.to_string(), RECVONLY);
    }
    #[test]
    fn test_from_str_property_attribute_sendrecv() {
        let property_attribute = PropertyAttribute::from_str(SENDRECV).unwrap();
        assert_eq!(property_attribute, PropertyAttribute::Sendrecv);
        assert_eq!(PropertyAttribute::Sendrecv.to_string(), SENDRECV);
    }
    #[test]
    fn test_from_str_property_attribute_sendonly() {
        let property_attribute = PropertyAttribute::from_str(SENDONLY).unwrap();
        assert_eq!(property_attribute, PropertyAttribute::SendOnly);
        assert_eq!(PropertyAttribute::SendOnly.to_string(), SENDONLY);
    }

    #[test]
    fn test_from_str_property_attribute_inactive() {
        let property_attribute = PropertyAttribute::from_str(INACTIVE).unwrap();
        assert_eq!(property_attribute, PropertyAttribute::Inactive);
        assert_eq!(PropertyAttribute::Inactive.to_string(), INACTIVE);
    }
    #[test]
    fn test_from_str_property_attribute_error() {
        let property_attribute = PropertyAttribute::from_str("hello").unwrap_err();
        assert_eq!(
            AttributeError::InvalidKeyAttribute("hello".to_string()),
            property_attribute
        );
        assert_eq!(
            format!("{}", property_attribute),
            format!(
                "{}: \"hello\" {}\n",
                ATTRIBUTE_ERROR, INVALID_KEY_ATTRIBUTE_ERROR
            )
        );
    }
}
