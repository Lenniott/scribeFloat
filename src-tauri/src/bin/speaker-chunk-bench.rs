//! Offline benchmark for speaker-aware chunking recall.
//!
//! Usage:
//!   cargo run --features bench --bin speaker-chunk-bench -- /path/to/test_audio.wav
//!
//! Truth times for the pitch_test fixture: 7.67, 15.09, 22.5, 32.42, 42.55 s
//! Target: ≥4/5 hits within 1.0 s tolerance.

use scribefloat_lib::{find_cuts, load_wav_pcm_16k, score_cuts, CHUNKING_SAMPLE_RATE};
use std::env;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let path = env::args()
        .nth(1)
        .map(|p| Path::new(&p).to_path_buf())
        .or_else(|| {
            env::var("SPEAKER_CHUNKING_FIXTURE")
                .ok()
                .map(|p| Path::new(&p).to_path_buf())
        })
        .ok_or_else(|| anyhow::anyhow!("pass WAV path or set SPEAKER_CHUNKING_FIXTURE"))?;

    let pcm = load_wav_pcm_16k(&path)?;
    let duration_s = pcm.len() as f64 / CHUNKING_SAMPLE_RATE as f64;
    let cuts = find_cuts(&pcm, CHUNKING_SAMPLE_RATE, true);
    let truth = [7.67, 15.09, 22.5, 32.42, 42.55];
    let score = score_cuts(&cuts, &truth, 1.0);

    println!("file: {}", path.display());
    println!("duration: {duration_s:.1}s");
    println!("cuts: {}", cuts.len());
    for cut in &cuts {
        println!(
            "  {:.2}s {:?} strength={:.1}",
            cut.time_s, cut.evidence, cut.strength
        );
    }
    println!(
        "recall: {}/{} (extra {})",
        score.caught, score.total_truth, score.extra
    );

    if score.caught < 4 {
        anyhow::bail!("recall below target (need ≥4/5)");
    }
    Ok(())
}
