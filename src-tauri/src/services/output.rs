use crate::types::{Note, Segment};
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
        notes: &[Note],
        title: &str,
        model_name: &str,
        include_timestamps: bool,
        dest: &Path,
    ) -> Result<PathBuf> {
        let transcript_body = segments
            .iter()
            .map(|seg| {
                if include_timestamps {
                    format!("[{}] {}", format_ms(seg.start_ms), seg.text)
                } else {
                    seg.text.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let duration_seconds = segments
            .last()
            .map(|s| s.end_ms.max(0) as f64 / 1000.0)
            .unwrap_or(0.0);
        let word_count = transcript_body.split_whitespace().count();
        let token_estimate = ((word_count as f64) * 1.3).round() as usize;

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("title: '{}'\n", title.replace('\'', "’")));
        md.push_str(&format!("duration_seconds: {:.1}\n", duration_seconds));
        md.push_str(&format!("word_count: {word_count}\n"));
        md.push_str(&format!("token_estimate: {token_estimate}\n"));
        md.push_str(&format!("model: {model_name}\n"));
        md.push_str("---\n\n");
        md.push_str("## Transcript\n\n");
        md.push_str(&transcript_body);

        if !notes.is_empty() {
            md.push_str("\n\n## Notes\n");
            for (i, note) in notes.iter().enumerate() {
                md.push_str(&format!(
                    "[{}] ({}) {}\n",
                    i + 1,
                    format_ms(note.recorded_at_ms as i64),
                    note.text
                ));
            }
        }

        md.push('\n');
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
