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
        let buf = self.buffer.lock().unwrap().clone();
        (buf, rate)
    }

}

pub struct AudioService;

impl AudioService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    /// Open a mic input stream. Uses preferred_name if provided and available,
    /// otherwise falls back to the system default input device.
    pub fn start_mic(&self, preferred_name: Option<&str>) -> Result<MicSession> {
        let host = cpal::default_host();

        let device = match preferred_name {
            Some(name) => host
                .input_devices()?
                .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                .or_else(|| host.default_input_device())
                .ok_or_else(|| anyhow!("no input device found"))?,
            None => host
                .default_input_device()
                .ok_or_else(|| anyhow!("no default input device"))?,
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
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _| push_mono(&buf, data, channels),
                    err_fn,
                    None,
                )?
            }
            cpal::SampleFormat::I16 => {
                let buf = Arc::clone(&buffer);
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _| {
                        let f32s: Vec<f32> =
                            data.iter().map(|&s| s as f32 / 32768.0).collect();
                        push_mono(&buf, &f32s, channels);
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
