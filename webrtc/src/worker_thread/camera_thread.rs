use crate::camera::camera_err::CameraError;
use crate::camera::camera_opencv::Camera;
use crate::worker_thread::error::worker_error::WorkerError;
use opencv::prelude::Mat;
use std::sync::mpsc::SyncSender;

pub struct CameraThread {
    tx_bgr: SyncSender<Mat>,
    tx_rgb: SyncSender<Mat>,
}
impl CameraThread {
    pub fn new(tx_bgr: SyncSender<Mat>, tx_rgb: SyncSender<Mat>) -> Self {
        CameraThread { tx_bgr, tx_rgb }
    }

    pub fn run(&mut self, camera: &mut Camera) -> Result<(), WorkerError> {
        loop {
            let frame_bgr = match camera.capture_frame() {
                Ok(f) => f,
                Err(CameraError::FrameEmpty) => {
                    // Salta frames vacíos sin terminar el hilo
                    continue;
                }
                Err(err) => return Err(WorkerError::CaptureFrameError(err)),
            };
            let frame_rgb =
                Camera::transform_frame_rgb(&frame_bgr).map_err(WorkerError::ConvertRgbFrame)?;
            self.tx_rgb
                .send(frame_rgb)
                .map_err(|_| WorkerError::SendError)?;
            self.tx_bgr
                .send(frame_bgr)
                .map_err(|_| WorkerError::SendError)?;
        }
    }
}
