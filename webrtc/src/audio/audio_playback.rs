//! Audio playback to speakers using rodio (better PipeWire compatibility).

use rodio::{OutputStream, Sink, Source};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 1; // Mono input

/// Error type for audio playback operations.
#[derive(Debug)]
pub enum AudioPlaybackError {
    StreamError(String),
}

impl std::fmt::Display for AudioPlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StreamError(e) => write!(f, "Stream error: {}", e),
        }
    }
}

/// Custom audio source that reads i16 samples from a channel
struct ChannelSource {
    rx: Arc<Mutex<Receiver<Vec<i16>>>>,
    current_buffer: Vec<i16>,
    position: usize,
}

impl ChannelSource {
    fn new(rx: Receiver<Vec<i16>>) -> Self {
        Self {
            rx: Arc::new(Mutex::new(rx)),
            current_buffer: Vec::new(),
            position: 0,
        }
    }
}

impl Iterator for ChannelSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // If we've consumed all samples in current buffer, try to get more
        if self.position >= self.current_buffer.len() {
            if let Ok(guard) = self.rx.lock() {
                // Try to receive without blocking
                if let Ok(new_samples) = guard.try_recv() {
                    // Log occasionally to confirm data flow
                    if new_samples.len() > 0 && rand::random::<u8>() < 5 { // ~2% chance to log per buffer
                        eprintln!("[PLAYBACK-RODIO] Consuming buffer of {} samples", new_samples.len());
                    }
                    self.current_buffer = new_samples;
                    self.position = 0;
                } else {
                    // No data available, return silence
                    return Some(0);
                }
            } else {
                return Some(0);
            }
        }

        // Return next sample
        if self.position < self.current_buffer.len() {
            let sample = self.current_buffer[self.position];
             self.position += 1;
            Some(sample)
        } else {
            Some(0)
        }
    }
}

impl Source for ChannelSource {
    fn current_frame_len(&self) -> Option<usize> {
        None // Infinite stream
    }

    fn channels(&self) -> u16 {
        CHANNELS
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None // Infinite stream
    }
}

/// Plays audio samples received from a channel.
pub struct AudioPlayback {
    _stream: OutputStream,
    _sink: Sink,
}

impl AudioPlayback {
    /// Creates a new audio playback that plays samples from the provided channel.
    pub fn new(rx: Receiver<Vec<i16>>) -> Result<Self, AudioPlaybackError> {
        eprintln!("[PLAYBACK-RODIO] Initializing rodio output stream...");
        
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioPlaybackError::StreamError(e.to_string()))?;

        eprintln!("[PLAYBACK-RODIO] Creating sink...");
        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| AudioPlaybackError::StreamError(e.to_string()))?;

        let source = ChannelSource::new(rx);
        
        eprintln!("[PLAYBACK-RODIO] Appending source to sink...");
        sink.append(source);
        
        eprintln!("[PLAYBACK-RODIO] Playback started successfully!");

        Ok(Self {
            _stream: stream,
            _sink: sink,
        })
    }
}
