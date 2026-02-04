use crate::protocols::sdp::property_attribute::PropertyAttribute;
use crate::protocols::sdp::sdp_consts::error_consts::{BOTH_ATTRIBUTE_NONE, BOTH_ATTRIBUTES_SOME};
use crate::protocols::sdp::sdp_consts::general_consts::{ATTRIBUTE_KEY, EQUAL_SYMBOL};
use crate::protocols::sdp::sdp_error::attribute_error::AttributeError;
use crate::protocols::sdp::value_attribute::ValueAttribute;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct Attribute {
    property_attribute: Option<PropertyAttribute>,
    value_attribute: Option<ValueAttribute>,
}
impl Attribute {
    pub fn new(
        property_attribute: Option<PropertyAttribute>,
        value_attribute: Option<ValueAttribute>,
    ) -> Self {
        Attribute {
            property_attribute,
            value_attribute,
        }
    }

    pub fn get_ice_ufrag(&self) -> Option<String> {
        match &self.value_attribute {
            Some(ValueAttribute::IceUfrag(ufrag)) => Some(ufrag.clone()),
            _ => None,
        }
    }

    pub fn get_ice_pwd(&self) -> Option<String> {
        match &self.value_attribute {
            Some(ValueAttribute::IcePwd(pwd)) => Some(pwd.clone()),
            _ => None,
        }
    }

    pub fn get_candidate(&self) -> Option<CandidateInfo> {
        match &self.value_attribute {
            Some(ValueAttribute::Candidate {
                foundation,
                component,
                protocol,
                priority,
                address,
                port,
                typ,
            }) => Some(CandidateInfo {
                foundation: *foundation,
                component: *component,
                protocol: protocol.clone(),
                priority: *priority,
                address: address.clone(),
                port: *port,
                typ: typ.clone(),
            }),
            _ => None,
        }
    }
    pub fn get_fingerprint(&self) -> Option<String> {
        match &self.value_attribute {
            // Devuelvo solo el hash
            Some(ValueAttribute::Fingerprint(_hash_func, fingerprint)) => Some(fingerprint.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateInfo {
    pub foundation: u32,
    pub component: u32,
    pub protocol: String,
    pub priority: u32,
    pub address: String,
    pub port: u32,
    pub typ: String,
}
impl fmt::Display for Attribute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.property_attribute, &self.value_attribute) {
            (Some(property_attribute), None) => {
                writeln!(f, "{}{}{}", ATTRIBUTE_KEY, EQUAL_SYMBOL, property_attribute)
            }
            (None, Some(value_attribute)) => {
                writeln!(f, "{}{}{}", ATTRIBUTE_KEY, EQUAL_SYMBOL, value_attribute)
            }
            (Some(_), Some(_)) => write!(f, "{}", BOTH_ATTRIBUTES_SOME),
            (None, None) => write!(f, "{}", BOTH_ATTRIBUTE_NONE),
        }
    }
}

impl FromStr for Attribute {
    type Err = AttributeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() < 2 || s[0..2] != format!("{}{}", ATTRIBUTE_KEY, EQUAL_SYMBOL) {
            return Err(AttributeError::InvalidAttributeFormat(s.to_string()));
        }

        if let Ok(property_attribute) = PropertyAttribute::from_str(s[2..].trim()) {
            return Ok(Attribute {
                property_attribute: Some(property_attribute),
                value_attribute: None,
            });
        } else if let Ok(value_attribute) = ValueAttribute::from_str(s[2..].trim()) {
            return Ok(Attribute {
                property_attribute: None,
                value_attribute: Some(value_attribute),
            });
        }
        Err(AttributeError::InvalidAttributeFormat(s.to_string()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::sdp_consts::error_consts::{
        ATTRIBUTE_ERROR, INVALID_ATTRIBUTE_FORMAT_ERROR,
    };
    #[test]
    fn test_from_str_property_attribute_ok() {
        let attribute_str = "a=recvonly\n";
        let attribute = Attribute::from_str(attribute_str).unwrap();
        assert!(attribute.property_attribute.is_some());
        assert!(attribute.value_attribute.is_none());
        assert_eq!(attribute.to_string(), attribute_str);
    }
    #[test]
    fn test_from_str_value_attribute_ok() {
        let attribute_str = "a=cat:hello\n";
        let attribute = Attribute::from_str(attribute_str).unwrap();
        assert!(attribute.property_attribute.is_none());
        assert!(attribute.value_attribute.is_some());
        assert_eq!(attribute.to_string(), attribute_str);
    }
    #[test]
    fn test_from_str_value_attribute_rtpmap_ok() {
        let attribute_str = "a=rtpmap:96 L8/8000\n";
        let attribute = Attribute::from_str(attribute_str).unwrap();

        assert!(attribute.property_attribute.is_none());
        assert!(attribute.value_attribute.is_some());
        assert_eq!(attribute.to_string(), "a=rtpmap:96 L8/8000\n");
    }
    #[test]
    fn test_from_str_value_attribute_fail() {
        let attribute_str = "s\n";
        let err = Attribute::from_str(attribute_str).unwrap_err();
        assert_eq!(
            AttributeError::InvalidAttributeFormat(attribute_str.to_string()),
            err
        );
        assert_eq!(
            format!("{}", err.to_string()),
            format!(
                "{}: {} \"{}\"\n",
                ATTRIBUTE_ERROR, INVALID_ATTRIBUTE_FORMAT_ERROR, attribute_str
            ),
        );
    }
    #[test]
    fn test_display_both_some_() {
        let property = PropertyAttribute::Recvonly;
        let value = ValueAttribute::Cat("hello".to_string());

        let attribute = Attribute {
            property_attribute: Some(property),
            value_attribute: Some(value),
        };

        assert_eq!(attribute.to_string(), format!("{}", BOTH_ATTRIBUTES_SOME));
    }
    #[test]
    fn test_display_both_none() {
        let attr = Attribute {
            property_attribute: None,
            value_attribute: None,
        };

        assert_eq!(attr.to_string(), BOTH_ATTRIBUTE_NONE);
    }
}
