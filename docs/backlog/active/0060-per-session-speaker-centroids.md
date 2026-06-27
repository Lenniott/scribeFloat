---
id: "0060"
title: Compute and store per-session speaker centroids after labeling
status: active
---

# Compute and store per-session speaker centroids after labeling

After `label_segments()` runs and `SpeakerBlock` embeddings are populated (story 0059), group segments by their label and compute a centroid embedding per speaker for that session. Store the centroids alongside the session transcript.

For each label that has at least one embedded segment:

- Average the member embeddings element-wise and L2-normalise the result → `centroid: Vec<f32>`
- Compute the mean cosine similarity of members to the centroid → `mean_score: f32`
- Compute the standard deviation of those cosine similarities → `std_dev: f32`
- Record `segment_count: usize`

Store these as a `speaker_centroids` map in the session JSON:

```json
{
  "speaker_centroids": {
    "You":   { "embedding": [...], "segment_count": 12, "mean_score": 0.89, "std_dev": 0.04 },
    "Alice": { "embedding": [...], "segment_count": 8,  "mean_score": 0.82, "std_dev": 0.07 }
  }
}
```

This centroid is the within-session reference that all subsequent scoring and correction flows use instead of the global profile embedding.

## Notes

- Depends on story 0059 (embeddings stored on `SpeakerBlock`)
- `Other` segments should still be centroid-aggregated — their centroid is useful for detecting when an unknown speaker is actually consistent across the session
- Recompute centroids whenever a label correction is applied (story 0062)
- The `std_dev` is the primary cluster-quality signal: low (~0.03) = reliable label, high (~0.15) = mixed or noisy
