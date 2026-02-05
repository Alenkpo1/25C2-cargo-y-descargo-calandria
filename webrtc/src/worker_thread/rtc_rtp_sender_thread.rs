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
        let mut consecutive_errors = 0;
        
        while let Ok(encoded_bytes) = self.rx_encoded.recv() {
            let send_result = {
                let mut socket = match peer_socket.lock() {
                    Ok(s) => s,
                    Err(_) => {
                        // Lock poisoned, but keep trying
                        consecutive_errors += 1;
                        if consecutive_errors > 100 {
                            eprintln!("RTP Sender: Too many consecutive errors, stopping");
                            return Err(WorkerError::SendError);
                        }
                        continue;
                    }
                };
                self.sender.send_video_payload(encoded_bytes, &mut socket)
            };
            
            match send_result {
                Ok(_) => {
                    consecutive_errors = 0; // Reset error counter on success
                }
                Err(e) => {
                    // Log but continue - network might recover
                    consecutive_errors += 1;
                    if consecutive_errors == 1 || consecutive_errors % 50 == 0 {
                        eprintln!("RTP Sender: Send failed ({}), continuing... (errors: {})", e, consecutive_errors);
                    }
                    // Only give up after many consecutive failures
                    if consecutive_errors > 300 {
                        eprintln!("RTP Sender: Too many errors, stopping");
                        return Err(WorkerError::SendError);
                    }
                }
            }
        }
        Ok(())
    }
}
