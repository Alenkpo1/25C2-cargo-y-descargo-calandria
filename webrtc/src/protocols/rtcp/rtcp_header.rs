pub struct RtcpHeader {
    version: u8,
    padding: bool,
    rc: u8,
    packet_type: u8,
    length: u16,
}
impl RtcpHeader {
    pub fn new(padding: bool, rc: u8, packet_type: u8, length: u16) -> RtcpHeader {
        RtcpHeader {
            version: 2,
            padding,
            rc,
            packet_type,
            length,
        }
    }

    pub fn write_bytes(&self) -> Vec<u8> {
        let byte0 = (self.version << 6) | ((self.padding as u8) << 5) | (self.rc & 0b00011111);
        let byte1 = self.packet_type;
        let byte2 = self.length.to_be_bytes();
        vec![byte0, byte1, byte2[0], byte2[1]]
    }
    pub fn read_bytes(protocol_bytes: &[u8]) -> RtcpHeader {
        let byte0 = protocol_bytes[0];
        let version = (byte0 >> 6) & 0b00000011;
        let padding = ((byte0 >> 5) & 0b00000001) != 0;
        let rc = byte0 & 0b00011111;

        let packet_type = protocol_bytes[1];
        let length = u16::from_be_bytes([protocol_bytes[2], protocol_bytes[3]]);
        RtcpHeader {
            version,
            padding,
            rc,
            packet_type,
            length,
        }
    }
    pub fn get_packet_type(&self) -> u8 {
        self.packet_type
    }
    pub fn get_report_count(&self) -> u8 {
        self.rc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtcp_header_write_bytes() {
        let header = RtcpHeader::new(true, 5, 200, 3000);

        let bytes = header.write_bytes();

        assert_eq!(bytes.len(), 4);

        assert_eq!(bytes[0], 165);

        assert_eq!(bytes[1], 200);

        assert_eq!([bytes[2], bytes[3]], 3000u16.to_be_bytes());

        let expected = [165, 200, 11, 184];
        assert_eq!(bytes, expected);
    }
    #[test]
    fn test_rtcp_header_read_bytes() {
        let bytes = [165, 200, 11, 184];

        let header = RtcpHeader::read_bytes(&bytes);
        assert_eq!(header.version, 2);
        assert_eq!(header.padding, true);
        assert_eq!(header.rc, 5);
        assert_eq!(header.packet_type, 200);
        assert_eq!(header.length, 3000);
    }
}
