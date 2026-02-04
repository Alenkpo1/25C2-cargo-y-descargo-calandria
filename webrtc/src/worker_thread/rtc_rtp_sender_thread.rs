use crate::rtc::rtc_rtp::rtc_rtp_sender::RtcRtpSender;
use crate::rtc::socket::peer_socket::PeerSocket;
use crate::worker_thread::error::worker_error::WorkerError;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

pub struct RtpSenderThread {
    rx_encoded: Receiver<Vec<u8>>,
    sender: RtcRtpSender,
}
impl RtpSenderThread {
    pub fn new(rx_encoded: Receiver<Vec<u8>>, sender: RtcRtpSender) -> Self {
        RtpSenderThread { rx_encoded, sender }
    }

    pub fn run(&mut self, peer_socket: Arc<Mutex<PeerSocket>>) -> Result<(), WorkerError> {
        while let Ok(encoded_bytes) = self.rx_encoded.recv() {
            let mut socket = peer_socket.lock().map_err(|_| WorkerError::SendError)?;
            self.sender
                .send_video_payload(encoded_bytes, &mut socket)
                .map_err(|_| WorkerError::SendError)?;
        }
        Ok(())
    }
}
