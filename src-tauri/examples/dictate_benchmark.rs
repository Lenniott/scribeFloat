//! Local, offline Dictate replay. No microphone, clipboard, or Note writes.
//! Arguments: WAV OUTPUT_JSON [simulated|realtime|cuts] [realtime_runs] [repeats].
//! Repeats insert one second of silence and are a synthetic scheduling fixture.
use anyhow::{Context, Result};
use scribefloat_lib::services::{
    audio::{pcm_as_saved_wav, resample_linear},
    model::ModelService,
    output::format_dictate_segments,
    streaming_transcription::{transcribe_phrase, PhraseSegmenter, StreamingTranscription},
    transcription::{
        run_post_capture_transcription, CaptureAudio, CaptureProfile, PostCaptureInput,
    },
};
use std::{path::PathBuf, time::Instant};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<String> = std::env::args().collect();
    let wav = PathBuf::from(
        args.get(1)
            .context("usage: dictate_benchmark WAV OUTPUT_JSON")?,
    );
    let output = PathBuf::from(args.get(2).context("missing OUTPUT_JSON")?);
    let model = ModelService::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bundled-models"));
    let path = model.default_model_path();
    let read = Instant::now();
    let mut reader = hound::WavReader::open(&wav)?;
    let spec = reader.spec();
    anyhow::ensure!(
        spec.channels == 1
            && spec.sample_format == hound::SampleFormat::Int
            && spec.bits_per_sample == 16,
        "fixture must be mono int16 WAV"
    );
    let native = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / 32768.0))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut pcm = resample_linear(&native, spec.sample_rate, 16000);
    let repeats = args
        .get(5)
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(1);
    anyhow::ensure!(
        repeats > 0 && repeats <= 20,
        "repeats must be between 1 and 20"
    );
    let clip = pcm.clone();
    for _ in 1..repeats {
        pcm.extend(std::iter::repeat_n(0.0, 16000));
        pcm.extend_from_slice(&clip);
    }
    let capture_pcm = pcm;
    let pcm = pcm_as_saved_wav(&capture_pcm);
    let read_ms = read.elapsed().as_secs_f64() * 1000.0;
    if args.get(3).is_some_and(|s| s == "cuts") {
        let vad = model
            .vad_path_for_pcm(pcm.len())?
            .context("fixture too short")?;
        let mut segmenter = PhraseSegmenter::new(&vad)?;
        let mut phrases = segmenter.push(&pcm)?;
        if let Some(tail) = segmenter.finish() {
            phrases.push(tail);
        }
        for phrase in phrases {
            eprintln!("CUT {} {}", phrase.start_sample, phrase.ready_sample);
        }
        return Ok(());
    }
    model.preload_context(&path);
    let mut runs = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let result = run_post_capture_transcription(
            &model,
            PostCaptureInput {
                profile: CaptureProfile::Dictate,
                audio: CaptureAudio {
                    mic_pcm_16k: &pcm,
                    speaker_pcm_16k: None,
                },
                model_path: &path,
                speaker_evidence: None,
                abort: None,
                on_model_loaded: None,
            },
            |_| {},
        )?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "BENCH baseline audio_secs={:.3} elapsed_ms={elapsed_ms:.1}",
            pcm.len() as f64 / 16000.0
        );
        runs.push(serde_json::json!({"elapsed_ms": elapsed_ms, "text": result.dictate_text, "segments": result.segments}));
    }
    let vad = model
        .vad_path_for_pcm(pcm.len())?
        .context("fixture too short")?;
    let mut segmenter = PhraseSegmenter::new(&vad)?;
    let detection_start = Instant::now();
    let mut phrases = segmenter.push(&pcm)?;
    if let Some(tail) = segmenter.finish() {
        phrases.push(tail);
    }
    let detection_ms = detection_start.elapsed().as_secs_f64() * 1000.0;
    let mut ready_at = 0.0_f64;
    let mut segments = Vec::new();
    let mut jobs = Vec::new();
    for phrase in phrases {
        let available_ms = phrase.ready_sample as f64 / 16.0;
        let audio_ms = phrase.pcm.len() as f64 / 16.0;
        let start = Instant::now();
        segments.extend(transcribe_phrase(
            &model,
            &path,
            phrase,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )?);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        ready_at = ready_at.max(available_ms) + elapsed_ms;
        jobs.push(serde_json::json!({"available_ms":available_ms,"audio_ms":audio_ms,"elapsed_ms":elapsed_ms}));
    }
    let stop_wait_ms = (ready_at - pcm.len() as f64 / 16.0).max(0.0);
    eprintln!("BENCH streamed estimated_stop_wait_ms={stop_wait_ms:.1}");
    let mut realtime = Vec::new();
    if args.get(3).is_some_and(|s| s == "realtime") {
        let count = args
            .get(4)
            .map(|s| s.parse::<usize>())
            .transpose()?
            .unwrap_or(3);
        for _ in 0..count {
            let live = StreamingTranscription::start(std::sync::Arc::clone(&model), path.clone())?;
            let tap = live.tap();
            let recording = Instant::now();
            let mut fed = 0;
            for packet in capture_pcm.chunks(1280) {
                fed += packet.len();
                let due = std::time::Duration::from_secs_f64(fed as f64 / 16000.0);
                std::thread::sleep(due.saturating_sub(recording.elapsed()));
                tap(packet);
            }
            let stopped = Instant::now();
            let result = live.finish()?.segments;
            let wait_ms = stopped.elapsed().as_secs_f64() * 1000.0;
            eprintln!("BENCH realtime stop_wait_ms={wait_ms:.1}");
            realtime.push(serde_json::json!({"stop_wait_ms":wait_ms,"text":format_dictate_segments(&result),"segments":result}));
        }
    }
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&serde_json::json!({
            "audio_secs": pcm.len() as f64 / 16000.0, "fixture_repeats": repeats, "read_ms": read_ms, "baseline": runs,
            "streamed": {"detection_ms":detection_ms,"estimated_stop_wait_ms":stop_wait_ms,"jobs":jobs,"text":format_dictate_segments(&segments),"segments":segments},
            "realtime": realtime
        }))?,
    )?;
    model.release_contexts();
    Ok(())
}
