/// Voice probe — find a good speaker-similarity threshold for the "is this me?" check.
///
/// Usage:
///   1. Run ./setup.sh to download the embedding model.
///   2. Put WAV files of your own voice in test-audio/me/
///   3. Put WAV files of other speakers in test-audio/other/
///   4. cargo run
///
/// WAVs should be 16 kHz mono, at least 2 seconds of clean speech.
/// Record yourself at different mic/distance combos; same for other speakers.
use anyhow::{anyhow, Context, Result};
use sherpa_onnx::{SpeakerEmbeddingExtractor, SpeakerEmbeddingExtractorConfig};
use std::path::{Path, PathBuf};

const DEFAULT_MODEL: &str = "3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx";
const ME_DIR: &str = "test-audio/me";
const OTHER_DIR: &str = "test-audio/other";

struct Row {
    label: &'static str,
    name: String,
    sim: f32,
    dur_s: f32,
}

fn main() -> Result<()> {
    let model_path = std::env::var("VOICE_PROBE_MODEL")
        .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    if !Path::new(&model_path).exists() {
        eprintln!("Model not found: {model_path}");
        eprintln!("Run ./setup.sh to download it.");
        eprintln!("Or: VOICE_PROBE_MODEL=/path/to/model.onnx cargo run");
        std::process::exit(1);
    }

    let config = SpeakerEmbeddingExtractorConfig {
        model: Some(model_path.clone()),
        num_threads: 2,
        debug: false,
        provider: Some("cpu".into()),
    };
    let extractor = SpeakerEmbeddingExtractor::create(&config)
        .ok_or_else(|| anyhow!("failed to initialise extractor — is the model file valid?"))?;

    println!("Model : {model_path}");
    println!("Dim   : {}", extractor.dim());

    let me_files = wav_files_in(Path::new(ME_DIR))?;
    let other_files = wav_files_in(Path::new(OTHER_DIR))?;

    if me_files.is_empty() {
        eprintln!("\nNo WAV files in {ME_DIR}/  — add recordings of your voice and re-run.");
        std::process::exit(1);
    }

    // ── Build reference from all ME files ────────────────────────────────────
    println!("\nEnrolling from {} file(s) in {ME_DIR}/", me_files.len());
    let mut me_pairs: Vec<(PathBuf, Vec<f32>)> = vec![];
    for path in &me_files {
        match embed(&extractor, path) {
            Ok(e) => {
                println!("  ok  {}", fname(path));
                me_pairs.push((path.clone(), e));
            }
            Err(e) => eprintln!("  SKIP  {} — {e}", fname(path)),
        }
    }
    if me_pairs.is_empty() {
        eprintln!("No usable ME embeddings. Files may be too short (need ≥ 2 s of speech).");
        std::process::exit(1);
    }
    let reference = mean_l2_normalised(me_pairs.iter().map(|(_, e)| e));

    // ── Score every file ─────────────────────────────────────────────────────
    let mut rows: Vec<Row> = vec![];

    for (path, emb) in &me_pairs {
        rows.push(Row {
            label: "ME",
            name: fname(path),
            sim: cosine(emb, &reference),
            dur_s: wav_dur_s(path).unwrap_or(0.0),
        });
    }
    for path in &other_files {
        match embed(&extractor, path) {
            Ok(emb) => rows.push(Row {
                label: "OTHER",
                name: fname(path),
                sim: cosine(&emb, &reference),
                dur_s: wav_dur_s(path).unwrap_or(0.0),
            }),
            Err(e) => eprintln!("  SKIP  {} — {e}", fname(path)),
        }
    }

    // ── Similarity table ─────────────────────────────────────────────────────
    println!("\n{:─<70}", "");
    println!("  {:<7} {:<40} {:>7}  {:>6}", "LABEL", "FILE", "SIM", "DUR(s)");
    println!("{:─<70}", "");
    for r in &rows {
        println!("  [{:<5}] {:<40} {:>7.3}  {:>6.1}", r.label, r.name, r.sim, r.dur_s);
    }
    println!("{:─<70}", "");

    let me_sims: Vec<f32> = rows.iter().filter(|r| r.label == "ME").map(|r| r.sim).collect();
    let other_sims: Vec<f32> = rows.iter().filter(|r| r.label == "OTHER").map(|r| r.sim).collect();

    println!(
        "\nME    range : {:.3} – {:.3}",
        me_sims.iter().cloned().fold(f32::MAX, f32::min),
        me_sims.iter().cloned().fold(f32::MIN, f32::max),
    );

    if other_sims.is_empty() {
        println!("\nNo OTHER files — add other-speaker WAVs to {OTHER_DIR}/ for threshold analysis.");
        write_csv(&rows)?;
        return Ok(());
    }

    let max_other = other_sims.iter().cloned().fold(f32::MIN, f32::max);
    let min_other = other_sims.iter().cloned().fold(f32::MAX, f32::min);
    println!("OTHER range : {:.3} – {:.3}", min_other, max_other);

    let gap = me_sims.iter().cloned().fold(f32::MAX, f32::min) - max_other;
    let quality = if gap > 0.30 {
        "excellent — clean separation"
    } else if gap > 0.15 {
        "good"
    } else if gap > 0.00 {
        "marginal — add more varied samples"
    } else {
        "OVERLAPPING — add more samples or try a different model"
    };
    println!("Gap (min-ME minus max-OTHER) : {:+.3}  {quality}", gap);

    // ── Threshold sweep ───────────────────────────────────────────────────────
    println!("\n── Threshold Sweep ─────────────────────────────────────────────────────");
    println!(
        "  {:>9}  {:>10}  {:>12}  {:>8}  {:>11}  {:>6}",
        "threshold", "me_pass", "other_block", "me_miss", "other_leak", "F1"
    );
    println!("{:─<70}", "");

    let mut best_t = 0.5f32;
    let mut best_f1 = -1.0f32;

    for step in 0..=20u32 {
        let t = step as f32 * 0.05;
        let tp = me_sims.iter().filter(|&&s| s >= t).count();
        let fp = other_sims.iter().filter(|&&s| s >= t).count();
        let tn = other_sims.iter().filter(|&&s| s < t).count();
        let fn_ = me_sims.iter().filter(|&&s| s < t).count();
        let prec = if tp + fp > 0 { tp as f32 / (tp + fp) as f32 } else { 1.0 };
        let rec = if tp + fn_ > 0 { tp as f32 / (tp + fn_) as f32 } else { 0.0 };
        let f1 = if prec + rec > 0.0 {
            2.0 * prec * rec / (prec + rec)
        } else {
            0.0
        };
        println!(
            "  {:>9.2}  {:>4}/{:<5}  {:>5}/{:<6}  {:>8}  {:>11}  {:>6.3}",
            t,
            tp,
            me_sims.len(),
            tn,
            other_sims.len(),
            fn_,
            fp,
            f1
        );
        if f1 > best_f1 {
            best_f1 = f1;
            best_t = t;
        }
    }

    println!("\nSuggested threshold : {best_t:.2}  (F1 = {best_f1:.3})");
    println!(
        "Tip: prefer the highest threshold where all ME files still pass — \
        missing your own speech is less disruptive than attributing others to you."
    );

    write_csv(&rows)?;
    Ok(())
}

// ── Embedding ────────────────────────────────────────────────────────────────

fn embed(extractor: &SpeakerEmbeddingExtractor, path: &Path) -> Result<Vec<f32>> {
    let mut reader =
        hound::WavReader::open(path).with_context(|| format!("open {}", path.display()))?;
    let spec = reader.spec();

    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|x| x as f32 / 32768.0))
            .collect::<std::result::Result<_, _>>()
            .context("read i16 samples")?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<_, _>>()
            .context("read f32 samples")?,
    };

    let ch = spec.channels as usize;
    let mono: Vec<f32> = if ch == 1 {
        interleaved
    } else {
        interleaved
            .chunks(ch)
            .map(|frame| frame.iter().sum::<f32>() / ch as f32)
            .collect()
    };

    let stream = extractor
        .create_stream()
        .ok_or_else(|| anyhow!("create_stream failed"))?;
    stream.accept_waveform(spec.sample_rate as i32, &mono);
    stream.input_finished();

    if !extractor.is_ready(&stream) {
        return Err(anyhow!(
            "{} is too short — need ≥ 2 s of speech",
            path.display()
        ));
    }

    extractor
        .compute(&stream)
        .ok_or_else(|| anyhow!("compute embedding failed for {}", path.display()))
}

// ── Maths ────────────────────────────────────────────────────────────────────

fn mean_l2_normalised<'a>(embeddings: impl Iterator<Item = &'a Vec<f32>>) -> Vec<f32> {
    let mut mean: Vec<f32> = vec![];
    let mut count = 0usize;
    for e in embeddings {
        if mean.is_empty() {
            mean = vec![0.0f32; e.len()];
        }
        for (m, v) in mean.iter_mut().zip(e.iter()) {
            *m += v;
        }
        count += 1;
    }
    if count == 0 {
        return mean;
    }
    mean.iter_mut().for_each(|v| *v /= count as f32);
    let mag: f32 = mean.iter().map(|v| v * v).sum::<f32>().sqrt();
    if mag > 0.0 {
        mean.iter_mut().for_each(|v| *v /= mag);
    }
    mean
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let ma: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if ma > 0.0 && mb > 0.0 { dot / (ma * mb) } else { 0.0 }
}

// ── WAV utils ────────────────────────────────────────────────────────────────

fn wav_files_in(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("read dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("wav"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    Ok(files)
}

fn wav_dur_s(path: &Path) -> Option<f32> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let samples = reader.len();
    let ch = spec.channels as u32;
    if ch == 0 || spec.sample_rate == 0 {
        return None;
    }
    Some(samples as f32 / (spec.sample_rate as f32 * ch as f32))
}

fn fname(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

// ── Output ───────────────────────────────────────────────────────────────────

fn write_csv(rows: &[Row]) -> Result<()> {
    let path = "voice-probe-results.csv";
    let mut out = String::from("label,file,similarity,duration_s\n");
    for r in rows {
        out.push_str(&format!(
            "{},{},{:.4},{:.1}\n",
            r.label, r.name, r.sim, r.dur_s
        ));
    }
    std::fs::write(path, &out).context("write voice-probe-results.csv")?;
    println!("\nRaw scores → {path}");
    Ok(())
}
