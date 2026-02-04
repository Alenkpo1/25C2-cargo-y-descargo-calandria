use crate::protocols::rtp::payload_type::PayloadType;
use crate::protocols::rtp::rtp_err::rtp_err::RtpError;
use crate::protocols::rtp::rtp_header::RtpHeader;

pub struct RtpPacket {
    pub rtp_header: RtpHeader,
    pub payload: PayloadType,
}

impl RtpPacket {
    pub fn new(rtp_header: RtpHeader, payload: PayloadType) -> RtpPacket {
        RtpPacket {
            rtp_header,
            payload,
        }
    }
    pub fn write_bytes(&self) -> Vec<u8> {
        let mut bytes = self.rtp_header.write_bytes();
        bytes.extend_from_slice(&self.payload.write_bytes());
        bytes
    }
    pub fn read_bytes(bytes: &[u8]) -> Result<RtpPacket, RtpError> {
        let (rtp_header, header_size) = RtpHeader::read_bytes(bytes);
        let payload_bytes = &bytes[header_size..];
        let payload = PayloadType::read_bytes(rtp_header.get_payload_type(), payload_bytes)?;
        Ok(RtpPacket {
            rtp_header,
            payload,
        })
    }
    pub fn get_payload(self) -> PayloadType {
        self.payload
    }
    pub fn get_payload_bytes(&self) -> Vec<u8> {
        self.payload.write_bytes()
    }
    pub fn get_marker(&self) -> bool {
        self.rtp_header.get_marker()
    }
    pub fn get_sequence_number(&self) -> u16 {
        self.rtp_header.get_sequence_number()
    }
    pub fn get_timestamp(&self) -> u32 {
        self.rtp_header.get_timestamp()
    }
    pub fn get_ssrc(&self) -> u32 {
        self.rtp_header.get_ssrc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::h264::nalu_header::NaluHeader;
    use crate::codec::h264::single_nal_unit_packet::SingleNalUnitPacket;
    use crate::protocols::rtp::constants::rtp_const::RTP_H264_TYPE;
    use crate::protocols::rtp::h264_video_type::H264VideoType;

    fn sample_packet() -> RtpPacket {
        let nalu_header = NaluHeader::new(false, 3, 7);
        let payload = PayloadType::H264Video(H264VideoType::Single(SingleNalUnitPacket::new(
            nalu_header,
            vec![1, 2, 3],
        )));
        let header = RtpHeader::new(2, false, false, 0, true, RTP_H264_TYPE, 10, 20, 30, vec![]);
        RtpPacket::new(header, payload)
    }

    #[test]
    fn rtp_packet_roundtrip() {
        let packet = sample_packet();
        let bytes = packet.write_bytes();
        let parsed = RtpPacket::read_bytes(&bytes).expect("parse rtp");
        assert_eq!(parsed.get_sequence_number(), 10);
        assert_eq!(parsed.get_timestamp(), 20);
        assert_eq!(parsed.get_ssrc(), 30);
        assert!(parsed.get_marker());
        assert_eq!(parsed.get_payload_bytes()[0], 103);
    }
}
