use crate::camera::camera_err::CameraError;
use opencv::videoio::VideoCapture;
use opencv::{imgproc, prelude::*, videoio};
// src/camera/camera_opencv.rs
//use opencv::prelude::*;
//use std::thread::sleep;
//use std::time::Duration;

pub struct Camera {
    video_capture: VideoCapture,
}

impl Camera {
    pub fn with_params(
        index: i32,
        width: f64,
        height: f64,
        fps: f64,
    ) -> std::result::Result<Camera, CameraError> {
        // CASE: WINDOWS
        // Intenta primero DSHOW
        #[cfg(target_os = "windows")]
        let backends = [
            (videoio::CAP_DSHOW, "DSHOW"),
            (videoio::CAP_MSMF, "MSMF"),
            (videoio::CAP_ANY, "ANY"),
        ];

        // CASE LINUX/UNIX
        #[cfg(not(target_os = "windows"))]
        let backends = [
            (videoio::CAP_V4L2, "V4L2"),
            (videoio::CAP_GSTREAMER, "GST"),
            (videoio::CAP_ANY, "ANY"),
        ];

        let candidates = [
            (width, height, fps),
            (640.0, 360.0, fps),
            (320.0, 240.0, (fps / 2.0).max(15.0)),
        ];

        for (backend_const, backend_name) in backends.iter() {
            eprintln!(
                "with_params -> intentando backend {} (code {})",
                backend_name, backend_const
            );

            let mut vc = match VideoCapture::new(index, *backend_const) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("  Error creando VideoCapture con {}: {:?}", backend_name, e);
                    continue;
                }
            };

            match vc.is_opened() {
                Ok(true) => { /* Sigue porque todo bien */ }
                Ok(false) => {
                    eprintln!("  VideoCapture error no quedó abierto {}", backend_name);
                    let _ = vc.release();
                    continue;
                }
                Err(e) => {
                    eprintln!("  is_opened() error for {}: {:?}", backend_name, e);
                    let _ = vc.release();
                    continue;
                }
            }

            for (w, h, f) in candidates.iter() {
                eprintln!("    Probando {}x{} @ {} fps", w, h, f);
                let _ = vc.set(videoio::CAP_PROP_FRAME_WIDTH, *w);
                let _ = vc.set(videoio::CAP_PROP_FRAME_HEIGHT, *h);
                let _ = vc.set(videoio::CAP_PROP_FPS, *f);
                let _ = vc.set(videoio::CAP_PROP_BUFFERSIZE, 1.0);

                // lets backend stabilize for a moment
                std::thread::sleep(std::time::Duration::from_millis(120));

                // reads priorities
                // WIP: clean unwraps
                let rw = vc.get(videoio::CAP_PROP_FRAME_WIDTH).unwrap_or(0.0);
                let rh = vc.get(videoio::CAP_PROP_FRAME_HEIGHT).unwrap_or(0.0);
                let rf = vc.get(videoio::CAP_PROP_FPS).unwrap_or(0.0);
                eprintln!("    Reportado por driver: {}x{} @ {} fps", rw, rh, rf);

                // Tries to read a frame
                let mut frame = opencv::prelude::Mat::default();
                match vc.read(&mut frame) {
                    Ok(_) => {
                        if frame.size().map(|s| s.width > 0).unwrap_or(false) {
                            eprintln!(
                                "    Read OK with backend {} and {}x{}@{}",
                                backend_name, rw, rh, rf
                            );
                            return Ok(Camera { video_capture: vc });
                        } else {
                            eprintln!(
                                "    Empty frame on backend {} (reported {}x{})",
                                backend_name, rw, rh
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("    Error reading frame with {}: {:?}", backend_name, e);
                    }
                }
            } // end candidatos

            let _ = vc.release();
            eprintln!(
                "  Backend {} did not work with tested resolutions. Trying next backend...",
                backend_name
            );
        } // end backends

        Err(CameraError::CameraCreationError(
            "Could not open camera with any backend/resolution".into(),
        ))
    }

    pub fn new(index: i32) -> std::result::Result<Camera, CameraError> {
        // reuses with_params with defaults
        Self::with_params(index, 1280.0, 720.0, 30.0).or_else(|_| {
            // fallback simple attempt with different backends
            #[cfg(target_os = "windows")]
            let backends = [
                (videoio::CAP_DSHOW, "DSHOW"),
                (videoio::CAP_MSMF, "MSMF"),
                (videoio::CAP_ANY, "ANY"),
            ];
            #[cfg(not(target_os = "windows"))]
            let backends = [
                (videoio::CAP_ANY, "ANY"),
                (videoio::CAP_V4L2, "V4L2"),
                (videoio::CAP_GSTREAMER, "GST"),
            ];

            for (backend, name) in backends.iter() {
                eprintln!(
                    "camera_opencv::new -> intentando backend {} (code {})",
                    name, backend
                );
                let mut vc = match VideoCapture::new(index, *backend) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("  Error creando VideoCapture con {}: {:?}", name, e);
                        continue;
                    }
                };

                if let Ok(true) = vc.is_opened() {
                    let mut frame = Mat::default();
                    if let Ok(_) = vc.read(&mut frame) {
                        if frame.size().map(|s| s.width > 0).unwrap_or(false) {
                            eprintln!("  Abierto OK con backend {}", name);
                            return Ok(Camera { video_capture: vc });
                        }
                    }
                }
                let _ = vc.release();
            }

            Err(CameraError::CameraCreationError(
                "Camera::new: no se pudo abrir cámara (fallback)".into(),
            ))
        })
    }

    /// Lee un frame (descarta frames viejos antes de read).
    pub fn capture_frame(&mut self) -> std::result::Result<Mat, CameraError> {
        let mut frame = Mat::default();
        for _ in 0..2 {
            let _ = self.video_capture.grab();
        }
        // read devuelve opencv::Result<bool>, así que lo mappeamos a CameraError
        self.video_capture.read(&mut frame).map_err(|e| {
            CameraError::ReadFrameError(format!("read error: code={} msg={}", e.code, e.message))
        })?;
        if frame.empty() {
            return Err(CameraError::FrameEmpty);
        }
        Ok(frame)
    }

    /// Convierte BGR -> RGB retornando nuevo Mat.
    pub fn transform_frame_rgb(bgr_frame: &Mat) -> std::result::Result<Mat, CameraError> {
        let mut rgb = Mat::default();
        imgproc::cvt_color(&bgr_frame, &mut rgb, imgproc::COLOR_BGR2RGB, 0).map_err(|e| {
            CameraError::BgrToRgbError(format!(
                "cvt_color error: code={} msg={}",
                e.code, e.message
            ))
        })?;
        Ok(rgb)
    }
}
