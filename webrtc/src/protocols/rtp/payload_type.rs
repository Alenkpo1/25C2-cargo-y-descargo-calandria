use crate::protocols::rtp::constants::rtp_const::RTP_H264_TYPE;
use crate::protocols::rtp::h264_video_type::H264VideoType;
use crate::protocols::rtp::rtp_err::rtp_err::RtpError;

pub enum PayloadType {
    H264Video(H264VideoType),
}
impl PayloadType {
    pub fn write_bytes(&self) -> Vec<u8> {
        match self {
            PayloadType::H264Video(payload) => payload.write_bytes(),
        }
    }
    pub fn read_bytes(payload_number: u8, bytes: &[u8]) -> Result<PayloadType, RtpError> {
        match payload_number {
            RTP_H264_TYPE => {
                let payload = H264VideoType::read_bytes(bytes).map_err(RtpError::InvalidH264)?;
                Ok(PayloadType::H264Video(payload))
            }
            _ => Err(RtpError::InvalidRtpPayloadType(payload_number)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::rtp::rtp_err::rtp_err::RtpError;
    #[test]
    fn h264_payload_roundtrip() -> Result<(), RtpError> {
        let payload = PayloadType::H264Video(H264VideoType::Single(
            crate::codec::h264::single_nal_unit_packet::SingleNalUnitPacket::new(
                crate::codec::h264::nalu_header::NaluHeader::new(false, 0, 1),
                vec![0xAA],
            ),
        ));
        let bytes = payload.write_bytes();
        let parsed = PayloadType::read_bytes(RTP_H264_TYPE, &bytes)?;
        match parsed {
            PayloadType::H264Video(inner) => {
                let written = inner.write_bytes();
                assert_eq!(written, bytes);
            }
        }
        Ok(())
    }
}
