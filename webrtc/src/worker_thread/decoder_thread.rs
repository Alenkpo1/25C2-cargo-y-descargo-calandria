use crate::codec::h264::decoder::H264Decoder;
use crate::worker_thread::error::worker_error::WorkerError;
use opencv::prelude::Mat;
use std::sync::mpsc::{Receiver, SyncSender};

pub struct DecodeThread {
    rx_encoded: Receiver<Vec<u8>>,
    tx_frame: SyncSender<Mat>,
    decoder: H264Decoder,
}
impl DecodeThread {
    pub fn new(rx_encoded: Receiver<Vec<u8>>, tx_frame: SyncSender<Mat>) -> Self {
        let decoder = H264Decoder::new().unwrap_or_else(|err| {
            panic!("No se pudo iniciar decodificador H264: {}", err);
        });
        Self {
            rx_encoded,
            tx_frame,
            decoder,
        }
    }
    pub fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            let encoded_bytes = match self.rx_encoded.recv() {
                Ok(data) => data,
                Err(_) => {
                    eprintln!("DecodeThread Close Channel");
                    break;
                }
            };

            let decoder = &mut self.decoder;
            if let Some(decoded_yuv) = decoder.decode_yuv(encoded_bytes) {
                match H264Decoder::yuv_to_bgr(&decoded_yuv) {
                    Ok(frame_bgr) => {
                        self.tx_frame
                            .send(frame_bgr)
                            .map_err(|_| WorkerError::SendError)?;
                    }
                    Err(err) => {
                        eprintln!("DecodeThread: error to convert to RGB: {:?}", err);
                        continue;
                    }
                }
            }
        }
        Ok(())
    }
}
