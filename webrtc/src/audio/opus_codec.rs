//! Opus encoding and decoding using audiopus.

use audiopus::coder::{Decoder, Encoder};
use audiopus::packet::Packet;
use audiopus::{Application, Channels, MutSignals, SampleRate};

const FRAME_SIZE: usize = 960; // 20ms at 48kHz

/// Error type for Opus codec operations.
#[derive(Debug)]
pub enum OpusError {
    EncoderInit(String),
    DecoderInit(String),
    EncodeError(String),
    DecodeError(String),
}

impl std::fmt::Display for OpusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncoderInit(e) => write!(f, "Encoder init failed: {}", e),
            Self::DecoderInit(e) => write!(f, "Decoder init failed: {}", e),
            Self::EncodeError(e) => write!(f, "Encode failed: {}", e),
            Self::DecodeError(e) => write!(f, "Decode failed: {}", e),
        }
    }
}

/// Opus audio encoder.
pub struct OpusEncoder {
    encoder: Encoder,
}

impl OpusEncoder {
    /// Creates a new Opus encoder for mono audio at 48kHz.
    pub fn new() -> Result<Self, OpusError> {
        let encoder = Encoder::new(
            SampleRate::Hz48000,
            Channels::Mono,
            Application::Voip,
        )
        .map_err(|e| OpusError::EncoderInit(e.to_string()))?;

        Ok(Self { encoder })
    }

    /// Encodes PCM samples to Opus.
    /// Input should be 960 samples (20ms at 48kHz).
    /// Returns the encoded Opus frame.
    pub fn encode(&mut self, samples: &[i16]) -> Result<Vec<u8>, OpusError> {
        // Opus encoder needs a buffer for output
        let mut output = vec![0u8; 1024]; // Max Opus frame size

        let len = self
            .encoder
            .encode(samples, &mut output)
            .map_err(|e| OpusError::EncodeError(e.to_string()))?;

        output.truncate(len);
        Ok(output)
    }

    /// Returns the expected frame size in samples.
    pub fn frame_size() -> usize {
        FRAME_SIZE
    }
}

/// Opus audio decoder.
pub struct OpusDecoder {
    decoder: Decoder,
}

impl OpusDecoder {
    /// Creates a new Opus decoder for mono audio at 48kHz.
    pub fn new() -> Result<Self, OpusError> {
        let decoder = Decoder::new(SampleRate::Hz48000, Channels::Mono)
            .map_err(|e| OpusError::DecoderInit(e.to_string()))?;

        Ok(Self { decoder })
    }

    /// Decodes an Opus frame to PCM samples.
    /// Returns decoded samples (typically 960 samples for 20ms at 48kHz).
    pub fn decode(&mut self, opus_data: &[u8]) -> Result<Vec<i16>, OpusError> {
        let mut output = vec![0i16; FRAME_SIZE * 2]; // Extra space for larger frames

        let packet = Packet::try_from(opus_data)
            .map_err(|e| OpusError::DecodeError(e.to_string()))?;
        
        let mut signals = MutSignals::try_from(&mut output[..])
            .map_err(|e| OpusError::DecodeError(e.to_string()))?;
        
        let samples = self
            .decoder
            .decode(Some(packet), signals, false)
            .map_err(|e| OpusError::DecodeError(e.to_string()))?;

        output.truncate(samples);
        Ok(output)
    }

    /// Generates concealment samples when a packet is lost.
    pub fn decode_lost(&mut self) -> Result<Vec<i16>, OpusError> {
        let mut output = vec![0i16; FRAME_SIZE];

        let mut signals = MutSignals::try_from(&mut output[..])
            .map_err(|e| OpusError::DecodeError(e.to_string()))?;
        
        let samples = self
            .decoder
            .decode(None, signals, false)
            .map_err(|e| OpusError::DecodeError(e.to_string()))?;

        output.truncate(samples);
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let mut encoder = OpusEncoder::new().expect("encoder");
        let mut decoder = OpusDecoder::new().expect("decoder");

        // Generate a simple sine wave
        let samples: Vec<i16> = (0..FRAME_SIZE)
            .map(|i| ((i as f32 * 0.1).sin() * 10000.0) as i16)
            .collect();

        let encoded = encoder.encode(&samples).expect("encode");
        assert!(!encoded.is_empty());

        let decoded = decoder.decode(&encoded).expect("decode");
        assert_eq!(decoded.len(), FRAME_SIZE);
    }
}
