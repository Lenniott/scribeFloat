use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Target sample rate required by Whisper for transcription.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// Empirical gain applied to RMS before clamping to the 0..1 meter range.
/// Speech audio typically has RMS < 0.25; this maps it to a visible level
/// without hard-coding a dB threshold.
const LEVEL_GAIN: f32 = 4.0;

/// Writer-thread poll interval. The writer wakes every `WRITER_POLL_MS` to check the stop
/// signal — short enough that `stop_and_finalize` returns promptly, long enough that the
/// atomic load cost is negligible against audio I/O.
const WRITER_POLL_MS: u64 = 100;
/// After the stop signal is raised, drain any chunks still queued by cpal. CoreAudio tears
/// down its audio unit asynchronously, so the callback (and its `Sender` clone) may live
/// briefly after `drop(stream)`. 200 ms covers that tail without risk of hanging.
const WRITER_DRAIN_TIMEOUT_MS: u64 = 200;
/// How often the writer thread flushes PCM to disk and patches the WAV RIFF header so a
/// crash mid-recording leaves a playable file.
const WAV_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(30);

/// Handle for an active capture session that streams mono f32 PCM to a 16 kHz WAV file on
/// disk. The cpal audio callback only mixes-to-mono and pushes into a channel; a separate
/// writer thread resamples to 16 kHz and appends to `hound::WavWriter`, keeping resident
/// memory flat regardless of recording length.
pub struct MicSession {
    /// Kept alive so the underlying audio unit stays open. Dropped in `stop_and_finalize`.
    stream: cpal::Stream,
    /// `None` after `stop_and_finalize` joins the writer.
    writer_handle: Option<JoinHandle<Result<()>>>,
    /// Set by `stop_and_finalize` to make the writer exit its poll loop.
    stop_signal: Arc<AtomicBool>,
    wav_path: PathBuf,
}

// cpal::Stream is Send on all supported platforms (macOS + Windows).
unsafe impl Send for MicSession {}

impl MicSession {
    /// Path of the WAV file being written. Callers cache this before `stop_and_finalize`
    /// when they need to clean up a partial file on a finalize error (the consuming method
    /// drops `self`, so the path would otherwise be unreachable).
    pub fn wav_path(&self) -> &Path {
        &self.wav_path
    }

    /// Stop capture, wait for the writer to drain pending chunks and finalize the WAV
    /// header on disk, and return the path. The file is safe to read after this returns.
    pub fn stop_and_finalize(mut self) -> Result<PathBuf> {
        // Pause before drop so CoreAudio releases the input unit promptly (avoids a
        // lingering green-mic indicator on macOS).
        let _ = self.stream.pause();
        // Dropping the stream tears down cpal's callback; the callback's `Sender` clone
        // is released asynchronously on macOS, so we also raise the signal to let the
        // writer exit on its own poll tick if the channel hasn't disconnected yet.
        self.stop_signal.store(true, Ordering::SeqCst);
        drop(self.stream);

        let handle = self
            .writer_handle
            .take()
            .ok_or_else(|| anyhow!("writer thread already joined"))?;
        match handle.join() {
            Ok(Ok(())) => Ok(self.wav_path),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("writer thread panicked")),
        }
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

    /// Open a loopback capture stream for system audio and write the resampled PCM to
    /// `wav_path` continuously. See [`Self::start_mic`] for the streaming model.
    pub fn start_loopback(
        &self,
        preferred_name: Option<&str>,
        wav_path: PathBuf,
        on_level: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    ) -> Result<MicSession> {
        let (device, supported) = crate::platform::loopback_device_and_config(preferred_name)
            .map_err(|e| anyhow!("{e}"))?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let config = supported.config();
        start_capture(
            device,
            config,
            supported.sample_format(),
            sample_rate,
            channels,
            wav_path,
            on_level,
            "loopback",
        )
    }

    /// Open a mic input stream and write the resampled PCM to `wav_path` continuously.
    /// The cpal callback is non-allocating beyond a mono mix-down per buffer; all WAV I/O
    /// runs on a dedicated thread, so callback latency stays bounded for the duration of
    /// the recording. Uses `preferred_name` if provided and available, otherwise falls
    /// back to the system default input device (when `allow_fallback_to_default`).
    pub fn start_mic(
        &self,
        preferred_name: Option<&str>,
        allow_fallback_to_default: bool,
        wav_path: PathBuf,
        on_level: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    ) -> Result<MicSession> {
        let host = cpal::default_host();
        let device = match preferred_name {
            Some(name) => {
                let mut found_exact = false;
                let selected = host
                    .input_devices()?
                    .find(|d| d.name().map(|n| n == name).unwrap_or(false))
                    .inspect(|_| {
                        found_exact = true;
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
        start_capture(
            device,
            config,
            supported.sample_format(),
            sample_rate,
            channels,
            wav_path,
            on_level,
            "mic",
        )
    }
}

/// Shared capture-and-write plumbing for mic + loopback. Creates the WAV writer, spawns
/// the writer thread, builds the cpal stream, and returns the live `MicSession`. On any
/// failure during setup the partially-created WAV file is removed so we don't leave an
/// empty header behind.
#[allow(clippy::too_many_arguments)]
fn start_capture(
    device: cpal::Device,
    config: cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    sample_rate: u32,
    channels: usize,
    wav_path: PathBuf,
    on_level: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    label: &'static str,
) -> Result<MicSession> {
    if let Some(parent) = wav_path.parent() {
        std::fs::create_dir_all(parent).context("create capture parent dir")?;
    }
    let streaming = match StreamingWavWriter::create(wav_path.clone()) {
        Ok(w) => w,
        Err(e) => {
            let _ = std::fs::remove_file(&wav_path);
            return Err(e);
        }
    };

    let (sender, receiver) = mpsc::channel::<Vec<f32>>();
    let stop_signal = Arc::new(AtomicBool::new(false));
    let writer_handle = spawn_writer_thread(streaming, receiver, sample_rate, Arc::clone(&stop_signal));

    let stream_result = build_input_stream(
        &device,
        &config,
        sample_format,
        channels,
        sender,
        on_level,
        label,
    );

    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            // Tear down the writer + delete the empty WAV so we leave the filesystem clean.
            stop_signal.store(true, Ordering::SeqCst);
            let _ = writer_handle.join();
            let _ = std::fs::remove_file(&wav_path);
            return Err(e);
        }
    };

    if let Err(e) = stream.play() {
        // Same cleanup as above plus drop the constructed stream.
        drop(stream);
        stop_signal.store(true, Ordering::SeqCst);
        let _ = writer_handle.join();
        let _ = std::fs::remove_file(&wav_path);
        return Err(e.into());
    }

    Ok(MicSession {
        stream,
        writer_handle: Some(writer_handle),
        stop_signal,
        wav_path,
    })
}

fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    channels: usize,
    sender: mpsc::Sender<Vec<f32>>,
    on_level: Option<Arc<dyn Fn(f32) + Send + Sync>>,
    label: &'static str,
) -> Result<cpal::Stream> {
    let err_fn = move |e: cpal::StreamError| tracing::error!(label, error = %e, "audio stream error");
    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let tx = sender;
            let level_cb = on_level;
            device.build_input_stream(
                config,
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
            let tx = sender;
            let level_cb = on_level;
            device.build_input_stream(
                config,
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
        fmt => return Err(anyhow!("unsupported sample format for {label}: {fmt:?}")),
    };
    Ok(stream)
}

/// Append-only 16-bit PCM WAV writer with periodic header checkpointing.
struct StreamingWavWriter {
    file: BufWriter<File>,
    path: PathBuf,
    sample_count: u64,
    last_checkpoint: Instant,
}

impl StreamingWavWriter {
    fn create(path: PathBuf) -> Result<Self> {
        crate::services::output::write_streaming_wav_placeholder(&path, WHISPER_SAMPLE_RATE, 1, 16)?;
        let file = OpenOptions::new().append(true).open(&path).context("open wav for append")?;
        Ok(Self {
            file: BufWriter::new(file),
            path,
            sample_count: 0,
            last_checkpoint: Instant::now(),
        })
    }

    fn write_i16_samples(&mut self, samples: &[i16]) -> Result<()> {
        for sample in samples {
            self.file
                .write_all(&sample.to_le_bytes())
                .context("write wav sample bytes")?;
        }
        self.sample_count += samples.len() as u64;
        if self.last_checkpoint.elapsed() >= WAV_CHECKPOINT_INTERVAL {
            self.checkpoint()?;
        }
        Ok(())
    }

    fn checkpoint(&mut self) -> Result<()> {
        self.file.flush().context("flush wav buffer")?;
        self.file.get_ref().sync_all().context("sync wav file")?;
        crate::services::output::sync_wav_header(
            &self.path,
            WHISPER_SAMPLE_RATE,
            1,
            16,
            self.sample_count,
        )?;
        self.last_checkpoint = Instant::now();
        Ok(())
    }

    fn finalize(mut self) -> Result<()> {
        self.checkpoint()
    }
}

/// Writer thread: drains the channel, resamples each chunk to 16 kHz, and appends to the
/// WAV file. Exits when either (a) the channel disconnects or (b) the stop signal is set
/// — whichever comes first. After a stop, drains residual chunks for up to
/// `WRITER_DRAIN_TIMEOUT_MS` so the tail of audio buffered by cpal is not lost.
fn spawn_writer_thread(
    mut streaming: StreamingWavWriter,
    receiver: mpsc::Receiver<Vec<f32>>,
    native_rate: u32,
    stop_signal: Arc<AtomicBool>,
) -> JoinHandle<Result<()>> {
    std::thread::spawn(move || -> Result<()> {
        let poll = Duration::from_millis(WRITER_POLL_MS);
        loop {
            if stop_signal.load(Ordering::SeqCst) {
                drain_remaining_chunks(&receiver, &mut streaming, native_rate)?;
                break;
            }
            match receiver.recv_timeout(poll) {
                Ok(chunk) => write_resampled_chunk(&chunk, native_rate, &mut streaming)?,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        streaming.finalize().context("finalize WAV")?;
        Ok(())
    })
}

fn drain_remaining_chunks(
    receiver: &mpsc::Receiver<Vec<f32>>,
    streaming: &mut StreamingWavWriter,
    native_rate: u32,
) -> Result<()> {
    let drain_timeout = Duration::from_millis(WRITER_DRAIN_TIMEOUT_MS);
    while let Ok(chunk) = receiver.recv_timeout(drain_timeout) {
        write_resampled_chunk(&chunk, native_rate, streaming)?;
    }
    Ok(())
}

fn write_resampled_chunk(
    chunk: &[f32],
    native_rate: u32,
    streaming: &mut StreamingWavWriter,
) -> Result<()> {
    let resampled = resample_linear(chunk, native_rate, WHISPER_SAMPLE_RATE);
    let samples: Vec<i16> = resampled
        .iter()
        .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect();
    streaming.write_i16_samples(&samples)
}

/// Read a 16 kHz mono i16 WAV file (the format `MicSession` writes) into f32 PCM in
/// [-1.0, 1.0]. Used by controllers to load captured audio back for transcription.
pub fn read_wav_mono_f32(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).context("open WAV")?;
    let spec = reader.spec();
    if spec.channels != 1 {
        return Err(anyhow!(
            "expected mono WAV, got {} channels",
            spec.channels
        ));
    }
    if spec.sample_rate != WHISPER_SAMPLE_RATE {
        return Err(anyhow!(
            "expected {} Hz WAV, got {} Hz",
            WHISPER_SAMPLE_RATE,
            spec.sample_rate
        ));
    }
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|x| x as f32 / 32768.0))
            .collect::<Result<_, _>>()
            .context("read int WAV samples")?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<_, _>>()
            .context("read float WAV samples")?,
    };
    Ok(samples)
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

/// Linear interpolation resampler. Good enough for speech at 16 kHz target.
pub(crate) fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        // Clamp defensively: float rounding could otherwise push `lo` to `input.len()`.
        let lo = (src.floor() as usize).min(input.len() - 1);
        let hi = (lo + 1).min(input.len() - 1);
        let frac = (src - lo as f64) as f32;
        out.push(input[lo] * (1.0 - frac) + input[hi] * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::{SampleFormat, WavSpec, WavWriter};

    fn wav_spec_int16() -> WavSpec {
        WavSpec {
            channels: 1,
            sample_rate: WHISPER_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        }
    }

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

    fn temp_wav() -> PathBuf {
        std::env::temp_dir().join(format!("scribefloat-audio-tests-{}.wav", uuid::Uuid::new_v4()))
    }

    #[test]
    fn read_wav_mono_f32_roundtrips_through_write() {
        // Write 100 samples of a 0.25 amplitude DC signal at 16 kHz mono i16, then read back.
        let path = temp_wav();
        let pcm: Vec<f32> = (0..100).map(|_| 0.25_f32).collect();
        let mut writer = WavWriter::create(&path, wav_spec_int16()).expect("create writer");
        for &s in &pcm {
            let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample).expect("write");
        }
        writer.finalize().expect("finalize");

        let round = read_wav_mono_f32(&path).expect("read");
        assert_eq!(round.len(), pcm.len());
        for (a, b) in round.iter().zip(pcm.iter()) {
            assert!((a - b).abs() < 1e-3, "roundtrip drift > 0.001 at {a} vs {b}");
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_wav_mono_f32_rejects_wrong_channel_count() {
        // Hand-write a stereo header and confirm the reader refuses it.
        let path = temp_wav();
        let spec = WavSpec {
            channels: 2,
            sample_rate: WHISPER_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&path, spec).expect("create writer");
        writer.write_sample(0_i16).unwrap();
        writer.write_sample(0_i16).unwrap();
        writer.finalize().expect("finalize");

        assert!(read_wav_mono_f32(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_wav_mono_f32_rejects_wrong_sample_rate() {
        let path = temp_wav();
        let spec = WavSpec {
            channels: 1,
            sample_rate: 48_000, // not the whisper rate
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = WavWriter::create(&path, spec).expect("create writer");
        writer.write_sample(0_i16).unwrap();
        writer.finalize().expect("finalize");

        assert!(read_wav_mono_f32(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
