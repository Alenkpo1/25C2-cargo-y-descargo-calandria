pub struct FuHeader {
    start_bit: bool,
    end_bit: bool,
    reserved: bool,
    payload_type: u8,
}
impl FuHeader {
    pub fn new(start_bit: bool, end_bit: bool, reserved: bool, payload_type: u8) -> Self {
        FuHeader {
            start_bit,
            end_bit,
            reserved,
            payload_type,
        }
    }
    pub fn write_byte(&self) -> u8 {
        ((self.start_bit as u8) << 7)
            | ((self.end_bit as u8) << 6)
            | ((self.reserved as u8) << 5)
            | (self.payload_type & 0b00011111)
    }
    pub fn read_byte(byte: u8) -> FuHeader {
        FuHeader {
            start_bit: ((byte >> 7) & 0b00000001) != 0,
            end_bit: ((byte >> 6) & 0b00000001) != 0,
            reserved: ((byte >> 5) & 0b0000001) != 0,
            payload_type: byte & 0b00011111,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_write_byte_fu_header() {
        let fu_header = FuHeader::new(true, false, false, 28);
        assert_eq!(fu_header.write_byte(), 0b10011100);
    }
    #[test]
    fn test_read_byte_fu_header() {
        let byte = 0b10011100;
        let fu_header = FuHeader::read_byte(byte);
        assert_eq!(fu_header.start_bit, true);
        assert_eq!(fu_header.end_bit, false);
        assert_eq!(fu_header.reserved, false);
        assert_eq!(fu_header.payload_type, 28);
    }
}
