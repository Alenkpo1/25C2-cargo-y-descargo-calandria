use crate::codec::h264::h264_err::encoder_err::EncoderError;
use opencv::{Result, prelude::*};
use openh264::OpenH264API;
use openh264::encoder::{
    BitRate, EncodedBitStream, Encoder, EncoderConfig, FrameRate, IntraFramePeriod, Profile,
    RateControlMode, SpsPpsStrategy, UsageType,
};
use openh264::formats::{RgbSliceU8, YUVBuffer};

pub struct H264Encoder {
    encoder: Encoder,
}
impl H264Encoder {
    pub fn new() -> Result<H264Encoder, EncoderError> {
        let api = OpenH264API::from_source();

        let config = EncoderConfig::new()
            .bitrate(BitRate::from_bps(2_000_000))
            .max_frame_rate(FrameRate::from_hz(30.0))
            .usage_type(UsageType::CameraVideoRealTime)
            .rate_control_mode(RateControlMode::Bitrate)
            .profile(Profile::Baseline)
            .sps_pps_strategy(SpsPpsStrategy::IncreasingId)
            .intra_frame_period(IntraFramePeriod::from_num_frames(30));

        let encoder =
            Encoder::with_api_config(api, config).map_err(EncoderError::CreateEncoderErr)?;

        Ok(H264Encoder { encoder })
    }
    pub fn encode_frame_yuv(
        &mut self,
        yuv: YUVBuffer,
    ) -> Result<EncodedBitStream<'_>, EncoderError> {
        let bitstream = self
            .encoder
            .encode(&yuv)
            .map_err(EncoderError::EncodeError)?;
        Ok(bitstream)
    }
    pub fn rgb_to_yuv(rgb: &Mat) -> Result<YUVBuffer> {
        let rgb_bytes = rgb.data_bytes()?;
        let width = rgb.cols() as usize;
        let height = rgb.rows() as usize;
        let rgb_slice = RgbSliceU8::new(rgb_bytes, (width, height));
        let yuv = YUVBuffer::from_rgb8_source(rgb_slice);
        Ok(yuv)
    }
    pub fn split_nal(bytes: Vec<u8>) -> Vec<Vec<u8>> {
        bytes.chunks(900).map(|chunk| chunk.to_vec()).collect()
    }
    pub fn split_by_startcode(data: &[u8]) -> Vec<Vec<u8>> {
        let mut nalus = Vec::new();
        let mut start = 0;

        for i in 0..data.len().saturating_sub(3) {
            if data[i..i + 4] == [0, 0, 0, 1] {
                if i > start {
                    nalus.push(data[start..i].to_vec());
                }
                start = i + 4;
            }
        }
        if start < data.len() {
            nalus.push(data[start..].to_vec());
        }

        nalus
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::h264::nalu_header::NaluHeader;

    #[test]
    fn test_split_by_startcode_multiple_nalus() {
        let data = vec![
            0, 0, 0, 1, 103, 66, 192, 31, 140, 104, 5, 0, 91, 160, 30, 17, 8, 212, 0, 0, 0, 1, 104,
            206, 60, 128, 0, 0, 0, 1, 101, 184, 0, 4, 19, 255, 255, 225, 56, 160, 0, 32, 39, 127,
            123, 239,
        ];

        let nalus = H264Encoder::split_by_startcode(&data);

        assert_eq!(nalus.len(), 3);
        assert_eq!(nalus[0][0], 0x67);
        assert_eq!(nalus[1][0], 0x68);
        assert_eq!(nalus[2][0], 0x65);

        let sps_header = NaluHeader::read_byte(nalus[0][0]);
        let pps_header = NaluHeader::read_byte(nalus[1][0]);
        let idr_header = NaluHeader::read_byte(nalus[2][0]);

        assert_eq!(sps_header.get_forbidden_zero_bit(), false);
        assert_eq!(sps_header.get_nri(), 3);
        assert_eq!(sps_header.get_nalu_type(), 7);

        assert_eq!(pps_header.get_forbidden_zero_bit(), false);
        assert_eq!(pps_header.get_nri(), 3);
        assert_eq!(pps_header.get_nalu_type(), 8);

        assert_eq!(idr_header.get_forbidden_zero_bit(), false);
        assert_eq!(idr_header.get_nri(), 3);
        assert_eq!(idr_header.get_nalu_type(), 5);
    }
    #[test]
    fn test_split_by_startcode_single_nalu() {
        let data = vec![
            0, 0, 0, 1, 101, 184, 0, 4, 19, 255, 255, 225, 56, 160, 0, 32, 39, 127, 123, 239,
        ];

        let nalus = H264Encoder::split_by_startcode(&data);

        assert_eq!(nalus.len(), 1);
        assert_eq!(nalus[0][0], 0x65);
    }

    #[test]
    fn test_split_by_startcode_empty_input() {
        let data = vec![];
        let nalus = H264Encoder::split_by_startcode(&data);
        assert!(nalus.is_empty());
    }
}
