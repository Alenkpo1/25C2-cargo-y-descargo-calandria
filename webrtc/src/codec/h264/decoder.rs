use opencv::{Result, imgproc, prelude::*};
use openh264::decoder::{DecodedYUV, Decoder};
use openh264::formats::YUVSource;

pub struct H264Decoder {
    decoder: Decoder,
}
impl H264Decoder {
    pub fn new() -> Result<H264Decoder, openh264::Error> {
        Ok(H264Decoder {
            decoder: Decoder::new()?,
        })
    }

    pub fn decode_yuv(&mut self, vec_codec: Vec<u8>) -> Option<DecodedYUV<'_>> {
        match self.decoder.decode(&vec_codec) {
            Ok(Some(decoded_yuv)) => Some(decoded_yuv),
            _ => None,
        }
    }
    pub fn yuv_to_bgr(decoded_yuv: &DecodedYUV) -> Result<Mat> {
        let (w, h) = decoded_yuv.dimensions();

        let mut rgb_bytes = vec![0u8; w * h * 3];
        decoded_yuv.write_rgb8(&mut rgb_bytes);
        let mat_rgb_flat = Mat::from_slice(&rgb_bytes)?;
        let mat_rgb = mat_rgb_flat.reshape(3, h as i32)?;
        let mut mat_bgr = Mat::default();
        imgproc::cvt_color(&mat_rgb, &mut mat_bgr, imgproc::COLOR_RGB2BGR, 0)?;
        Ok(mat_bgr)
    }
}
impl Default for H264Decoder {
    fn default() -> Self {
        H264Decoder::new().expect("openh264 decoder init")
    }
}
