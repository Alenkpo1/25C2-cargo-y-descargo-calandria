pub struct RtpHeader {
    version: u8,
    padding: bool,
    extension: bool,
    csrc_count: u8,
    marker: bool,
    payload_type: u8,
    sequence_number: u16,
    timestamp: u32,
    ssrc: u32,
    csrc_list: Vec<u32>,
}
impl RtpHeader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version: u8,
        padding: bool,
        extension: bool,
        csrc_count: u8,
        marker: bool,
        payload_type: u8,
        sequence_number: u16,
        timestamp: u32,
        ssrc: u32,
        csrc_list: Vec<u32>,
    ) -> Self {
        RtpHeader {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            csrc_list,
        }
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut protocol = Vec::new();

        let byte0 = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (self.csrc_count & 0b00001111);
        protocol.push(byte0);
        let byte1 = ((self.marker as u8) << 7) | (self.payload_type & 0x7F);
        protocol.push(byte1);
        let byte2 = self.sequence_number.to_be_bytes();
        add_vec_bytes(&byte2, &mut protocol);
        let byte3 = self.timestamp.to_be_bytes();
        add_vec_bytes(&byte3, &mut protocol);
        let byte4 = self.ssrc.to_be_bytes();
        add_vec_bytes(&byte4, &mut protocol);
        for csrc in &self.csrc_list {
            let byte = csrc.to_be_bytes();
            add_vec_bytes(&byte, &mut protocol);
        }
        protocol
    }
    pub fn read_bytes(protocol_bytes: &[u8]) -> (Self, usize) {
        let byte0 = protocol_bytes[0];
        let version = (byte0 >> 6) & 0b00000011;
        let padding = ((byte0 >> 5) & 0b1) != 0;
        let extension = ((byte0 >> 4) & 0b1) != 0;
        let csrc_count = byte0 & 0b00001111;
        let byte1 = protocol_bytes[1];
        let marker = byte1 >> 7 != 0;
        let payload_type = byte1 & 0b01111111;
        let sequence_number = u16::from_be_bytes([protocol_bytes[2], protocol_bytes[3]]);
        let timestamp = u32::from_be_bytes([
            protocol_bytes[4],
            protocol_bytes[5],
            protocol_bytes[6],
            protocol_bytes[7],
        ]);
        let ssrc = u32::from_be_bytes([
            protocol_bytes[8],
            protocol_bytes[9],
            protocol_bytes[10],
            protocol_bytes[11],
        ]);
        let mut csrc_list = Vec::new();
        let header_size = 12 + (csrc_count as usize) * 4;
        for i in 0..csrc_count {
            let start = 12 + (i as usize) * 4;
            let csrc = u32::from_be_bytes([
                protocol_bytes[start],
                protocol_bytes[start + 1],
                protocol_bytes[start + 2],
                protocol_bytes[start + 3],
            ]);
            csrc_list.push(csrc);
        }
        (
            RtpHeader {
                version,
                padding,
                extension,
                csrc_count,
                marker,
                payload_type,
                sequence_number,
                timestamp,
                ssrc,
                csrc_list,
            },
            header_size,
        )
    }
    pub fn get_payload_type(&self) -> u8 {
        self.payload_type
    }
    pub fn get_sequence_number(&self) -> u16 {
        self.sequence_number
    }
    pub fn get_timestamp(&self) -> u32 {
        self.timestamp
    }
    pub fn get_marker(&self) -> bool {
        self.marker
    }
    pub fn get_ssrc(&self) -> u32 {
        self.ssrc
    }
}

fn add_vec_bytes(bytes: &[u8], protocol: &mut Vec<u8>) {
    for &b in bytes {
        protocol.push(b);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtp_header_to_bytes() {
        let header = RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: true,
            payload_type: 10,
            sequence_number: 4660,
            timestamp: 1450744508,
            ssrc: 3735928559,
            csrc_list: vec![],
        };
        let bytes = header.write_bytes();
        assert_eq!(bytes.len(), 12);
        assert_eq!(bytes[0], 0b10000000);
        assert_eq!(bytes[1], 0b10001010);
        assert_eq!(bytes[2..4], [18, 52]);
        assert_eq!(bytes[4..8], [86, 120, 154, 188]);
        assert_eq!(bytes[8..12], [222, 173, 190, 239]);
    }

    #[test]
    fn test_add_vec_bytes() {
        let mut protocol = Vec::new();
        let data: [u8; 4] = [18, 52, 86, 120];
        add_vec_bytes(&data, &mut protocol);
        assert_eq!(protocol.len(), 4);
        assert_eq!(protocol, vec![18, 52, 86, 120]);
    }
    #[test]
    fn test_rtp_header_read_write_bytes() {
        let original = RtpHeader {
            version: 2,
            padding: true,
            extension: false,
            csrc_count: 2,
            marker: true,
            payload_type: 96,
            sequence_number: 23,
            timestamp: 100,
            ssrc: 25,
            csrc_list: vec![122, 125],
        };

        let bytes = original.write_bytes();

        let (parsed, _) = RtpHeader::read_bytes(&bytes);

        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.padding, original.padding);
        assert_eq!(parsed.extension, original.extension);
        assert_eq!(parsed.csrc_count, original.csrc_count);
        assert_eq!(parsed.marker, original.marker);
        assert_eq!(parsed.payload_type, original.payload_type);
        assert_eq!(parsed.sequence_number, original.sequence_number);
        assert_eq!(parsed.timestamp, original.timestamp);
        assert_eq!(parsed.ssrc, original.ssrc);
        assert_eq!(parsed.csrc_list, original.csrc_list);
    }

    #[test]
    fn roundtrip_accessors() {
        let header = RtpHeader::new(2, true, true, 1, false, 33, 7, 55, 999, vec![42]);
        assert_eq!(header.get_payload_type(), 33);
        assert_eq!(header.get_sequence_number(), 7);
        assert_eq!(header.get_timestamp(), 55);
        assert!(!header.get_marker());
        assert_eq!(header.get_ssrc(), 999);
    }
}
