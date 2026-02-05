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
        let supported = device
            .supported_output_configs()
            .map_err(|_| AudioPlaybackError::NoSupportedConfig)?;

        for config in supported {
            if config.channels() == CHANNELS && config.sample_format() == SampleFormat::I16 {
                if config.min_sample_rate().0 <= SAMPLE_RATE
                    && config.max_sample_rate().0 >= SAMPLE_RATE
                {
                    return Ok(config
                        .with_sample_rate(cpal::SampleRate(SAMPLE_RATE))
                        .into());
                }
            }
        }

        // Fallback: use default config
        device
            .default_output_config()
            .map(|c| c.into())
            .map_err(|_| AudioPlaybackError::NoSupportedConfig)
    }

    fn build_stream(
        device: &Device,
        config: &StreamConfig,
        buffer: Arc<Mutex<VecDeque<i16>>>,
    ) -> Result<Stream, AudioPlaybackError> {
        let err_fn = |err| eprintln!("Audio playback error: {}", err);

        let callback_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let callback_counter = Arc::clone(&callback_count);

        device
            .build_output_stream(
                config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let count = callback_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count % 100 == 0 {
                        eprintln!("[PLAYBACK] Callback #{}, requested {} samples", count, data.len());
                    }
                    if let Ok(mut buf) = buffer.lock() {
                        // Stereo output: duplicate each mono sample to both channels
                        for chunk in data.chunks_mut(2) {
                            let mono_sample = buf.pop_front().unwrap_or(0);
                            if chunk.len() == 2 {
                                chunk[0] = mono_sample; // Left channel
                                chunk[1] = mono_sample; // Right channel
                            }
                        }
                    } else {
                        // If lock fails, output silence
                        for sample in data.iter_mut() {
                            *sample = 0;
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
