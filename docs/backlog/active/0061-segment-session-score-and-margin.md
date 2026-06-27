---
id: "0061"
title: Score each segment against session centroid and compute speaker margin
status: active
---

# Score each segment against session centroid and compute speaker margin

As a user reviewing a transcript, I want to see which speaker labels the app is confident about and which ones it was unsure of so that I know where to focus my corrections rather than reading every line.

Once session centroids exist (story 0060), back-fill two confidence signals onto each `SpeakerBlock`:

**`session_score: Option<f32>`** — cosine similarity of the segment's embedding to its own label's session centroid. Measures how well the segment fits its assigned speaker cluster.

- `>= 0.90` → high confidence, matches the cluster well
- `0.75–0.90` → consistent, probably correct
- `< 0.75` → outlier — likely mislabeled or noisy audio

**`margin: Option<f32>`** — difference between the best-matching centroid score and the second-best. Measures how clearly the segment belongs to one speaker vs another.

- `>= 0.15` → unambiguous
- `0.05–0.15` → uncertain, worth surfacing to the user
- `< 0.05` → genuinely ambiguous — do not auto-label

These two signals together gate the inline correction UI (story 0062) and the global profile update (story 0063). Segments with low `session_score` or low `margin` are candidates for the user to review.

## Notes

- Depends on stories 0059 and 0060
- Computation is pure cosine math against the in-memory centroid map — no re-embedding needed
- Segments with `embedding: None` (too short) get `session_score: None, margin: None`
- The UI should render a visual confidence indicator (e.g. faint / normal / bold text or a small badge) based on these values — exact design is out of scope here
