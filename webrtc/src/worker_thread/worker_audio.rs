//! Audio worker that handles audio capture, encoding, transmission and playback.

use crate::audio::audio_capture::{AudioCapture, AudioCaptureError};
use crate::audio::audio_playback::{AudioPlayback, AudioPlaybackError};
use crate::audio::opus_codec::{OpusDecoder, OpusEncoder, OpusError};
use crate::crypto::srtp::SrtpContext;
use crate::protocols::rtp::constants::rtp_const::RTP_OPUS_TYPE;
use crate::protocols::rtp::rtp_header::RtpHeader;
use crate::rtc::socket::peer_socket::PeerSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

const AUDIO_SSRC: u32 = 2000;
const OPUS_FRAME_SIZE: usize = 960; // 20ms at 48kHz

/// Error type for audio worker operations.
#[derive(Debug)]
pub enum WorkerAudioError {
    Capture(String),
    Playback(String),
    Codec(String),
}

impl std::fmt::Display for WorkerAudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capture(e) => write!(f, "Audio capture error: {}", e),
            Self::Playback(e) => write!(f, "Audio playback error: {}", e),
            Self::Codec(e) => write!(f, "Audio codec error: {}", e),
        }
    }
}

impl From<AudioCaptureError> for WorkerAudioError {
    fn from(e: AudioCaptureError) -> Self {
        Self::Capture(e.to_string())
    }
}

impl From<AudioPlaybackError> for WorkerAudioError {
    fn from(e: AudioPlaybackError) -> Self {
        Self::Playback(e.to_string())
    }
}

impl From<OpusError> for WorkerAudioError {
    fn from(e: OpusError) -> Self {
        Self::Codec(e.to_string())
    }
}

/// Manages audio transmission and reception.
pub struct WorkerAudio {
    capture: Option<AudioCapture>,
    tx_incoming: SyncSender<Vec<u8>>,
    running: Arc<AtomicBool>,
    #[allow(dead_code)]
    handles: Vec<JoinHandle<()>>,
}

impl WorkerAudio {
    /// Starts the audio worker with capture, encoding, transmission and playback.
    pub fn start(
        peer_socket: Arc<Mutex<PeerSocket>>,
        srtp_context: Option<SrtpContext>,
    ) -> Result<Self, WorkerAudioError> {
        let running = Arc::new(AtomicBool::new(true));
        let mut handles = Vec::new();

        // Channels for audio pipeline
        let (tx_pcm_capture, rx_pcm_capture) = mpsc::sync_channel::<Vec<i16>>(4);
        let (tx_opus_encoded, rx_opus_encoded) = mpsc::sync_channel::<Vec<u8>>(4);
        let (tx_incoming, rx_incoming) = mpsc::sync_channel::<Vec<u8>>(8);
        let (tx_pcm_playback, rx_pcm_playback) = mpsc::sync_channel::<Vec<i16>>(4);

        // Start audio capture
        let capture = AudioCapture::new(tx_pcm_capture)?;

        // Start audio playback
        let _playback = AudioPlayback::new(rx_pcm_playback)?;

        // Encoder thread: PCM -> Opus
        let running_enc = Arc::clone(&running);
        let encoder_handle = thread::spawn(move || {
            let mut encoder = match OpusEncoder::new() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Failed to create Opus encoder: {}", e);
                    return;
                }
            };

            let mut buffer = Vec::with_capacity(OPUS_FRAME_SIZE * 2);

            while running_enc.load(Ordering::Relaxed) {
                match rx_pcm_capture.recv() {
                    Ok(samples) => {
                        buffer.extend(samples);

                        // Process complete frames
                        while buffer.len() >= OPUS_FRAME_SIZE {
                            let frame: Vec<i16> = buffer.drain(..OPUS_FRAME_SIZE).collect();
                            if let Ok(encoded) = encoder.encode(&frame) {
                                eprintln!("[AUDIO] Encoded {} bytes", encoded.len());
                                let _ = tx_opus_encoded.try_send(encoded);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        handles.push(encoder_handle);

        // RTP sender thread: Opus -> RTP -> Socket
        let running_rtp = Arc::clone(&running);
        let socket_for_rtp = Arc::clone(&peer_socket);
        let srtp_for_sender = srtp_context.clone();
        let rtp_sender_handle = thread::spawn(move || {
            let mut sequence: u16 = rand::random();
            let mut timestamp: u32 = rand::random();

            while running_rtp.load(Ordering::Relaxed) {
                match rx_opus_encoded.recv() {
                    Ok(opus_frame) => {
                        // Build RTP header
                        let header = RtpHeader::new(
                            2,              // version
                            false,          // padding
                            false,          // extension
                            0,              // csrc count
                            true,           // marker (each Opus frame is complete)
                            RTP_OPUS_TYPE,  // payload type
                            sequence,
                            timestamp,
                            AUDIO_SSRC,
                            vec![],
                        );

                        // Encrypt payload if SRTP is available
                        let payload = if let Some(ref ctx) = srtp_for_sender {
                            match ctx.protect(sequence, timestamp, &opus_frame) {
                                Some(encrypted) => encrypted,
                                None => opus_frame.clone(),
                            }
                        } else {
                            opus_frame
                        };

                        let mut packet_bytes = header.write_bytes();
                        packet_bytes.extend(payload);

                        if let Ok(socket) = socket_for_rtp.lock() {
                            let _ = socket.send(&packet_bytes);
                            eprintln!("[AUDIO] Sent RTP packet: seq={}, ts={}, size={}", sequence, timestamp, packet_bytes.len());
                        }

                        sequence = sequence.wrapping_add(1);
                        timestamp = timestamp.wrapping_add(OPUS_FRAME_SIZE as u32);
                    }
                    Err(_) => break,
                }
            }
        });
        handles.push(rtp_sender_handle);

        // Decoder thread: RTP -> Opus -> PCM
        let running_dec = Arc::clone(&running);
        let srtp_for_receiver = srtp_context;
        let decoder_handle = thread::spawn(move || {
            let mut decoder = match OpusDecoder::new() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to create Opus decoder: {}", e);
                    return;
                }
            };

            while running_dec.load(Ordering::Relaxed) {
                match rx_incoming.recv() {
                    Ok(rtp_data) => {
                        eprintln!("[AUDIO] Decoder received RTP packet: size={}", rtp_data.len());
                        if rtp_data.len() < 12 {
                            continue;
                        }

                        // Extract payload from RTP
                        let (header, header_size) = RtpHeader::read_bytes(&rtp_data);
                        if header.get_ssrc() != AUDIO_SSRC {
                            continue; // Not an audio packet
                        }

                        let encrypted_payload = &rtp_data[header_size..];
                        
                        let opus_data = if let Some(ref ctx) = srtp_for_receiver {
                            match ctx.unprotect(
                                header.get_sequence_number(),
                                header.get_timestamp(),
                                encrypted_payload,
                            ) {
                                Some(data) => data,
                                None => continue,
                            }
                        } else {
                            encrypted_payload.to_vec()
                        };

                        if let Ok(pcm) = decoder.decode(&opus_data) {
                            eprintln!("[AUDIO] Decoded {} PCM samples", pcm.len());
                            let _ = tx_pcm_playback.try_send(pcm);
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        handles.push(decoder_handle);

        Ok(Self {
            capture: Some(capture),
            tx_incoming,
            running,
            handles,
        })
    }

    /// Returns the sender for incoming audio RTP packets.
    pub fn incoming_sender(&self) -> SyncSender<Vec<u8>> {
        self.tx_incoming.clone()
    }

    /// Mutes or unmutes the microphone.
    pub fn set_muted(&self, muted: bool) {
        if let Some(ref capture) = self.capture {
            capture.set_muted(muted);
        }
    }

    /// Returns whether the microphone is currently muted.
    pub fn is_muted(&self) -> bool {
        self.capture.as_ref().map(|c| c.is_muted()).unwrap_or(false)
    }

    /// Toggles mute state and returns the new state.
    pub fn toggle_mute(&self) -> bool {
        if let Some(ref capture) = self.capture {
            capture.toggle_mute()
        } else {
            false
        }
    }

    /// Returns the SSRC used for audio.
    pub fn ssrc() -> u32 {
        AUDIO_SSRC
    }
}

impl Drop for WorkerAudio {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.capture.take();
        // Handles will be dropped automatically
    }
}
