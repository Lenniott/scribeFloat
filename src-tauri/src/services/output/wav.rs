use anyhow::{anyhow, Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

/// Write mono f32 PCM as a 16-bit WAV file.
pub fn write_wav(pcm: &[f32], sample_rate: u32, dest: &Path) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(dest, spec).context("failed to create WAV writer")?;
    for &s in pcm {
        let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

/// Write a placeholder 16-bit PCM WAV header for streaming capture.
pub fn write_streaming_wav_placeholder(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
) -> Result<()> {
    sync_wav_header(path, sample_rate, channels, bits_per_sample, 0)
}

/// Patch RIFF/data chunk sizes for a 16-bit PCM WAV without finalizing the writer.
pub fn sync_wav_header(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    sample_count: u64,
) -> Result<()> {
    let block_align = channels as u32 * (bits_per_sample as u32 / 8);
    let byte_rate = sample_rate * block_align;
    let data_size = sample_count * block_align as u64;
    let riff_size = 36 + data_size;

    if sample_count == 0 {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .context("open wav for header init")?;
        let mut file = std::io::BufWriter::new(file);
        file.write_all(b"RIFF")?;
        file.write_all(&(36u32).to_le_bytes())?;
        file.write_all(b"WAVEfmt ")?;
        file.write_all(&16u32.to_le_bytes())?;
        file.write_all(&1u16.to_le_bytes())?; // PCM
        file.write_all(&channels.to_le_bytes())?;
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&(block_align as u16).to_le_bytes())?;
        file.write_all(&bits_per_sample.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&0u32.to_le_bytes())?;
        file.flush()?;
        file.into_inner()?.sync_all()?;
    } else {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .context("open wav for header patch")?;
        file.seek(SeekFrom::Start(4))?;
        file.write_all(&(riff_size as u32).to_le_bytes())?;
        file.seek(SeekFrom::Start(40))?;
        file.write_all(&(data_size as u32).to_le_bytes())?;
        file.sync_all()?;
    }
    Ok(())
}

/// Infer sample count from on-disk byte length and rewrite the WAV header.
pub fn repair_wav_header_from_file_size(path: &Path) -> Result<u64> {
    let len = std::fs::metadata(path).context("stat wav")?.len();
    if len <= 44 {
        return Err(anyhow!("wav file too small to repair"));
    }
    let sample_count = (len - 44) / 2;
    sync_wav_header(
        path,
        crate::services::audio::WHISPER_SAMPLE_RATE,
        1,
        16,
        sample_count,
    )?;
    Ok(sample_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("wav-tests-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn streaming_wav_placeholder_is_valid_wav() {
        let dir = wav_dir();
        let path = dir.join("placeholder.wav");
        write_streaming_wav_placeholder(&path, 16_000, 1, 16).unwrap();
        let reader = hound::WavReader::open(&path).expect("open placeholder");
        assert_eq!(reader.spec().bits_per_sample, 16);
    }

    #[test]
    fn sync_wav_header_makes_checkpointed_wav_readable() {
        use std::io::Write;
        let dir = wav_dir();
        let path = dir.join("partial.wav");
        write_streaming_wav_placeholder(&path, 16_000, 1, 16).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&vec![0u8; 3200]).unwrap();
        sync_wav_header(&path, 16_000, 1, 16, 1600).unwrap();
        let pcm = crate::services::audio::read_wav_mono_f32(&path)
            .expect("read checkpointed wav");
        assert_eq!(pcm.len(), 1600);
    }

    #[test]
    fn repair_wav_header_from_file_size_roundtrip() {
        use std::io::Write;
        let dir = wav_dir();
        let path = dir.join("truncated.wav");
        write_streaming_wav_placeholder(&path, 16_000, 1, 16).unwrap();
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&vec![0u8; 6400]).unwrap();
        repair_wav_header_from_file_size(&path).expect("repair");
        let pcm = crate::services::audio::read_wav_mono_f32(&path)
            .expect("read repaired wav");
        assert_eq!(pcm.len(), 3200);
    }
}
