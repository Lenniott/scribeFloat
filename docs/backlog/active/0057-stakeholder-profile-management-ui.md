---
id: "0057"
title: Named stakeholder profile management UI
status: active
adr: ADR-0011
---

# Named stakeholder profile management UI

A settings screen that lists all enrolled voiceprint profiles, shows their metadata (mic device, sample count, last updated), and lets the user add, rename, or delete them. This is the central management surface for the voiceprint feature.

Depends on: 0052 (VoiceprintService IPC), 0054 (onboarding flow — "Add" button triggers it).

---

## Backend

No new backend work beyond what 0052 provides. This story is purely frontend using the IPC commands from 0052:
- `voiceprint_list_profiles` → `Vec<ProfileSummary>`
- `voiceprint_rename_profile(slug, name)` → `()`
- `voiceprint_delete_profile(slug)` → `()`

`ProfileSummary` (returned by `voiceprint_list_profiles`):

```rust
pub struct ProfileSummary {
    pub slug: String,
    pub name: String,
    pub mic_device_id: Option<String>,
    pub mic_device_label: Option<String>,  // human-readable, resolved at list time
    pub sample_count: u32,
    pub updated_at: String,  // ISO-8601
    pub is_user: bool,       // true if this is the "You" profile
}
```

---

## Frontend

### Settings → Voice panel

Add a "Voice" tab (or section) to the settings panel. Structure:

```
Settings › Voice

  Speaker labels
  ──────────────────────────────────────────────
  When a voiceprint is enrolled, transcripts from Record sessions
  are automatically labelled by speaker.

  Enrolled profiles
  ┌──────────────────────────────────────────────────────┐
  │  ● You (Built-in microphone)                         │
  │    12 clips · Last updated 2026-06-20    [Rename] [✕]│
  ├──────────────────────────────────────────────────────┤
  │  ● You (USB Audio Device)                            │
  │    5 clips · Last updated 2026-06-22     [Rename] [✕]│
  ├──────────────────────────────────────────────────────┤
  │  ● Alice                                             │
  │    1 clip · Last updated 2026-06-23      [Rename] [✕]│
  └──────────────────────────────────────────────────────┘

  [+ Enroll my voice]   [Clear all voiceprints]
```

### Rename inline flow

Tapping [Rename] on a profile row:
- Row transitions to an inline edit: text input pre-filled with current name.
- [Save] / [Cancel] buttons.
- On save: call `voiceprint_rename_profile(slug, newName)`.
- Optimistic update: update the list immediately; revert on error.

### Delete confirmation

Tapping [✕] on a profile row:
- Inline confirmation: "Delete Alice? Speaker labels using this profile will show as [Other]."
- [Delete] (destructive) / [Cancel].
- On confirm: call `voiceprint_delete_profile(slug)`, remove row from list.

### "Clear all voiceprints" action

- Confirmation dialog: "Delete all voiceprints? Speaker labels will stop working until you re-enroll."
- [Delete all] / [Cancel].
- On confirm: call `voiceprint_delete_profile` for each profile.

### Empty state

When no profiles are enrolled:

```
  No voiceprints enrolled yet.

  Speaker labels appear automatically after a few Dictate sessions,
  or you can enroll your voice manually.

  [Enroll my voice]
```

### "Enroll my voice" button

Opens the onboarding flow from story 0054.

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- Settings → Voice panel shows all enrolled profiles with mic label, sample count, and last-updated date
- Rename flow updates the profile name and re-renders the list
- Delete with confirmation removes the profile from disk and list
- "Clear all" removes all profiles with a single confirmation
- "Enroll my voice" launches the onboarding flow (0054)
- Empty state renders when no profiles exist
- Deleting the "You" profile does not crash; speaker labels fall back to unlabelled transcript
