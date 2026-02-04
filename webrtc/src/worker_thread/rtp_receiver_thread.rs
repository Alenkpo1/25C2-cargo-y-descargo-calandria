use crate::crypto::srtp::SrtpContext;
use crate::protocols::rtcp::rtcp_packet::RtcpPacket;
use crate::protocols::rtcp::rtcp_payload::RtcpPayload;
use crate::protocols::rtp::rtp_packet::RtpPacket;
use crate::rtc::jitter_buffer::j_buffer::JitterBuffer;
use crate::worker_thread::error::worker_error::WorkerError;
use crate::worker_thread::media_metrics::MediaMetrics;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct RtpReceiverThread {
    rx_socket: Receiver<Vec<u8>>,
    tx_decoded: SyncSender<Vec<u8>>,
    jitter: JitterBuffer,
    metrics: Arc<Mutex<MediaMetrics>>,
    srtp: Option<SrtpContext>,
}

impl RtpReceiverThread {
    pub fn new(
        rx_socket: Receiver<Vec<u8>>,
        tx_decoded: SyncSender<Vec<u8>>,
        metrics: Arc<Mutex<MediaMetrics>>,
        srtp_context: Option<SrtpContext>,
    ) -> Self {
        Self {
            rx_socket,
            tx_decoded,
            jitter: JitterBuffer::new(),
            metrics,
            srtp: srtp_context,
        }
    }
    pub fn run(&mut self) -> Result<(), WorkerError> {
        while let Ok(bytes) = self.rx_socket.recv() {
            if Self::is_rtcp(&bytes) {
                self.handle_rtcp(&bytes, Instant::now());
                continue;
            }

            let plain_bytes = if let Some(ref srtp) = self.srtp {
                match Self::decrypt_rtp(&bytes, srtp) {
                    Some(p) => p,
                    None => continue,
                }
            } else {
                bytes
            };

            let arrival = Instant::now();
            let rtp_packet = match RtpPacket::read_bytes(&plain_bytes) {
                Ok(rtp_packet) => rtp_packet,
                Err(_) => {
                    continue;
                }
            };

            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.update_receiver_on_rtp(&rtp_packet, arrival);
            }

            self.jitter.push(rtp_packet);

            if let Some(mut frame) = self.jitter.pop() {
                let full_bytes = frame.to_bytes();
                self.tx_decoded
                    .send(full_bytes)
                    .map_err(|_| WorkerError::SendError)?;
            }
        }

        Ok(())
    }

    fn is_rtcp(bytes: &[u8]) -> bool {
        bytes.get(1).is_some_and(|pt| (200..=204).contains(pt))
    }

    fn handle_rtcp(&self, bytes: &[u8], arrival: Instant) {
        if let Ok(packet) = RtcpPacket::read_bytes(bytes) {
            match packet.payload {
                RtcpPayload::SenderReport(sr) => {
                    if let Ok(mut metrics) = self.metrics.lock() {
                        metrics.record_remote_sr(&sr, arrival);
                    }
                }
                RtcpPayload::Bye(_) => {}
                _ => {}
            }
        }
    }

    fn decrypt_rtp(bytes: &[u8], srtp: &SrtpContext) -> Option<Vec<u8>> {
        if bytes.len() <= 12 {
            return None;
        }
        let seq = u16::from_be_bytes([bytes[2], bytes[3]]);
        let ts = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let cipher = &bytes[12..];
        let payload = srtp.unprotect(seq, ts, cipher)?;
        let mut out = Vec::with_capacity(12 + payload.len());
        out.extend_from_slice(&bytes[..12]);
        out.extend_from_slice(&payload);
        Some(out)
    }
}
