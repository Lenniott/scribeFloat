---
id: "0063"
title: Quality-gated global voiceprint profile update from session centroid
status: active
---

# Quality-gated global voiceprint profile update from session centroid

As a user who records meetings regularly, I want each good recording to quietly improve how well the app recognises my voice and other speakers over time so that I don't have to manually re-enrol or go to Settings to keep accuracy high.

After a session ends (or after inline corrections settle), offer to improve a speaker's global voiceprint profile using the session centroid — but only when all three quality gates pass:

**Audio quality** (from stored signals on `SpeakerBlock`, story 0059):
- Mean segment duration across the label's members >= 6 s
- Mean VAD purity >= 0.80
- No clipped segments in the label's group

**Cluster quality** (from session centroid, story 0060):
- `mean_score >= 0.85` — members agree with each other
- `std_dev <= 0.06` — tight cluster, not a mixed group

**Discriminability** (from margin scores, story 0061):
- Mean `margin` across the label's members >= 0.15 — clearly separated from other speakers

If all gates pass, blend the session centroid into the global profile using the existing `update_profile_embedding()` rolling-average logic, weighted by `segment_count`.

If any gate fails, do not update and surface the reason to the user ("cluster too noisy — std_dev 0.14", "audio too short", etc.) so they understand why automatic improvement didn't happen.

## Notes

- Depends on stories 0059, 0060, 0061
- The update should be opt-in (prompt the user) unless they have previously opted into automatic improvement
- The `Other` label should never update a global profile — only named enrolled speakers
- Store a `last_session_update` timestamp on the profile so the user can see when it was last improved from a session
