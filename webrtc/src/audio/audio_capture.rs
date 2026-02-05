//! Audio capture from microphone using cpal.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: u16 = 1;

/// Error type for audio capture operations.
#[derive(Debug)]
pub enum AudioCaptureError {
    NoInputDevice,
    NoSupportedConfig,
    BuildStreamError(String),
    PlayStreamError(String),
}

impl std::fmt::Display for AudioCaptureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoInputDevice => write!(f, "No input device found"),
            Self::NoSupportedConfig => write!(f, "No supported audio config"),
            Self::BuildStreamError(e) => write!(f, "Failed to build stream: {}", e),
            Self::PlayStreamError(e) => write!(f, "Failed to play stream: {}", e),
        }
    }
}

/// Captures audio from the default input device.
pub struct AudioCapture {
    stream: Option<Stream>,
    muted: Arc<AtomicBool>,
}

impl AudioCapture {
    /// Creates a new audio capture that sends PCM samples to the provided channel.
    pub fn new(tx: SyncSender<Vec<i16>>) -> Result<Self, AudioCaptureError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioCaptureError::NoInputDevice)?;

        let config = Self::find_config(&device)?;
        let muted = Arc::new(AtomicBool::new(false));
        let muted_clone = Arc::clone(&muted);

        let stream = Self::build_stream(&device, &config, tx, muted_clone)?;
        stream
            .play()
            .map_err(|e| AudioCaptureError::PlayStreamError(e.to_string()))?;

        Ok(Self {
            stream: Some(stream),
            muted,
        })
    }

    fn find_config(device: &Device) -> Result<StreamConfig, AudioCaptureError> {
        let supported = device
            .supported_input_configs()
            .map_err(|_| AudioCaptureError::NoSupportedConfig)?;

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
            .default_input_config()
            .map(|c| c.into())
            .map_err(|_| AudioCaptureError::NoSupportedConfig)
    }

    fn build_stream(
        device: &Device,
        config: &StreamConfig,
        tx: SyncSender<Vec<i16>>,
        muted: Arc<AtomicBool>,
    ) -> Result<Stream, AudioCaptureError> {
        let err_fn = |err| eprintln!("Audio capture error: {}", err);

        device
            .build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if muted.load(Ordering::Relaxed) {
                        // Send silence when muted
                        let silence = vec![0i16; data.len()];
                        let _ = tx.try_send(silence);
                    } else {
                        let _ = tx.try_send(data.to_vec());
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioCaptureError::BuildStreamError(e.to_string()))
    }

    /// Mutes or unmutes the microphone.
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    /// Returns whether the microphone is currently muted.
    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    /// Toggles mute state and returns the new state.
    pub fn toggle_mute(&self) -> bool {
        let new_state = !self.is_muted();
        self.set_muted(new_state);
        new_state
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stream.take();
    }
}
