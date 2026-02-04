pub struct NaluHeader {
    forbidden_zero_bit: bool,
    nri: u8,
    nalu_type: u8,
}

impl NaluHeader {
    pub fn new(forbidden_zero_bit: bool, nri: u8, nalu_type: u8) -> Self {
        NaluHeader {
            forbidden_zero_bit,
            nri,
            nalu_type,
        }
    }

    pub fn write_byte(&self) -> u8 {
        ((self.forbidden_zero_bit as u8) << 7)
            | ((self.nri & 0b00000011) << 5)
            | (self.nalu_type & 0b00011111)
    }

    pub fn read_byte(byte: u8) -> NaluHeader {
        NaluHeader {
            forbidden_zero_bit: ((byte >> 7) & 0b00000001) != 0,
            nri: (byte >> 5) & 0b00000011,
            nalu_type: byte & 0b00011111,
        }
    }
    pub fn get_nalu_type(&self) -> u8 {
        self.nalu_type
    }
    pub fn get_forbidden_zero_bit(&self) -> bool {
        self.forbidden_zero_bit
    }
    pub fn get_nri(&self) -> u8 {
        self.nri
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_write_byte() {
        let header = NaluHeader {
            forbidden_zero_bit: false,
            nri: 0,
            nalu_type: 1,
        };
        assert_eq!(header.write_byte(), 0b00000001);
    }
    #[test]
    fn test_read_byte() {
        let byte = 0b01000101;
        let nalu = NaluHeader::read_byte(byte);
        assert_eq!(nalu.forbidden_zero_bit, false);
        assert_eq!(nalu.nri, 2);
        assert_eq!(nalu.nalu_type, 5);
    }
}
