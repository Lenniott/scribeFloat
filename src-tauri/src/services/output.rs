use crate::types::Segment;
use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct OutputService;

impl OutputService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }

    /// Create a timestamped session directory inside save_folder.
    pub fn make_session_dir(&self, save_folder: &str) -> Result<PathBuf> {
        let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let dir = PathBuf::from(save_folder).join(&ts);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Build the transcript file path: `{session_dir}/{ts}_{model_stem}.md`
    pub fn transcript_path(&self, session_dir: &Path, model_path: &Path) -> PathBuf {
        let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let stem = model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "model".to_string());
        session_dir.join(format!("{}_{}.md", ts, stem))
    }

    /// Write mono f32 PCM as a 16-bit WAV file.
    pub fn write_wav(&self, pcm: &[f32], sample_rate: u32, dest: &Path) -> Result<()> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer =
            WavWriter::create(dest, spec).context("failed to create WAV writer")?;
        for &s in pcm {
            let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(())
    }

    /// Render segments as markdown and write. Verifies file is non-empty before returning Ok.
    pub fn write_transcript(
        &self,
        segments: &[Segment],
        title: &str,
        dest: &Path,
    ) -> Result<PathBuf> {
        let mut md = format!("# {}\n\n", title);
        for seg in segments {
            md.push_str(&format!("[{}] {}\n\n", format_ms(seg.start_ms), seg.text));
        }
        std::fs::write(dest, &md).context("failed to write transcript")?;
        if std::fs::metadata(dest)?.len() == 0 {
            return Err(anyhow::anyhow!("transcript was written empty"));
        }
        Ok(dest.to_path_buf())
    }

    /// Delete a WAV file. Silent no-op if it no longer exists.
    pub fn delete_wav(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn format_ms(ms: i64) -> String {
    let total = ms / 1000;
    format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
}
