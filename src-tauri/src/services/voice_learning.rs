//! Quality-gated global voiceprint learning from transcript evidence
//! (story 0063). A confirmed session speaker's clean-chunk centroid may be
//! added to the matching global profile — but only when audio quality,
//! cluster tightness, and discriminability all clear their gates, so one
//! noisy meeting can never drag a profile off course.

use crate::services::speaker_chunks::cosine;
use crate::types::{ProfileEvidence, SessionSpeaker, SpeakerChunk, VoiceprintProfile};
use anyhow::{anyhow, Result};
use serde::Serialize;

/// Audio gate: the speaker group needs this much clean speech.
const MIN_CLEAN_SPEECH_S: f32 = 6.0;
/// Audio gate: duration-weighted mean VAD purity across clean chunks.
const MIN_MEAN_PURITY: f32 = 0.80;
/// Cluster gate: clean chunks must agree with their own centroid.
const MIN_MEAN_SCORE: f32 = 0.85;
/// Cluster gate: a loose cluster is probably a mixed speaker group.
const MAX_SCORE_STD_DEV: f32 = 0.06;
/// Discriminability gate: clearly separated from other session speakers.
const MIN_MEAN_MARGIN: f32 = 0.15;
/// Discriminability gate: the target profile must beat the next closest
/// saved profile by this much when matching the session centroid.
const MIN_PROFILE_MARGIN: f32 = 0.05;

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceGateReport {
    pub eligible: bool,
    /// Human-readable gate failures; empty when eligible.
    pub reasons: Vec<String>,
    pub clean_duration_s: f32,
    pub mean_purity: f32,
    pub mean_score: Option<f32>,
    pub score_std_dev: Option<f32>,
    pub mean_margin: Option<f32>,
}

/// Evaluate whether a session speaker's clean-chunk evidence is good enough
/// to improve the global profile it is being attributed to.
pub fn evaluate_speaker_evidence(
    speaker: &SessionSpeaker,
    chunks: &[SpeakerChunk],
    target_profile: Option<&VoiceprintProfile>,
    all_profiles: &[VoiceprintProfile],
) -> EvidenceGateReport {
    let clean: Vec<&SpeakerChunk> = speaker
        .clean_chunk_ids
        .iter()
        .filter_map(|id| chunks.iter().find(|chunk| &chunk.id == id))
        .collect();
    let group_has_clipping = chunks.iter().any(|chunk| {
        chunk.cluster_id.as_deref() == Some(&speaker.session_speaker_id) && chunk.clipping
    });

    let total_duration_ms: u64 = clean
        .iter()
        .map(|c| c.end_ms.saturating_sub(c.start_ms))
        .sum();
    let clean_duration_s = total_duration_ms as f32 / 1000.0;
    let mean_purity = if total_duration_ms == 0 {
        0.0
    } else {
        clean
            .iter()
            .map(|c| c.vad_purity * c.end_ms.saturating_sub(c.start_ms) as f32)
            .sum::<f32>()
            / total_duration_ms as f32
    };

    let scores: Vec<f32> = clean.iter().filter_map(|c| c.session_score).collect();
    let mean_score = mean(&scores);
    let score_std_dev = std_dev(&scores);
    let margins: Vec<f32> = clean.iter().filter_map(|c| c.margin).collect();
    let mean_margin = mean(&margins);

    let mut reasons = Vec::new();
    if clean_duration_s < MIN_CLEAN_SPEECH_S {
        reasons.push(format!(
            "not enough clean speech ({clean_duration_s:.1}s < {MIN_CLEAN_SPEECH_S:.0}s)"
        ));
    }
    if mean_purity < MIN_MEAN_PURITY {
        reasons.push(format!(
            "mean VAD purity too low ({mean_purity:.2} < {MIN_MEAN_PURITY:.2})"
        ));
    }
    if group_has_clipping {
        reasons.push("speaker group contains clipped audio".to_string());
    }
    match mean_score {
        Some(score) if score < MIN_MEAN_SCORE => reasons.push(format!(
            "clean chunks do not agree with their centroid (mean score {score:.2} < {MIN_MEAN_SCORE:.2})"
        )),
        None => reasons.push("no scored clean chunks to agree on".to_string()),
        _ => {}
    }
    if let Some(std) = score_std_dev {
        if std > MAX_SCORE_STD_DEV {
            reasons.push(format!(
                "cluster is not tight (score std dev {std:.3} > {MAX_SCORE_STD_DEV:.2}) — possibly a mixed speaker group"
            ));
        }
    }
    match mean_margin {
        Some(margin) if margin < MIN_MEAN_MARGIN => reasons.push(format!(
            "not clearly separated from other session speakers (mean margin {margin:.2} < {MIN_MEAN_MARGIN:.2})"
        )),
        None => reasons.push("no margin data for clean chunks".to_string()),
        _ => {}
    }
    if let Some(target) = target_profile {
        let target_score = cosine(&speaker.centroid_embedding, &target.embedding);
        let best_other = all_profiles
            .iter()
            .filter(|p| {
                p.slug != target.slug && p.embedding.len() == speaker.centroid_embedding.len()
            })
            .map(|p| cosine(&speaker.centroid_embedding, &p.embedding))
            .fold(None::<f32>, |acc, s| Some(acc.map_or(s, |a| a.max(s))));
        if let Some(best_other) = best_other {
            if target_score - best_other < MIN_PROFILE_MARGIN {
                reasons.push(format!(
                    "target profile does not beat the next closest profile clearly ({:.2} vs {:.2})",
                    target_score, best_other
                ));
            }
        }
    }

    EvidenceGateReport {
        eligible: reasons.is_empty(),
        reasons,
        clean_duration_s,
        mean_purity,
        mean_score,
        score_std_dev,
        mean_margin,
    }
}

/// Add (or refresh) one session's evidence on a profile and rebuild the
/// global embedding from enrollment prints plus all accepted evidence.
pub fn apply_evidence(profile: &mut VoiceprintProfile, evidence: ProfileEvidence) -> Result<()> {
    if evidence.centroid_embedding.is_empty() {
        return Err(anyhow!("evidence centroid cannot be empty"));
    }
    if !profile.embedding.is_empty() && profile.embedding.len() != evidence.centroid_embedding.len()
    {
        return Err(anyhow!(
            "evidence dimension mismatch: profile={}, evidence={}",
            profile.embedding.len(),
            evidence.centroid_embedding.len()
        ));
    }

    if profile.enrollment_embedding.is_none() {
        profile.enrollment_embedding = Some(profile.embedding.clone());
    }

    profile.evidence.retain(|existing| {
        !(existing.note_id == evidence.note_id
            && existing.session_speaker_id == evidence.session_speaker_id)
    });
    profile.evidence.push(evidence);
    rebuild_embedding(profile);
    profile.updated_at = chrono::Utc::now();
    Ok(())
}

/// Global centroid = enrollment average weighted by its sample count plus one
/// vote per accepted evidence centroid. Deterministic from stored parts.
fn rebuild_embedding(profile: &mut VoiceprintProfile) {
    let enrollment = profile
        .enrollment_embedding
        .as_deref()
        .unwrap_or(&profile.embedding);
    if enrollment.is_empty() {
        return;
    }
    let enrollment_weight = profile.sample_count.max(1) as f32;
    let mut combined: Vec<f32> = enrollment.iter().map(|v| v * enrollment_weight).collect();
    for record in &profile.evidence {
        if record.centroid_embedding.len() != combined.len() {
            continue;
        }
        for (dst, value) in combined.iter_mut().zip(record.centroid_embedding.iter()) {
            *dst += value;
        }
    }
    let mag = combined.iter().map(|v| v * v).sum::<f32>().sqrt();
    if mag > 0.0 {
        combined.iter_mut().for_each(|v| *v /= mag);
    }
    profile.embedding = combined;
}

fn mean(values: &[f32]) -> Option<f32> {
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

fn std_dev(values: &[f32]) -> Option<f32> {
    let mean = mean(values)?;
    let variance =
        values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / values.len() as f32;
    Some(variance.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LabelCorrection, SessionSpeaker, SpeakerChunk};

    fn norm(mut v: Vec<f32>) -> Vec<f32> {
        let mag = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            v.iter_mut().for_each(|x| *x /= mag);
        }
        v
    }

    fn chunk(id: &str, start: u64, end: u64, emb: Vec<f32>, cluster: &str) -> SpeakerChunk {
        SpeakerChunk {
            id: id.into(),
            start_ms: start,
            end_ms: end,
            label: "Alice".into(),
            cluster_id: Some(cluster.into()),
            matched_profile: None,
            embedding: Some(norm(emb)),
            encrypted_embedding: None,
            audio_duration_s: (end - start) as f32 / 1000.0,
            vad_purity: 0.95,
            rms_energy: 0.1,
            clipping: false,
            profile_score: None,
            session_score: Some(0.95),
            margin: Some(0.3),
            corrections: Vec::new(),
        }
    }

    fn speaker(chunk_ids: &[&str], centroid: Vec<f32>) -> SessionSpeaker {
        SessionSpeaker {
            session_speaker_id: "speaker-1".into(),
            label: "Alice".into(),
            centroid_embedding: norm(centroid),
            encrypted_centroid_embedding: None,
            clean_chunk_ids: chunk_ids.iter().map(|s| s.to_string()).collect(),
            start_ms: 0,
            end_ms: 10_000,
            duration_ms: 10_000,
            radius: 0.05,
            quality_score: 0.9,
            user_confirmed: true,
        }
    }

    fn profile(name: &str, emb: Vec<f32>) -> VoiceprintProfile {
        VoiceprintProfile {
            name: name.into(),
            slug: name.to_lowercase(),
            mic_device_id: None,
            embedding: norm(emb),
            encrypted_embedding: None,
            sample_count: 2,
            updated_at: chrono::Utc::now(),
            enrollment_embedding: None,
            encrypted_enrollment_embedding: None,
            evidence: Vec::new(),
        }
    }

    fn good_fixture() -> (SessionSpeaker, Vec<SpeakerChunk>) {
        let chunks = vec![
            chunk("chunk-0001", 0, 4_000, vec![1.0, 0.05], "speaker-1"),
            chunk("chunk-0002", 4_000, 8_000, vec![0.99, 0.0], "speaker-1"),
        ];
        let speaker = speaker(&["chunk-0001", "chunk-0002"], vec![1.0, 0.02]);
        (speaker, chunks)
    }

    #[test]
    fn clean_tight_discriminable_group_is_eligible() {
        let (speaker, chunks) = good_fixture();
        let alice = profile("Alice", vec![1.0, 0.0]);
        let bob = profile("Bob", vec![0.0, 1.0]);
        let report =
            evaluate_speaker_evidence(&speaker, &chunks, Some(&alice), &[alice.clone(), bob]);
        assert!(report.eligible, "unexpected failures: {:?}", report.reasons);
        assert!(report.reasons.is_empty());
        assert!(report.clean_duration_s >= 6.0);
    }

    #[test]
    fn short_clean_speech_fails_audio_gate() {
        let (mut speaker, mut chunks) = good_fixture();
        chunks[0].end_ms = 2_000;
        chunks[0].audio_duration_s = 2.0;
        chunks[1].start_ms = 2_000;
        chunks[1].end_ms = 4_500;
        chunks[1].audio_duration_s = 2.5;
        speaker.duration_ms = 4_500;
        let report = evaluate_speaker_evidence(&speaker, &chunks, None, &[]);
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("clean speech")));
    }

    #[test]
    fn low_purity_fails_audio_gate() {
        let (speaker, mut chunks) = good_fixture();
        chunks[0].vad_purity = 0.6;
        chunks[1].vad_purity = 0.65;
        let report = evaluate_speaker_evidence(&speaker, &chunks, None, &[]);
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("purity")));
    }

    #[test]
    fn clipped_chunk_in_group_fails_audio_gate() {
        let (speaker, mut chunks) = good_fixture();
        // A clipped chunk in the same cluster, even if not in the clean set.
        let mut clipped = chunk("chunk-0003", 8_000, 10_500, vec![1.0, 0.1], "speaker-1");
        clipped.clipping = true;
        chunks.push(clipped);
        let report = evaluate_speaker_evidence(&speaker, &chunks, None, &[]);
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("clipped")));
    }

    #[test]
    fn loose_cluster_fails_cluster_gate() {
        let (speaker, mut chunks) = good_fixture();
        chunks[0].session_score = Some(0.95);
        chunks[1].session_score = Some(0.70);
        let report = evaluate_speaker_evidence(&speaker, &chunks, None, &[]);
        assert!(!report.eligible);
        assert!(
            report.reasons.iter().any(|r| r.contains("agree"))
                || report.reasons.iter().any(|r| r.contains("tight")),
            "got {:?}",
            report.reasons
        );
    }

    #[test]
    fn low_margin_fails_discriminability_gate() {
        let (speaker, mut chunks) = good_fixture();
        chunks[0].margin = Some(0.02);
        chunks[1].margin = Some(0.05);
        let report = evaluate_speaker_evidence(&speaker, &chunks, None, &[]);
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("margin")));
    }

    #[test]
    fn near_identical_profiles_fail_profile_margin_gate() {
        let (speaker, chunks) = good_fixture();
        let alice = profile("Alice", vec![1.0, 0.0]);
        let impostor = profile("Bob", vec![1.0, 0.01]);
        let report =
            evaluate_speaker_evidence(&speaker, &chunks, Some(&alice), &[alice.clone(), impostor]);
        assert!(!report.eligible);
        assert!(report.reasons.iter().any(|r| r.contains("profile")));
    }

    #[test]
    fn user_corrected_chunks_still_count_as_evidence() {
        // Correction history must not disqualify a chunk — corrected labels
        // are the most trustworthy evidence we have.
        let (speaker, mut chunks) = good_fixture();
        chunks[0].corrections.push(LabelCorrection {
            from_label: "Speaker B".into(),
            to_label: "Alice".into(),
            corrected_at_ms: 1,
            auto: false,
        });
        let report = evaluate_speaker_evidence(&speaker, &chunks, None, &[]);
        assert!(report.eligible, "got {:?}", report.reasons);
    }

    fn evidence(note: &str, emb: Vec<f32>) -> ProfileEvidence {
        ProfileEvidence {
            note_id: note.into(),
            session_speaker_id: "speaker-1".into(),
            centroid_embedding: norm(emb),
            encrypted_centroid_embedding: None,
            duration_ms: 10_000,
            mean_score: 0.9,
            std_dev: 0.03,
            mean_margin: 0.3,
            accepted_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn apply_evidence_rebuilds_embedding_from_enrollment_plus_evidence() {
        let mut alice = profile("Alice", vec![1.0, 0.0]);
        alice.sample_count = 2;

        apply_evidence(&mut alice, evidence("note-1", vec![0.0, 1.0])).expect("applies");

        // Enrollment average preserved before mixing.
        assert_eq!(alice.enrollment_embedding, Some(vec![1.0, 0.0]));
        // Rebuilt: 2 enrollment samples at [1,0] + 1 evidence at [0,1].
        let expected = norm(vec![2.0, 1.0]);
        for (got, want) in alice.embedding.iter().zip(expected.iter()) {
            assert!((got - want).abs() < 1e-6, "{:?}", alice.embedding);
        }
        assert_eq!(alice.evidence.len(), 1);
    }

    #[test]
    fn reapplying_same_session_evidence_replaces_not_duplicates() {
        let mut alice = profile("Alice", vec![1.0, 0.0]);
        apply_evidence(&mut alice, evidence("note-1", vec![0.0, 1.0])).expect("applies");
        let first = alice.embedding.clone();
        apply_evidence(&mut alice, evidence("note-1", vec![0.0, 1.0])).expect("re-applies");
        assert_eq!(alice.evidence.len(), 1, "same note+speaker must replace");
        assert_eq!(alice.embedding, first);
    }

    #[test]
    fn apply_evidence_rejects_dimension_mismatch() {
        let mut alice = profile("Alice", vec![1.0, 0.0]);
        assert!(apply_evidence(&mut alice, evidence("note-1", vec![0.0, 1.0, 0.0])).is_err());
        assert!(alice.evidence.is_empty());
    }
}
