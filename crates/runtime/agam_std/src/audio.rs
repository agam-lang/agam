//! # Lossless Audio DSP & Uncompressed WAV Container Engine (`agam_std::audio`)
//!
//! Provides multi-channel audio buffer representations, loudness metrics,
//! spatial panning/mixing, FFT spectral frequency analysis, and lossless PCM WAV I/O.

use crate::complex::Complex;
use crate::fft::{fft, hanning_window};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Nyāya-grounded structured diagnostic error for audio processing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl AudioError {
    pub fn new(
        cause: impl Into<String>,
        context: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            cause: cause.into(),
            context: context.into(),
            remedy: remedy.into(),
        }
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AudioError: {}\n  Context: {}\n  Remedy: {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for AudioError {}

/// Standard PCM Bit Depth and Format Specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PcmBitDepth {
    /// 16-bit signed integer linear PCM.
    Bits16,
    /// 24-bit signed integer studio-grade linear PCM.
    Bits24,
    /// 32-bit floating-point high-dynamic-range PCM.
    Float32,
}

/// Contiguous multichannel audio waveform buffer normalized in `[-1.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    channels: usize,
    sample_rate: u32,
    samples: Vec<f32>,
}

impl AudioBuffer {
    /// Allocate an empty multichannel audio buffer initialized with silence.
    pub fn new(channels: usize, sample_rate: u32, frame_count: usize) -> Result<Self, AudioError> {
        if channels == 0 {
            return Err(AudioError::new(
                "Invalid audio channel count",
                "Channel count must be at least 1 (mono)",
                "Provide a valid channel configuration (e.g. 1 for mono, 2 for stereo)",
            ));
        }
        if sample_rate == 0 {
            return Err(AudioError::new(
                "Invalid audio sample rate",
                "Sample rate must be greater than 0 Hz",
                "Specify a standard audio sampling rate (e.g. 44100, 48000, 96000)",
            ));
        }

        let total_samples = channels.checked_mul(frame_count).ok_or_else(|| {
            AudioError::new(
                "Audio buffer sample count overflow",
                format!(
                    "Channels {} * Frames {} exceeds usize capacity",
                    channels, frame_count
                ),
                "Allocate audio buffer in smaller time slices",
            )
        })?;

        Ok(Self {
            channels,
            sample_rate,
            samples: vec![0.0; total_samples],
        })
    }

    /// Construct an audio buffer from interleaved multichannel sample data.
    pub fn from_interleaved(
        channels: usize,
        sample_rate: u32,
        samples: Vec<f32>,
    ) -> Result<Self, AudioError> {
        if channels == 0 {
            return Err(AudioError::new(
                "Channel count must be non-zero",
                "0 channels specified",
                "Set channels to 1 or 2",
            ));
        }
        if sample_rate == 0 {
            return Err(AudioError::new(
                "Sample rate must be non-zero",
                "0 Hz sample rate specified",
                "Set sample rate to valid audio clock (e.g. 48000)",
            ));
        }
        if !samples.len().is_multiple_of(channels) {
            return Err(AudioError::new(
                "Sample count is not evenly divisible by channel count",
                format!("Samples len {} % channels {} != 0", samples.len(), channels),
                "Verify interleaved audio stream is complete and not truncated",
            ));
        }

        Ok(Self {
            channels,
            sample_rate,
            samples,
        })
    }

    #[inline]
    pub fn channels(&self) -> usize {
        self.channels
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    pub fn frame_count(&self) -> usize {
        if self.channels > 0 {
            self.samples.len() / self.channels
        } else {
            0
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[f32] {
        &self.samples
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    /// Total audio duration in fractional seconds.
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate > 0 {
            self.frame_count() as f64 / self.sample_rate as f64
        } else {
            0.0
        }
    }

    /// Extract a single channel's planar sample vector.
    pub fn extract_channel(&self, ch: usize) -> Result<Vec<f32>, AudioError> {
        if ch >= self.channels {
            return Err(AudioError::new(
                "Channel index out of bounds",
                format!("Requested channel {} but buffer has {}", ch, self.channels),
                "Specify channel index in range 0..channels",
            ));
        }

        let frames = self.frame_count();
        let mut out = Vec::with_capacity(frames);
        for frame in 0..frames {
            out.push(self.samples[frame * self.channels + ch]);
        }
        Ok(out)
    }

    /// Peak absolute sample amplitude across all channels ($[0.0, 1.0]$).
    pub fn peak_amplitude(&self) -> f32 {
        let mut peak = 0.0f32;
        for &s in &self.samples {
            let abs = s.abs();
            if abs > peak {
                peak = abs;
            }
        }
        peak
    }

    /// Root-Mean-Square (RMS) loudness metric.
    pub fn rms_loudness(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = self.samples.iter().map(|&s| s * s).sum();
        (sum_sq / self.samples.len() as f32).sqrt()
    }

    /// Calculate dynamic range in decibels (dB FS).
    pub fn dynamic_range_db(&self) -> f32 {
        let peak = self.peak_amplitude();
        let rms = self.rms_loudness();
        if rms <= 1e-9 || peak <= 1e-9 {
            return 0.0;
        }
        20.0 * (peak / rms).log10()
    }

    /// Apply linear gain multiplier across all audio samples.
    pub fn apply_gain(&mut self, gain_linear: f32) {
        for s in &mut self.samples {
            *s = (*s * gain_linear).clamp(-1.0, 1.0);
        }
    }

    /// Normalize waveform to peak target level (e.g. 0.99 for -0.1 dB ceiling).
    pub fn normalize(&mut self, target_peak: f32) {
        let current_peak = self.peak_amplitude();
        if current_peak > 1e-6 {
            let gain = target_peak / current_peak;
            self.apply_gain(gain);
        }
    }

    /// Downmix multi-channel audio to mono with equal energy distribution.
    pub fn to_mono(&self) -> Result<AudioBuffer, AudioError> {
        if self.channels == 1 {
            return Ok(self.clone());
        }

        let frames = self.frame_count();
        let mut mono_samples = Vec::with_capacity(frames);
        let scale = 1.0 / self.channels as f32;

        for f in 0..frames {
            let start = f * self.channels;
            let sum: f32 = self.samples[start..start + self.channels].iter().sum();
            mono_samples.push(sum * scale);
        }

        AudioBuffer::from_interleaved(1, self.sample_rate, mono_samples)
    }

    /// Spatial pan mono audio to stereo (-1.0 = full left, 0.0 = center, +1.0 = full right).
    pub fn pan_to_stereo(&self, pan: f32) -> Result<AudioBuffer, AudioError> {
        let mono = self.to_mono()?;
        let pan = pan.clamp(-1.0, 1.0);

        let left_gain = (1.0 - pan.max(0.0)).clamp(0.0, 1.0);
        let right_gain = (1.0 + pan.min(0.0)).clamp(0.0, 1.0);

        let frames = mono.frame_count();
        let mut stereo_samples = Vec::with_capacity(frames * 2);

        for &sample in &mono.samples {
            stereo_samples.push(sample * left_gain);
            stereo_samples.push(sample * right_gain);
        }

        AudioBuffer::from_interleaved(2, self.sample_rate, stereo_samples)
    }

    /// Compute frequency magnitude spectrum using Hanning window and Fast Fourier Transform.
    pub fn compute_spectrum(
        &self,
        channel: usize,
        window_size: usize,
    ) -> Result<Vec<f32>, AudioError> {
        if !window_size.is_power_of_two() || window_size < 4 {
            return Err(AudioError::new(
                "Invalid FFT window size",
                format!("Window size {} must be a power of two >= 4", window_size),
                "Use standard window sizes (e.g. 512, 1024, 2048, 4048)",
            ));
        }

        let ch_data = self.extract_channel(channel)?;
        if ch_data.len() < window_size {
            return Err(AudioError::new(
                "Audio signal shorter than requested FFT window",
                format!("Signal len {} < window {}", ch_data.len(), window_size),
                "Provide longer audio signal or decrease window size",
            ));
        }

        let window = hanning_window(window_size);
        let complex_input: Vec<Complex> = (0..window_size)
            .map(|i| Complex::new(ch_data[i] as f64 * window[i], 0.0))
            .collect();

        let spectrum_complex = fft(&complex_input);
        let half = window_size / 2;
        let mut magnitudes = Vec::with_capacity(half);

        for c in &spectrum_complex[..half] {
            magnitudes.push(c.magnitude() as f32);
        }

        Ok(magnitudes)
    }
}

/// Uncompressed PCM WAV (RIFF) Parser and Emitter.
pub struct WavCodec;

impl WavCodec {
    /// Encode audio buffer into standard 16-bit PCM WAV byte stream.
    pub fn encode_wav_pcm16(audio: &AudioBuffer) -> Vec<u8> {
        let channels = audio.channels as u16;
        let sample_rate = audio.sample_rate;
        let bits_per_sample = 16u16;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * block_align as u32;

        let sample_count = audio.samples.len();
        let data_chunk_size = (sample_count * 2) as u32;
        let riff_chunk_size = 36 + data_chunk_size;

        let mut buf = Vec::with_capacity(44 + data_chunk_size as usize);

        // RIFF Header
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_chunk_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");

        // "fmt " Subchunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
        buf.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 = PCM)
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());

        // "data" Subchunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_chunk_size.to_le_bytes());

        for &sample in &audio.samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let s16 = (clamped * 32767.0).round() as i16;
            buf.extend_from_slice(&s16.to_le_bytes());
        }

        buf
    }

    /// Decode standard 16-bit PCM WAV byte stream into an AudioBuffer.
    pub fn decode_wav_pcm16(bytes: &[u8]) -> Result<AudioBuffer, AudioError> {
        if bytes.len() < 44 {
            return Err(AudioError::new(
                "WAV file truncated before header completed",
                format!("Received {} bytes, minimum header size is 44", bytes.len()),
                "Verify file is a complete WAV recording",
            ));
        }

        if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(AudioError::new(
                "Invalid RIFF/WAVE header signature",
                "Stream lacks valid 'RIFF....WAVE' magic header",
                "Ensure file is an uncompressed WAV audio file",
            ));
        }

        // Parse chunks sequentially
        let mut offset = 12;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits_per_sample = 0u16;
        let mut data_offset = None;
        let mut data_len = 0usize;

        while offset + 8 <= bytes.len() {
            let chunk_id = &bytes[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]) as usize;
            offset += 8;

            if chunk_id == b"fmt " {
                if offset + 16 > bytes.len() {
                    return Err(AudioError::new(
                        "Truncated fmt chunk in WAV file",
                        "Chunk boundary exceeds file size",
                        "Ensure WAV file is not corrupted",
                    ));
                }
                let audio_format = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                if audio_format != 1 {
                    return Err(AudioError::new(
                        "Unsupported WAV audio compression format",
                        format!("Format tag {} != 1 (Uncompressed PCM)", audio_format),
                        "Convert compressed audio to standard uncompressed PCM WAV",
                    ));
                }
                channels = u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]);
                sample_rate = u32::from_le_bytes([
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ]);
                bits_per_sample = u16::from_le_bytes([bytes[offset + 14], bytes[offset + 15]]);
                offset += chunk_size;
            } else if chunk_id == b"data" {
                data_offset = Some(offset);
                data_len = chunk_size.min(bytes.len().saturating_sub(offset));
                break;
            } else {
                // Skip unknown metadata chunks (e.g. LIST, JUNK, ID3)
                offset += chunk_size;
            }
        }

        let d_offset = data_offset.ok_or_else(|| {
            AudioError::new(
                "Missing 'data' subchunk in WAV stream",
                "Reached end of file without finding audio payload",
                "Ensure WAV container contains valid PCM data",
            )
        })?;

        if bits_per_sample != 16 {
            return Err(AudioError::new(
                "Unsupported bit depth for PCM16 decoder",
                format!(
                    "WAV file has {}-bit samples, expected 16-bit",
                    bits_per_sample
                ),
                "Provide 16-bit PCM WAV recording or convert beforehand",
            ));
        }

        let raw_pcm = &bytes[d_offset..d_offset + data_len];
        let sample_count = raw_pcm.len() / 2;
        let mut samples = Vec::with_capacity(sample_count);

        for chunk in raw_pcm.chunks_exact(2) {
            let s16 = i16::from_le_bytes([chunk[0], chunk[1]]);
            samples.push(s16 as f32 / 32768.0);
        }

        AudioBuffer::from_interleaved(channels as usize, sample_rate, samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_buffer_allocation_and_metrics() -> Result<(), AudioError> {
        let mut audio = AudioBuffer::new(2, 48000, 48000)?;
        assert_eq!(audio.channels(), 2);
        assert_eq!(audio.sample_rate(), 48000);
        assert_eq!(audio.frame_count(), 48000);
        assert!((audio.duration_seconds() - 1.0).abs() < 1e-6);

        // Fill with sine wave on Left channel
        for i in 0..48000 {
            let t = i as f32 / 48000.0;
            let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.8;
            audio.samples[i * 2] = val;
        }

        let peak = audio.peak_amplitude();
        assert!((peak - 0.8).abs() < 1e-3);

        let rms = audio.rms_loudness();
        assert!(rms > 0.3 && rms < 0.6);
        Ok(())
    }

    #[test]
    fn test_audio_stereo_panning_and_mono_downmix() -> Result<(), AudioError> {
        let mut mono = AudioBuffer::new(1, 44100, 100)?;
        for s in mono.as_mut_slice() {
            *s = 0.5;
        }

        let stereo_center = mono.pan_to_stereo(0.0)?;
        assert_eq!(stereo_center.channels(), 2);

        let downmixed = stereo_center.to_mono()?;
        assert_eq!(downmixed.channels(), 1);
        assert!((downmixed.samples[0] - 0.5).abs() < 1e-3);
        Ok(())
    }

    #[test]
    fn test_wav_pcm16_codec_roundtrip() -> Result<(), AudioError> {
        let mut original = AudioBuffer::new(2, 44100, 1000)?;
        for i in 0..1000 {
            original.samples[i * 2] = 0.25;
            original.samples[i * 2 + 1] = -0.25;
        }

        let encoded = WavCodec::encode_wav_pcm16(&original);
        assert!(encoded.starts_with(b"RIFF"));

        let decoded = WavCodec::decode_wav_pcm16(&encoded)?;
        assert_eq!(decoded.channels(), original.channels());
        assert_eq!(decoded.sample_rate(), original.sample_rate());
        assert_eq!(decoded.frame_count(), original.frame_count());

        let diff = (decoded.samples[0] - 0.25).abs();
        assert!(
            diff < 1e-3,
            "Quantization error should be minimal for 16-bit PCM"
        );
        Ok(())
    }

    #[test]
    fn test_fft_spectral_analysis_detects_peak_frequency() -> Result<(), AudioError> {
        let sample_rate = 8000;
        let freq = 1000.0f32;
        let mut audio = AudioBuffer::new(1, sample_rate, 1024)?;

        for i in 0..1024 {
            let t = i as f32 / sample_rate as f32;
            audio.samples[i] = (2.0 * std::f32::consts::PI * freq * t).sin();
        }

        let spectrum = audio.compute_spectrum(0, 1024)?;
        assert_eq!(spectrum.len(), 512);

        // Find bin with highest magnitude
        let max_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        let bin_freq = (max_bin as f32 * sample_rate as f32) / 1024.0;
        assert!(
            (bin_freq - freq).abs() < 20.0,
            "Detected frequency {} should be close to 1000 Hz",
            bin_freq
        );
        Ok(())
    }
}
