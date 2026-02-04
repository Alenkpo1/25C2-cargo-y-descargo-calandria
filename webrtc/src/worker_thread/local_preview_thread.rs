use crate::worker_thread::error::worker_error::WorkerError;
use opencv::prelude::Mat;
use std::sync::mpsc::{Receiver, Sender};

pub struct PreviewThread {
    rx_bgr: Receiver<Mat>,
    tx_output: Sender<Mat>,
}

impl PreviewThread {
    pub fn new(rx_bgr: Receiver<Mat>, tx_output: Sender<Mat>) -> Self {
        PreviewThread { rx_bgr, tx_output }
    }

    pub fn run(&mut self) -> Result<(), WorkerError> {
        loop {
            let frame = match self.rx_bgr.recv() {
                Ok(f) => f,
                Err(_) => {
                    break;
                }
            };
            self.tx_output
                .send(frame)
                .map_err(|_| WorkerError::SendError)?;
        }
        Ok(())
    }
}
