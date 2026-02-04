use crate::protocols::rtcp::rtcp_const::rtp_controller_const::CNAME_TYPE;
use crate::protocols::rtcp::rtcp_err::rtcp_error::RtcpError;
use crate::protocols::rtcp::source_description_items::SdesItem;

pub enum SdesEnum {
    CName(SdesItem),
}

impl SdesEnum {
    pub fn write_bytes(&self) -> Vec<u8> {
        match self {
            SdesEnum::CName(item) => item.write_bytes(),
        }
    }
    pub fn read_bytes(bytes: &[u8]) -> Result<SdesEnum, RtcpError> {
        let item = SdesItem::read_bytes(bytes);
        let sdes_enum = match item.get_type() {
            CNAME_TYPE => SdesEnum::CName(item),
            _ => return Err(RtcpError::SdesEnumReadError(item.get_type())),
        };
        Ok(sdes_enum)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdes_item() {
        let item = SdesItem::new(1, "house".to_string());
        assert_eq!(item.get_type(), 1);
        assert_eq!(item.get_length(), 5);
        assert_eq!(item.get_value(), "house");
        let bytes = item.write_bytes();
        assert_eq!(bytes, vec![1, 5, b'h', b'o', b'u', b's', b'e']);
        let parsed = SdesItem::read_bytes(&bytes);
        assert_eq!(parsed.get_type(), 1);
        assert_eq!(parsed.get_length(), 5);
        assert_eq!(parsed.get_value(), "house");
        assert_eq!(item.get_value(), parsed.get_value());
    }
}
