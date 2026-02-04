use crate::codec::h264::fu_header::FuHeader;
use crate::codec::h264::nalu_header::NaluHeader;

pub struct FragmentationUnitTypeA {
    fu_indicator: NaluHeader,
    fu_header: FuHeader,
    payload: Vec<u8>,
}
impl FragmentationUnitTypeA {
    pub fn new(fu_indicator: NaluHeader, fu_header: FuHeader, payload: Vec<u8>) -> Self {
        FragmentationUnitTypeA {
            fu_indicator,
            fu_header,
            payload,
        }
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        let byte0 = self.fu_indicator.write_byte();
        bytes.push(byte0);
        let byte1 = self.fu_header.write_byte();
        bytes.push(byte1);
        for b in &self.payload {
            bytes.push(*b);
        }
        bytes
    }
    pub fn read_bytes(bytes: &[u8]) -> FragmentationUnitTypeA {
        let fu_indicator = NaluHeader::read_byte(bytes[0]);
        let fu_header = FuHeader::read_byte(bytes[1]);
        let payload = bytes[2..].to_vec();
        FragmentationUnitTypeA {
            fu_indicator,
            fu_header,
            payload,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_write_bytes_fu_a() {
        let fu_indicator = NaluHeader::new(false, 1, 28);
        let fu_header = FuHeader::new(true, false, false, 5);
        let payload = vec![10, 20, 30, 40];
        let fu_a = FragmentationUnitTypeA::new(fu_indicator, fu_header, payload);
        let bytes = fu_a.write_bytes();
        assert_eq!(bytes[0], 60);
        assert_eq!(bytes[1], 133);
        assert_eq!(&bytes[2..], [10, 20, 30, 40]);
    }
    #[test]
    fn test_read_bytes_fu_a() {
        let bytes: Vec<u8> = vec![0b00111100, 0b10000101, 10, 20, 30, 40];
        let fua = FragmentationUnitTypeA::read_bytes(&bytes);
        assert_eq!(fua.fu_indicator.write_byte(), 0b00111100);
        assert_eq!(fua.fu_header.write_byte(), 0b10000101);
        assert_eq!(fua.payload, vec![10, 20, 30, 40]);
    }
}
