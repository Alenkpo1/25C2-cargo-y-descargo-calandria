use crate::codec::h264::nalu_header::NaluHeader;

pub struct SingleNalUnitPacket {
    nalu_header: NaluHeader,
    payload: Vec<u8>,
}
impl SingleNalUnitPacket {
    pub fn new(nalu_header: NaluHeader, payload: Vec<u8>) -> Self {
        SingleNalUnitPacket {
            nalu_header,
            payload,
        }
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        let byte0 = self.nalu_header.write_byte();
        bytes.push(byte0);
        for b in &self.payload {
            bytes.push(*b);
        }
        bytes
    }
    pub fn read_bytes(bytes: &[u8]) -> SingleNalUnitPacket {
        let nalu_header = NaluHeader::read_byte(bytes[0]);
        let payload = bytes[1..].to_vec();
        SingleNalUnitPacket {
            nalu_header,
            payload,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn test_write_bytes_single_nal_unit_packet() {
        let nalu_header = NaluHeader::new(false, 3, 7);
        let payload = vec![10, 20, 30, 40];
        let single_nal = SingleNalUnitPacket::new(nalu_header, payload);
        let bytes = single_nal.write_bytes();
        assert_eq!(bytes[0], 103);
        assert_eq!(bytes[1..], [10, 20, 30, 40]);
    }
}
