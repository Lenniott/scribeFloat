use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;

/// Target sample rate required by Whisper for transcription.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// Empirical gain applied to RMS before clamping to the 0..1 meter range.
/// Speech audio typically has RMS < 0.25; this maps it to a visible level
/// without hard-coding a dB threshold.
const LEVEL_GAIN: f32 = 4.0;

/// Handle for an active mic recording session.
/// Call stop_and_take() to end the session and collect the samples.
pub struct MicSession {
    /// Kept alive to hold the stream open; dropping it stops capture.
    _stream: cpal::Stream,
    receiver: mpsc::Receiver<Vec<f32>>,
    sample_rate: u32,
}

// cpal::Stream is Send on all supported platforms (macOS + Windows).
unsafe impl Send for MicSession {}

impl MicSession {
    /// Stop the stream and return (mono_f32_pcm, sample_rate).
    pub fn stop_and_take(self) -> (Vec<f32>, u32) {
        let MicSession {
            _stream,
            receiver,
            sample_rate,
        } = self;
        // Pause before teardown — on macOS dropping alone can leave TCC / menu-bar mic active briefly
        // or until another route tick; pause asks CoreAudio to release capture promptly.
        let _ = _stream.pause();
        drop(_stream);
        let mut all = Vec::new();
        // Drain until the stream callback's sender is dropped — try_recv alone can miss the
        // final chunks if the main thread runs ahead of the audio thread.
        while let Ok(chunk) = receiver.recv() {
            all.extend(chunk);
        }
        (all, sample_rate)
    }
}

pub struct AudioService;

impl AudioService {
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self)
    }

    pub fn list_input_devices(&self) -> Vec<String> {
        let host = cpal::default_host();
        host.input_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }

    pub fn list_output_devices(&self) -> Vec<String> {
        let host = cpal::default_host();
        host.output_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default()
    }

    pub fn get_output_device(&self) -> Option<String> {
        crate::platform::get_default_output_device().ok()
    }

    pub fn set_output_device(&self, device: &str) -> Result<()> {
        crate::platform::set_default_output_device(device).map_err(|e| anyhow!("{e}"))
    }

    pub fn output_device_exists(&self, name: &str) -> bool {
        crate::platform::output_device_exists(name)
    }

    /// Open a mic input stream. Uses preferred_name if provided and available,
    /// otherwise falls back to the system default input device.
    pub fn start_mic(
        &self,
        preferred_name: Option<&str>,
        allow_fallback_to_default: bool,
        on_level: Option<std::sync::Arc<dyn Fn(f32) + Send + Sync>>,
    ) -> Result<MicSession> {
        let host = cpal::default_host();
        let device = match preferred_name {
            Some(name) => {
                let mut found_exact = false;
                let selected = host
                    .input_devices()?
                    .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                    .map(|d| {
                        found_exact = true;
                        d
                    })
                    .or_else(|| {
                        if allow_fallback_to_default {
                            host.default_input_device()
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| anyhow!("no input device found"))?;
                if !found_exact && !allow_fallback_to_default {
                    return Err(anyhow!(
                        "preferred input device `{name}` was not found; refusing fallback for this stream"
                    ));
                }
                selected
            }
            None => host
                .default_input_device()
                .ok_or_else(|| anyhow!("no default input device"))?,
        };

        let supported = device.default_input_config()?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let config = supported.config();

        let (sender, receiver) = mpsc::channel::<Vec<f32>>();
        let err_fn = |e: cpal::StreamError| eprintln!("mic stream error: {e}");

        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                let tx = sender.clone();
                let level_cb = on_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        let mono = mix_to_mono(data, channels);
                        if let Some(cb) = &level_cb {
                            cb(level_from_mono(&mono));
                        }
                        tx.send(mono).ok();
                    },
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let tx = sender.clone();
                let level_cb = on_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let f32s: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        let mono = mix_to_mono(&f32s, channels);
                        if let Some(cb) = &level_cb {
                            cb(level_from_mono(&mono));
                        }
                        tx.send(mono).ok();
                    },
                    err_fn,
                    None,
                )?
            }
            fmt => return Err(anyhow!("unsupported sample format: {fmt:?}")),
        };

        stream.play()?;

        Ok(MicSession {
            _stream: stream,
            receiver,
            sample_rate,
        })
    }
}

/// Mix multi-channel interleaved audio down to mono. Single-channel input is
/// returned as-is (cloned). Multi-channel frames are averaged across channels.
fn mix_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        data.to_vec()
    } else {
        data.chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    }
}

/// Compute a normalised level (0..1) from a mono PCM slice.
fn level_from_mono(mono: &[f32]) -> f32 {
    if mono.is_empty() {
        return 0.0;
    }
    let rms = (mono.iter().map(|s| s * s).sum::<f32>() / mono.len() as f32).sqrt();
    (rms * LEVEL_GAIN).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_to_mono_keeps_single_channel_samples() {
        assert_eq!(mix_to_mono(&[0.1, -0.2, 0.3], 1), &[0.1, -0.2, 0.3]);
    }

    #[test]
    fn mix_to_mono_averages_multichannel_frames() {
        // Two stereo frames: (0.2, 0.6) → 0.4 and (-0.4, 0.2) → -0.1
        let result = mix_to_mono(&[0.2, 0.6, -0.4, 0.2], 2);
        assert_eq!(result, &[0.4, -0.1]);
    }

    #[test]
    fn level_from_mono_tracks_signal_strength_and_clamps() {
        let quiet = level_from_mono(&[0.01, -0.01, 0.01, -0.01]);
        let loud = level_from_mono(&[0.5, -0.5, 0.5, -0.5]);
        let clipped = level_from_mono(&[2.0, -2.0]);

        assert!(quiet > 0.0);
        assert!(loud > quiet);
        assert!((0.0..=1.0).contains(&loud));
        assert_eq!(clipped, 1.0);
    }
}
