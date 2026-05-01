use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// Handle for an active mic recording session.
/// Drop or call stop_and_take() to end it.
pub struct MicSession {
    /// Kept alive to hold the stream open; drop stops capture.
    _stream: cpal::Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

// cpal::Stream is Send on all supported platforms (macOS + Windows).
unsafe impl Send for MicSession {}

impl MicSession {
    /// Stop the stream and return (mono_f32_pcm, sample_rate).
    pub fn stop_and_take(self) -> (Vec<f32>, u32) {
        let rate = self.sample_rate;
        // Drop stream first — stops callbacks before we read the buffer.
        drop(self._stream);
        let buf = self.buffer.lock().unwrap_or_else(|p| p.into_inner()).clone();
        (buf, rate)
    }
}

pub struct AudioService;

impl AudioService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
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
        on_level: Option<Arc<dyn Fn(f32) + Send + Sync>>,
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
            None => {
                let selected = host
                    .default_input_device()
                    .ok_or_else(|| anyhow!("no default input device"))?;
                selected
            }
        };

        let supported = device.default_input_config()?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let config = supported.config();

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let err_fn = |e: cpal::StreamError| eprintln!("mic stream error: {e}");

        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                let buf = Arc::clone(&buffer);
                let level_cb = on_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| {
                        push_mono(&buf, data, channels);
                        if let Some(cb) = &level_cb {
                            cb(level_from_chunk(data, channels));
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let buf = Arc::clone(&buffer);
                let level_cb = on_level.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let f32s: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        push_mono(&buf, &f32s, channels);
                        if let Some(cb) = &level_cb {
                            cb(level_from_chunk(&f32s, channels));
                        }
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
            buffer,
            sample_rate,
        })
    }

}
/// Append samples to buffer, mixing down to mono if needed.
/// Uses try_lock so we never block the audio callback thread.
fn push_mono(buf: &Mutex<Vec<f32>>, data: &[f32], channels: usize) {
    if let Ok(mut b) = buf.try_lock() {
        if channels == 1 {
            b.extend_from_slice(data);
        } else {
            b.extend(
                data.chunks(channels)
                    .map(|c| c.iter().sum::<f32>() / channels as f32),
            );
        }
    }
}

fn level_from_chunk(data: &[f32], channels: usize) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let rms = if channels <= 1 {
        let sum = data.iter().map(|s| s * s).sum::<f32>();
        (sum / data.len() as f32).sqrt()
    } else {
        let mut sum = 0.0f32;
        let mut n = 0usize;
        for frame in data.chunks(channels) {
            let mono = frame.iter().copied().sum::<f32>() / channels as f32;
            sum += mono * mono;
            n += 1;
        }
        if n == 0 {
            0.0
        } else {
            (sum / n as f32).sqrt()
        }
    };
    (rms * 4.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_mono_keeps_single_channel_samples() {
        let buf = Mutex::new(Vec::new());
        push_mono(&buf, &[0.1, -0.2, 0.3], 1);
        assert_eq!(buf.lock().unwrap().as_slice(), &[0.1, -0.2, 0.3]);
    }

    #[test]
    fn push_mono_averages_multichannel_frames() {
        let buf = Mutex::new(Vec::new());
        // Two stereo frames: (0.2, 0.6) and (-0.4, 0.2)
        push_mono(&buf, &[0.2, 0.6, -0.4, 0.2], 2);
        assert_eq!(buf.lock().unwrap().as_slice(), &[0.4, -0.1]);
    }

    #[test]
    fn level_from_chunk_tracks_signal_strength_and_clamps() {
        let quiet = level_from_chunk(&[0.01, -0.01, 0.01, -0.01], 1);
        let loud = level_from_chunk(&[0.5, -0.5, 0.5, -0.5], 1);
        let clipped = level_from_chunk(&[2.0, -2.0], 1);

        assert!(quiet > 0.0);
        assert!(loud > quiet);
        assert!((0.0..=1.0).contains(&loud));
        assert_eq!(clipped, 1.0);
    }
}
