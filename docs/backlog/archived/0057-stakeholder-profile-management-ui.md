---
id: "0057"
title: Voiceprint profile manager in settings
status: done
adr: ADR-0011
---

# Voiceprint profile manager in settings

A Settings → Voice panel that lists all enrolled voiceprint profiles, shows how many prints each profile has, and lets the user add more prints to any profile, rename profiles, or delete them. Adding more prints to an existing profile is the main way to improve accuracy after initial onboarding.

Depends on: 0052 (VoiceprintService IPC), 0054 (enrollment flow — "Add print" reuses it).

---

## Backend

No new backend beyond 0052 and 0054. Reuses:
- `voiceprint_list_profiles` → `Vec<ProfileSummary>`
- `voiceprint_rename_profile(slug, name)` → `()`
- `voiceprint_delete_profile(slug)` → `()`
- `voiceprint_start_clip`, `voiceprint_stop_clip`, `voiceprint_commit_clip` from 0054

`ProfileSummary`:

```rust
pub struct ProfileSummary {
    pub slug: String,
    pub name: String,
    pub mic_device_id: Option<String>,
    pub mic_device_label: Option<String>,
    pub sample_count: u32,      // number of clips enrolled
    pub updated_at: String,     // ISO-8601
}
```

---

## Frontend

### Settings → Voice panel

```
Settings › Voice

  Voiceprints
  ──────────────────────────────────────────────────────────
  Each voiceprint is built from one or more clips. More clips
  across different distances and mics makes identification
  more accurate.

  ┌──────────────────────────────────────────────────────────┐
  │  You                                                     │
  │  Built-in microphone · 4 clips · Updated 2026-06-22      │
  │  [+ Add print]  [Rename]  [Delete]                       │
  ├──────────────────────────────────────────────────────────┤
  │  Alice                                                   │
  │  Built-in microphone · 1 clip · Updated 2026-06-23       │
  │  [+ Add print]  [Rename]  [Delete]                       │
  └──────────────────────────────────────────────────────────┘

  [+ Enroll a voice]
```

### "+ Add print" action

Tapping "+ Add print" on any profile row launches the enrollment flow from story 0054, with the profile name pre-filled in the name step and the dropdown disabled (you are adding to this specific profile). The flow runs inline or as a sheet — same component as onboarding, just pre-seeded.

On completion, the profile row updates its clip count.

### "Rename" inline flow

Row expands to an inline text input:

```
  ┌──────────────────────────────────────────────────────────┐
  │  [ Alice (CEO)                                        ]  │
  │                                   [Cancel]  [Save]       │
  └──────────────────────────────────────────────────────────┘
```

On save: `voiceprint_rename_profile(slug, newName)`. Optimistic update; revert on error.

### "Delete" confirmation

Inline confirmation below the row:

```
  Delete Alice? Segments labelled [Alice] will show as [Other].
  [Cancel]  [Delete]
```

On confirm: `voiceprint_delete_profile(slug)`, row removed.

### "Enroll a voice" button

Opens the enrollment flow from story 0054 with no pre-filled name (standard flow). After completion, a new profile row appears.

### Empty state

```
  No voiceprints yet.

  Add voiceprints to label your transcripts by speaker —
  start with yourself, then add others as you meet them.

  [+ Enroll a voice]
```

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- Settings → Voice lists all enrolled profiles with mic label, clip count, and last-updated date
- "+ Add print" launches the enrollment flow with the profile name pre-filled and locked
- After adding a print, the clip count on that row increments
- Rename updates the profile name on disk and in the list
- Delete with inline confirmation removes the profile and its row
- "Enroll a voice" launches a fresh enrollment flow; new profile appears in the list on completion
- Empty state renders when no profiles exist
