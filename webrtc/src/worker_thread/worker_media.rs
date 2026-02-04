use crate::camera::camera_opencv::Camera;
use opencv::prelude::Mat;
use std::sync::{Arc, Mutex};

use crate::crypto::srtp::SrtpContext;
use crate::protocols::rtcp::rtcp_packet::RtcpPacket;
use crate::rtc::rtc_rtp::rtc_rtp_sender::RtcRtpSender;
use crate::rtc::socket::peer_socket::PeerSocket;
use crate::worker_thread::camera_thread::CameraThread;
use crate::worker_thread::decoder_thread::DecodeThread;
use crate::worker_thread::encode_thread::EncoderThread;
use crate::worker_thread::error::worker_error::WorkerError;
use crate::worker_thread::media_metrics::{CallMetricsSnapshot, MediaMetrics};
use crate::worker_thread::rtc_rtp_sender_thread::RtpSenderThread;
use crate::worker_thread::rtcp_reporter_thread::RtcpReporterThread;
use crate::worker_thread::rtp_receiver_thread::RtpReceiverThread;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;

const VIDEO_SSRC: u32 = 1000;
#[derive(Clone, Copy)]
pub struct VideoParams {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

pub struct WorkerMedia {
    rx_preview: Receiver<Mat>,
    rx_decoded: Receiver<Mat>,
    tx_incoming: SyncSender<Vec<u8>>,
    peer_socket: Arc<Mutex<PeerSocket>>,
    ssrc: u32,
    metrics: Arc<Mutex<MediaMetrics>>,
}

impl WorkerMedia {
    pub fn start(
        camera_index: i32,
        peer_socket: Arc<Mutex<PeerSocket>>,
        params: VideoParams,
        srtp_context: Option<SrtpContext>,
    ) -> Result<Self, WorkerError> {
        let (tx_bgr, rx_bgr) = mpsc::sync_channel(1);
        let (tx_rgb, rx_rgb) = mpsc::sync_channel::<Mat>(3);
        let (tx_encoded, rx_encoded) = mpsc::sync_channel::<Vec<u8>>(1);
        let (tx_rtp, rx_rtp) = mpsc::sync_channel::<Vec<u8>>(3);
        let (tx_incoming, rx_incoming) = mpsc::sync_channel::<Vec<u8>>(8);
        let (tx_decoded, rx_decoded) = mpsc::sync_channel::<Mat>(1);
        println!("DEBUG: WorkerMedia initializing camera...");
        let mut camera = match Camera::with_params(
            camera_index,
            params.width as f64,
            params.height as f64,
            params.fps as f64,
        ) {
            Ok(cam) => cam,
            Err(err) => {
                eprintln!(
                    "No se pudo abrir cámara con {}x{}@{}fps: {:?}. Intentando fallback...",
                    params.width, params.height, params.fps, err
                );
                Camera::new(camera_index).map_err(|_| WorkerError::SendError)?
            }
        };
        println!("DEBUG: Camera initialized successfully");
        let socket_for_rtp = Arc::clone(&peer_socket);
        let socket_for_rtcp = Arc::clone(&peer_socket);
        let metrics = Arc::new(Mutex::new(MediaMetrics::new(VIDEO_SSRC)));
        let sender_metrics = Arc::clone(&metrics);
        let receiver_metrics = Arc::clone(&metrics);
        let reporter_metrics = Arc::clone(&metrics);

        // Extract the raw SRTP key bytes
        let srtp_key_bytes = srtp_context.as_ref().map(|ctx| ctx.get_key().to_vec());

        let rtp_sender = RtcRtpSender::new(VIDEO_SSRC, sender_metrics, srtp_key_bytes);

        let mut camera_thread = CameraThread::new(tx_bgr, tx_rgb);
        thread::spawn(move || {
            if let Err(err) = camera_thread.run(&mut camera) {
                eprintln!("{:?}", err);
            }
        });

        let mut encode_thread =
            EncoderThread::new(rx_rgb, tx_encoded).map_err(|_| WorkerError::SendError)?;
        thread::spawn(move || {
            if let Err(err) = encode_thread.run() {
                eprintln!("{:?}", err);
            }
        });

        let mut rtp_thread = RtpSenderThread::new(rx_encoded, rtp_sender);
        thread::spawn(move || {
            if let Err(err) = rtp_thread.run(socket_for_rtp) {
                eprintln!("{:?}", err);
            }
        });

        let mut receiver_thread =
            RtpReceiverThread::new(rx_incoming, tx_rtp, receiver_metrics, srtp_context);
        thread::spawn(move || {
            if let Err(err) = receiver_thread.run() {
                eprintln!("{:?}", err);
            }
        });

        thread::spawn(move || {
            let mut reporter = RtcpReporterThread::new(reporter_metrics);
            if let Err(err) = reporter.run(socket_for_rtcp) {
                eprintln!("{:?}", err);
            }
        });

        let mut decode_thread = DecodeThread::new(rx_rtp, tx_decoded);
        thread::spawn(move || {
            if let Err(err) = decode_thread.run() {
                eprintln!("{:?}", err);
            }
        });
        Ok(Self {
            rx_preview: rx_bgr,
            rx_decoded,
            tx_incoming,
            peer_socket,
            ssrc: VIDEO_SSRC,
            metrics,
        })
    }

    pub fn get_preview_receiver(&self) -> &Receiver<Mat> {
        &self.rx_preview
    }

    pub fn get_decoded_receiver(&self) -> &Receiver<Mat> {
        &self.rx_decoded
    }

    pub fn incoming_sender(&self) -> SyncSender<Vec<u8>> {
        self.tx_incoming.clone()
    }

    pub fn metrics(&self) -> Arc<Mutex<MediaMetrics>> {
        Arc::clone(&self.metrics)
    }

    pub fn metrics_snapshot(&self) -> CallMetricsSnapshot {
        match self.metrics.lock() {
            Ok(m) => m.snapshot(),
            Err(err) => {
                eprintln!("metrics_snapshot: lock poisoned ({})", err);
                CallMetricsSnapshot::default()
            }
        }
    }

    pub fn send_rtcp_bye(&self) -> Result<(), WorkerError> {
        let packet = RtcpPacket::bye(self.ssrc);
        let bytes = packet.write_bytes();
        let socket = self
            .peer_socket
            .lock()
            .map_err(|_| WorkerError::SendError)?;
        socket.send(&bytes).map_err(|_| WorkerError::SendError)
    }
}
