//! Audio playback to speakers using cpal.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 2; // Stereo for better compatibility
const BUFFER_SIZE: usize = 4800; // 100ms buffer at 48kHz

/// Error type for audio playback operations.
#[derive(Debug)]
pub enum AudioPlaybackError {
    NoOutputDevice,
    NoSupportedConfig,
    BuildStreamError(String),
    PlayStreamError(String),
}

impl std::fmt::Display for AudioPlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOutputDevice => write!(f, "No output device found"),
            Self::NoSupportedConfig => write!(f, "No supported audio config"),
            Self::BuildStreamError(e) => write!(f, "Failed to build stream: {}", e),
            Self::PlayStreamError(e) => write!(f, "Failed to play stream: {}", e),
        }
    }
}

/// Plays audio samples received from a channel.
pub struct AudioPlayback {
    stream: Option<Stream>,
    buffer: Arc<Mutex<VecDeque<i16>>>,
}

impl AudioPlayback {
    /// Creates a new audio playback that plays samples from the provided channel.
    pub fn new(rx: Receiver<Vec<i16>>) -> Result<Self, AudioPlaybackError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioPlaybackError::NoOutputDevice)?;

        eprintln!("[PLAYBACK] Using device: {}", device.name().unwrap_or_else(|_| "Unknown".to_string()));

        let config = Self::find_config(&device)?;
        eprintln!("[PLAYBACK] Config: channels={}, sample_rate={}, sample_format={:?}", 
            config.channels, config.sample_rate.0, SampleFormat::I16);

        let buffer: Arc<Mutex<VecDeque<i16>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_SIZE * 2)));

        let buffer_producer = Arc::clone(&buffer);
        let buffer_consumer = Arc::clone(&buffer);

        // Thread to receive samples and add to buffer
        let samples_received = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let samples_counter = Arc::clone(&samples_received);
        std::thread::spawn(move || {
            while let Ok(samples) = rx.recv() {
                let count = samples_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 50 == 0 { // Log every 50 packets (~1 second)
                    eprintln!(
                        "[PLAYBACK] Received packet #{}, {} samples, buffer: {}",
                        count,
                        samples.len(),
                        buffer_producer.lock().map(|b| b.len()).unwrap_or(0)
                    );
                }
                if let Ok(mut buf) = buffer_producer.lock() {
                    buf.extend(samples);
                    // Limit buffer size to prevent unbounded growth
                    while buf.len() > BUFFER_SIZE * 4 {
                        buf.pop_front();
                    }
                }
            }
        });

        let stream = Self::build_stream(&device, &config, buffer_consumer)?;
        stream
            .play()
            .map_err(|e| AudioPlaybackError::PlayStreamError(e.to_string()))?;

        eprintln!("[PLAYBACK] Stream started successfully");

        Ok(Self {
            stream: Some(stream),
            buffer,
        })
    }

    fn find_config(device: &Device) -> Result<StreamConfig, AudioPlaybackError> {
        eprintln!("[PLAYBACK] Finding config for device...");
        let supported = device
            .supported_output_configs()
            .map_err(|_| AudioPlaybackError::NoSupportedConfig)?;

        eprintln!("[PLAYBACK] Available configs:");
        for config in supported {
            eprintln!("[PLAYBACK]   - channels={}, format={:?}, rate={}-{}",
                config.channels(),
                config.sample_format(),
                config.min_sample_rate().0,
                config.max_sample_rate().0
            );
        }

        // Use default config directly - let ALSA/PipeWire handle conversion
        let default_config = device
            .default_output_config()
            .map_err(|_| AudioPlaybackError::NoSupportedConfig)?;
        
        eprintln!("[PLAYBACK] Using default config: channels={}, format={:?}, rate={}",
            default_config.channels(),
            default_config.sample_format(),
            default_config.sample_rate().0
        );
        
        Ok(default_config.into())
    }

    fn build_stream(
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<Mutex<VecDeque<i16>>>,
    ) -> Result<Stream, AudioPlaybackError> {
        let err_fn = |err| eprintln!("Audio playback error: {}", err);

        let callback_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let callback_counter = Arc::clone(&callback_count);

        // Use f32 stream to match device format
        device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let count = callback_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count == 0 || count % 100 == 0 {
                        println!("[PLAYBACK-CALLBACK] Callback #{}, requested {} samples", count, data.len());
                        let _ = std::io::Write::flush(&mut std::io::stdout());
                    }
                    if let Ok(mut buf) = buffer.lock() {
                        // Stereo output: duplicate each mono sample to both channels
                        // Convert i16 to f32 (normalize -32768..32767 to -1.0..1.0)
                        for chunk in data.chunks_mut(2) {
                            let mono_sample = buf.pop_front().unwrap_or(0);
                            let f32_sample = mono_sample as f32 / 32768.0;
                            if chunk.len() == 2 {
                                chunk[0] = f32_sample; // Left channel
                                chunk[1] = f32_sample; // Right channel
                            }
                        }
                    } else {
                        // If lock fails, output silence
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioPlaybackError::BuildStreamError(e.to_string()))
    }
}

impl Drop for AudioPlayback {
    fn drop(&mut self) {
        self.stream.take();
    }
}
