use crate::services::audio::{resample_linear, WHISPER_SAMPLE_RATE};
use crate::types::TranscribeSourceType;
use std::collections::HashSet;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

pub struct TranscribeInputService;

#[derive(Debug, Clone)]
pub struct TranscribeInputItem {
    pub id: String,
    pub source_path: PathBuf,
    pub display_name: String,
    pub source_type: TranscribeSourceType,
    pub mic_path: PathBuf,
    pub speaker_path: Option<PathBuf>,
    pub duration_ms: u64,
}

pub struct DecodedTranscribeInput {
    pub mic_pcm_16k: Vec<f32>,
    pub speaker_pcm_16k: Option<Vec<f32>>,
}

impl TranscribeInputService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    pub fn expand_inputs(&self, paths: &[String]) -> Result<Vec<TranscribeInputItem>, String> {
        if paths.is_empty() {
            return Err("no input paths provided".to_string());
        }

        let mut items: Vec<TranscribeInputItem> = Vec::new();
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();

        for raw in paths {
            let canonical = canonicalize_existing(raw)?;
            if canonical.is_dir() {
                if let Some((mic_path, speaker_path)) = classify_session_dir(&canonical) {
                    if seen_paths.insert(canonical.clone()) {
                        let duration_ms = self.estimate_duration_ms(&mic_path)?;
                        let source_type = if speaker_path.is_some() {
                            TranscribeSourceType::DualSourceSession
                        } else {
                            TranscribeSourceType::SingleAudio
                        };
                        items.push(TranscribeInputItem {
                            id: uuid::Uuid::new_v4().to_string(),
                            source_path: canonical.clone(),
                            display_name: canonical
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_else(|| canonical.to_string_lossy().to_string()),
                            source_type,
                            mic_path,
                            speaker_path,
                            duration_ms,
                        });
                    }
                    continue;
                }

                let mut files = Vec::new();
                collect_audio_files_recursively(&canonical, &mut files)
                    .map_err(|e| format!("failed to read folder `{}`: {e}", canonical.display()))?;
                files.sort();
                for file in files {
                    if seen_paths.insert(file.clone()) {
                        items.push(self.make_single_audio_item(file)?);
                    }
                }
                continue;
            }

            if !is_supported_audio_file(&canonical) {
                return Err(format!(
                    "unsupported audio file `{}` (expected mp3, m4a, wav, ogg, flac)",
                    canonical.display()
                ));
            }
            if seen_paths.insert(canonical.clone()) {
                items.push(self.make_single_audio_item(canonical)?);
            }
        }

        if items.is_empty() {
            return Err("no supported audio files found in selected inputs".to_string());
        }
        Ok(items)
    }

    pub fn decode_input(
        &self,
        input: &TranscribeInputItem,
    ) -> Result<DecodedTranscribeInput, String> {
        let (mic_pcm, mic_rate) = decode_audio_file(&input.mic_path)?;
        let mic_pcm_16k = resample_linear(&mic_pcm, mic_rate, WHISPER_SAMPLE_RATE);
        let speaker_pcm_16k = if let Some(speaker_path) = &input.speaker_path {
            let (speaker_pcm, speaker_rate) = decode_audio_file(speaker_path)?;
            Some(resample_linear(
                &speaker_pcm,
                speaker_rate,
                WHISPER_SAMPLE_RATE,
            ))
        } else {
            None
        };
        Ok(DecodedTranscribeInput {
            mic_pcm_16k,
            speaker_pcm_16k,
        })
    }

    fn make_single_audio_item(&self, path: PathBuf) -> Result<TranscribeInputItem, String> {
        let duration_ms = self.estimate_duration_ms(&path)?;
        let display_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Ok(TranscribeInputItem {
            id: uuid::Uuid::new_v4().to_string(),
            source_path: path.clone(),
            display_name,
            source_type: TranscribeSourceType::SingleAudio,
            mic_path: path,
            speaker_path: None,
            duration_ms,
        })
    }

    fn estimate_duration_ms(&self, path: &Path) -> Result<u64, String> {
        let probed = probe_stream(path)?;
        if let Some(track) = probed.format.default_track() {
            if let (Some(time_base), Some(n_frames)) =
                (track.codec_params.time_base, track.codec_params.n_frames)
            {
                let time = time_base.calc_time(n_frames);
                let millis = time
                    .seconds
                    .saturating_mul(1000)
                    .saturating_add((time.frac * 1000.0).round() as u64);
                return Ok(millis.max(1));
            }
        }

        let (pcm, sample_rate) = decode_audio_file(path)?;
        if sample_rate == 0 {
            return Ok(0);
        }
        Ok(((pcm.len() as f64 / sample_rate as f64) * 1000.0).round() as u64)
    }
}

struct ProbedStream {
    format: Box<dyn symphonia::core::formats::FormatReader>,
}

fn canonicalize_existing(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("input path cannot be empty".to_string());
    }
    let candidate = Path::new(trimmed);
    if !candidate.exists() {
        return Err(format!("input path does not exist: `{trimmed}`"));
    }
    std::fs::canonicalize(candidate).map_err(|e| format!("failed to resolve `{trimmed}`: {e}"))
}

fn classify_session_dir(dir: &Path) -> Option<(PathBuf, Option<PathBuf>)> {
    let mic = dir.join("mic.wav");
    if !mic.is_file() {
        return None;
    }
    let speaker = dir.join("speaker.wav");
    let has_session_metadata =
        dir.join("session.json").is_file() || dir.join("notes.json").is_file();
    if speaker.is_file() {
        return Some((mic, Some(speaker)));
    }
    if has_session_metadata {
        return Some((mic, None));
    }
    // Scribe single-source folders in this repo can still contain only mic.wav.
    Some((mic, None))
}

fn collect_audio_files_recursively(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_audio_files_recursively(&path, out)?;
            continue;
        }
        if is_supported_audio_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_supported_audio_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "wav" | "mp3" | "m4a" | "ogg" | "flac"
    )
}

fn probe_stream(path: &Path) -> Result<ProbedStream, String> {
    let file = File::open(path).map_err(|e| format!("failed to open `{}`: {e}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }
    let probed = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("failed to probe `{}`: {e}", path.display()))?;
    Ok(ProbedStream {
        format: probed.format,
    })
}

fn decode_audio_file(path: &Path) -> Result<(Vec<f32>, u32), String> {
    let file = File::open(path).map_err(|e| format!("failed to open `{}`: {e}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("failed to parse `{}`: {e}", path.display()))?
        .format;

    let track = format
        .default_track()
        .ok_or_else(|| format!("no audio track found in `{}`", path.display()))?;
    if track.codec_params.codec == CODEC_TYPE_NULL {
        return Err(format!("unsupported codec in `{}`", path.display()));
    }

    let mut decoder = get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("failed to initialize decoder for `{}`: {e}", path.display()))?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| format!("missing sample-rate metadata for `{}`", path.display()))?;

    let mut mono_pcm: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err)) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => {
                return Err(format!(
                    "failed while reading audio packets from `{}`: {err}",
                    path.display()
                ));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(err)) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => {
                return Err(format!("failed to decode `{}`: {err}", path.display()));
            }
        };

        let channels = decoded.spec().channels.count();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.frames() as u64, *decoded.spec());
        sample_buf.copy_interleaved_ref(decoded);
        let interleaved = sample_buf.samples();
        if channels <= 1 {
            mono_pcm.extend_from_slice(interleaved);
            continue;
        }

        for frame in interleaved.chunks(channels) {
            let sum: f32 = frame.iter().copied().sum();
            mono_pcm.push(sum / channels as f32);
        }
    }

    Ok((mono_pcm, sample_rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_wav(path: &Path, sample_rate: u32, seconds: f32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        let total = (sample_rate as f32 * seconds).round() as usize;
        for _ in 0..total {
            writer.write_sample(0_i16).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    #[test]
    fn expand_inputs_accepts_single_audio_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = dir.path().join("clip.wav");
        write_test_wav(&wav, 16_000, 1.0);

        let service = TranscribeInputService;
        let items = service
            .expand_inputs(&[wav.to_string_lossy().to_string()])
            .expect("expand inputs");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_type, TranscribeSourceType::SingleAudio);
        assert!(items[0].duration_ms >= 900);
    }

    #[test]
    fn expand_inputs_recognizes_dual_source_session_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mic = dir.path().join("mic.wav");
        let speaker = dir.path().join("speaker.wav");
        write_test_wav(&mic, 16_000, 1.0);
        write_test_wav(&speaker, 16_000, 0.5);

        let service = TranscribeInputService;
        let items = service
            .expand_inputs(&[dir.path().to_string_lossy().to_string()])
            .expect("expand session");
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].source_type,
            TranscribeSourceType::DualSourceSession
        );
        assert!(items[0].speaker_path.is_some());
    }

    #[test]
    fn expand_inputs_collects_supported_files_from_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let a = dir.path().join("a.wav");
        let nested = dir.path().join("nested");
        std::fs::create_dir_all(&nested).expect("mkdir");
        let b = nested.join("b.wav");
        write_test_wav(&a, 16_000, 0.2);
        write_test_wav(&b, 16_000, 0.2);
        std::fs::write(dir.path().join("ignore.txt"), "nope").expect("write txt");

        let service = TranscribeInputService;
        let items = service
            .expand_inputs(&[dir.path().to_string_lossy().to_string()])
            .expect("expand folder");
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.source_type == TranscribeSourceType::SingleAudio));
    }

    #[test]
    fn decode_input_keeps_original_source_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = dir.path().join("source.wav");
        write_test_wav(&wav, 16_000, 0.5);

        let service = TranscribeInputService;
        let items = service
            .expand_inputs(&[wav.to_string_lossy().to_string()])
            .expect("expand");
        let decoded = service.decode_input(&items[0]).expect("decode");
        assert!(!decoded.mic_pcm_16k.is_empty());
        assert!(
            wav.exists(),
            "source file must not be deleted by transcribe"
        );
    }
}
