use crate::codec::h264::encoder::H264Encoder;
use crate::worker_thread::error::worker_error::WorkerError;
use opencv::prelude::Mat;
use std::sync::mpsc::{Receiver, SyncSender};

pub struct EncoderThread {
    rx_rgb: Receiver<Mat>,
    tx_encoded: SyncSender<Vec<u8>>,
    encoder: H264Encoder,
}
impl EncoderThread {
    pub fn new(
        rx_rgb: Receiver<Mat>,
        tx_encoded: SyncSender<Vec<u8>>,
    ) -> Result<Self, WorkerError> {
        let encoder = H264Encoder::new().map_err(|_| WorkerError::SendError)?;
        Ok(Self {
            rx_rgb,
            tx_encoded,
            encoder,
        })
    }
    pub fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            let frame = match self.rx_rgb.recv() {
                Ok(f) => f,
                Err(_) => {
                    break;
                }
            };
            let yuv = H264Encoder::rgb_to_yuv(&frame).map_err(WorkerError::ConvertToYuvError)?;
            let bitstream = self
                .encoder
                .encode_frame_yuv(yuv)
                .map_err(WorkerError::InvalidEncoding)?;
            let encoded_bytes = bitstream.to_vec();
            self.tx_encoded
                .send(encoded_bytes)
                .map_err(|_| WorkerError::SendError)?;
        }
        Ok(())
    }
}
