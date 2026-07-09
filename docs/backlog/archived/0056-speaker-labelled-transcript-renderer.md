---
id: "0056"
title: Speaker-labelled transcript format and renderer
status: done
adr: ADR-0011
---

# Speaker-labelled transcript format and renderer

Applies voiceprint identification to Whisper segments after transcription and renders the result as grouped speaker blocks — `[You]` / `[Other]` / `[Name]` with optional timestamp ranges. Only activates when at least one voiceprint profile is enrolled; falls back to the existing plain transcript when no profiles exist.

Depends on: 0052 (VoiceprintService), 0058 (config for `user_display_name`).

---

## Backend

### 1. Speaker labelling pass

After `transcribe_pcm_with_progress()` returns a `Vec<Segment>` in `run_batch`, run the labelling pass when `VoiceprintService` has at least one profile:

```rust
pub fn label_segments(
    segments: &[Segment],
    session_pcm: &[f32],
    sample_rate: u32,
    voiceprint_svc: &VoiceprintService,
    user_label: &str,
) -> Vec<SpeakerBlock>
```

Steps per segment:
1. Slice `session_pcm[start_sample..end_sample]` using `segment.start_ms` / `segment.end_ms`.
2. If the slice is < 2 s of speech: label as `"Other"` (too short to embed reliably; conservative default).
3. Otherwise: `embed(slice)` → `identify(embedding, profiles)` → label.
4. Push `SpeakerBlock { label, start_ms, end_ms, text: segment.text.clone() }`.

### 2. Merge consecutive blocks

```rust
pub fn merge_blocks(blocks: Vec<SpeakerBlock>) -> Vec<SpeakerBlock>
```

Merge consecutive blocks with the same `label` by extending `end_ms` and appending text with a space. This collapses runs of short same-speaker segments into readable paragraphs.

### 3. `SpeakerBlock` struct

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpeakerBlock {
    pub label: String,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
    pub text: String,
}
```

### 4. Expose via IPC

The existing `transcribe_*` commands should return `Vec<SpeakerBlock>` when labelling is active, or fall back to a single unlabelled block wrapping the full transcript text when no profiles are enrolled. The frontend distinguishes the two cases by checking whether any block has a non-null `label` (or by a separate `labelled: bool` flag on the response).

### 5. Markdown export

When exporting a note to markdown and speaker blocks are present:

```markdown
**[You]** · 0:00–2:14

So the core question here is…

---

**[Other]** · 2:14–3:45

I think we should wait…
```

When no timestamp is available (Dictate mode):

```markdown
**[You]**

So the core question here is…

---

**[Other]**

I think we should wait…
```

---

## Frontend

### 1. Speaker block component

Create `src/lib/ui/2_blocks/SpeakerBlock.svelte`:

```svelte
<div class="speaker-block">
  <div class="speaker-header">
    <span class="speaker-chip">[{label}]</span>
    {#if start_ms != null}
      <span class="speaker-time">{formatTime(start_ms)} → {formatTime(end_ms)}</span>
    {/if}
  </div>
  <p class="speaker-text">{text}</p>
</div>
```

`formatTime(ms)` → `"02:14"` (mm:ss).

### 2. Transcript view

In the note detail / transcript view, when the note has `speaker_blocks`:

- Render `<SpeakerBlock>` for each block instead of the plain text paragraph.
- Separate adjacent blocks with a subtle horizontal rule.

When no `speaker_blocks` (no profiles enrolled): render existing plain transcript — no regression.

### 3. Transcript toolbar filters

Add two toggle buttons to the transcript toolbar:

- **"Hide Other"** — collapses all blocks where `label === "Other"` (or any non-user label).
- **"Me only"** — same effect, different framing. These are mutually equivalent; implement as a single toggle.

State: `hideOthers: boolean`. When true, non-user blocks are `display: none` (not removed from DOM — timestamps remain stable).

### 4. Speaker chip tap-to-rename

Tapping a `[Name]` chip opens a small inline popover:

```
┌──────────────────┐
│  Rename speaker  │
│  [ Alice      ]  │
│  [Cancel] [Save] │
└──────────────────┘
```

On save, call `voiceprint_rename_profile(slug, name)` IPC and re-render all blocks with the updated label (optimistic update; revert on error).

---

## Definition of done

- `cargo clippy -- -D warnings` passes
- `npm run check` passes
- When ≥ 1 voiceprint profile is enrolled, a transcribed Record session shows speaker blocks
- `[You]` blocks show the user's `user_display_name` (default "You")
- `[Other]` blocks show for segments below threshold
- Named profile blocks show the profile name
- Consecutive same-speaker segments are merged into single blocks
- Timestamp range is shown for Record (has timing) and omitted for Dictate (no timing)
- "Hide Other" toggle collapses non-user blocks
- Tapping a speaker chip opens rename popover; rename persists
- When no profiles are enrolled, plain transcript renders unchanged
- Markdown export includes speaker headers and separators
