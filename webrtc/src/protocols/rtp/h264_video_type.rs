use crate::codec::h264::fu_a::FragmentationUnitTypeA;
use crate::codec::h264::nalu_header::NaluHeader;
use crate::codec::h264::single_nal_unit_packet::SingleNalUnitPacket;
use crate::protocols::rtp::constants::rtp_const::FU_A_PAYLOAD_NUMBER;
use crate::protocols::rtp::rtp_err::h26_video_type_err::H26VideoTypeErr;

pub enum H264VideoType {
    Single(SingleNalUnitPacket),
    Fragmented(FragmentationUnitTypeA),
}
impl H264VideoType {
    pub fn write_bytes(&self) -> Vec<u8> {
        match self {
            H264VideoType::Single(single_nal) => single_nal.write_bytes(),
            H264VideoType::Fragmented(fu_a) => fu_a.write_bytes(),
        }
    }
    pub fn read_bytes(bytes: &[u8]) -> Result<H264VideoType, H26VideoTypeErr> {
        let nalu_header = NaluHeader::read_byte(bytes[0]);
        let nalu_type = nalu_header.get_nalu_type();
        match nalu_type {
            1..=23 => {
                let single = SingleNalUnitPacket::read_bytes(bytes);
                Ok(H264VideoType::Single(single))
            }
            FU_A_PAYLOAD_NUMBER => {
                let fu_a = FragmentationUnitTypeA::read_bytes(bytes);
                Ok(H264VideoType::Fragmented(fu_a))
            }
            _ => Err(H26VideoTypeErr::InvalidNalPayloadType(nalu_type)),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::h264::nalu_header::NaluHeader;
    #[test]
    fn test_write_bytes_h264_video_type_single_nal() {
        let header = NaluHeader::new(false, 2, 1);
        let payload = vec![222, 173, 190, 239];
        let single = SingleNalUnitPacket::new(header, payload);
        let video_type = H264VideoType::Single(single);

        let bytes = video_type.write_bytes();

        assert_eq!(bytes[0], 0b01000001);
        assert_eq!(&bytes[1..], [222, 173, 190, 239]);
    }
}
