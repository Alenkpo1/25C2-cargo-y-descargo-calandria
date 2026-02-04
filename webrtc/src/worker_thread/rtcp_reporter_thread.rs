use crate::protocols::rtcp::rtcp_const::rtp_controller_const::{
    RECEIVER_REPORT_TYPE, SENDER_REPORT_TYPE,
};
use crate::protocols::rtcp::rtcp_packet::RtcpPacket;
use crate::protocols::rtcp::rtcp_payload::RtcpPayload;
use crate::rtc::socket::peer_socket::PeerSocket;
use crate::worker_thread::error::worker_error::WorkerError;
use crate::worker_thread::media_metrics::{MediaMetrics, system_time_to_ntp};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

pub struct RtcpReporterThread {
    metrics: Arc<Mutex<MediaMetrics>>,
    interval: Duration,
}

impl RtcpReporterThread {
    pub fn new(metrics: Arc<Mutex<MediaMetrics>>) -> Self {
        Self {
            metrics,
            interval: Duration::from_secs(1),
        }
    }

    pub fn run(&mut self, peer_socket: Arc<Mutex<PeerSocket>>) -> Result<(), WorkerError> {
        loop {
            thread::sleep(self.interval);
            let now = system_time_to_ntp(SystemTime::now());

            let (sender_report, receiver_report) = {
                let mut guard = self.metrics.lock().map_err(|_| WorkerError::SendError)?;
                (
                    guard.build_sender_report(now),
                    guard.build_receiver_report(),
                )
            };

            if sender_report.is_none() && receiver_report.is_none() {
                continue;
            }

            let socket = peer_socket.lock().map_err(|_| WorkerError::SendError)?;

            if let Some(sr) = sender_report {
                let packet = RtcpPacket::from_payload(
                    SENDER_REPORT_TYPE,
                    sr.report_blocks.len() as u8,
                    RtcpPayload::SenderReport(sr),
                );
                let bytes = packet.write_bytes();
                socket.send(&bytes).map_err(|_| WorkerError::SendError)?;
            }

            if let Some(rr) = receiver_report {
                let packet = RtcpPacket::from_payload(
                    RECEIVER_REPORT_TYPE,
                    rr.report_blocks.len() as u8,
                    RtcpPayload::ReceiverReport(rr),
                );
                let bytes = packet.write_bytes();
                socket.send(&bytes).map_err(|_| WorkerError::SendError)?;
            }
        }
    }
}
