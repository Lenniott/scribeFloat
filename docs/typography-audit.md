# Typography consistency audit

> Generated for the typography consistency pass. Scope: `src/**/*.svelte` + role classes in `src/app.css`.
>
> **Agent guidance:** `.cursor/skills/ui-enforcement/SKILL.md` → `references/typography.md`
>
> **Target role classes** (defined in `app.css`; component migration not done yet):
> `sf-display-lg`, `sf-headline-sm`, `sf-section-label`, `sf-body-md`, `sf-body-md-strong`,
> `sf-label-md`, `sf-label-sm`, `sf-field-label`, `sf-meta-sm`

## Executive summary

| Metric | Count |
|--------|------:|
| Svelte files total | 64 |
| Files with typography tokens | 47 |
| Files using `sf-*` role classes | 11 (17%) |
| Files with inline-only typography | 39 (61%) |
| `sf-*` class usages | 18 |
| Inline size-token usages (`text-label-*`, `text-body-*`, etc.) | ~300+ |

**Adoption is very low.** The five role classes exist, but almost all UI still composes typography inline. Worse, several screens use **off-scale** Tailwind defaults (`text-base`, `text-sm`) or **undefined tokens** (`text-headline-lg`, `text-body-lg`, `tracking-heading`) that are not registered in `@theme inline` in `app.css`.

**Realistic target for the pass:** consolidate to **~8–10 semantic roles** (not 126 unique inline combos). The current five roles are a good skeleton; they need extensions for field labels, metadata/timestamps, and muted body — plus retirement of off-scale tokens.

---

## Canonical role classes

Defined in `src/app.css` (see `references/typography.md` in ui-enforcement skill):

| Class | Intended role |
|-------|---------------|
| `.sf-display-lg` | Hero / welcome display |
| `.sf-headline-sm` | Page & panel titles |
| `.sf-section-label` | In-panel section headers |
| `.sf-body-md` | Default reading text |
| `.sf-body-md-strong` | Emphasized body |
| `.sf-label-md` | Tabs, buttons, compact labels |
| `.sf-label-sm` | Stamped small labels, column headers |
| `.sf-field-label` | Form field labels (`text-fg-dim` baked in) |
| `.sf-meta-sm` | Timestamps, durations (`tabular-nums`) |

### Registered type scale (`@theme inline`)

| Token | Size | Line height |
|-------|------|-------------|
| `text-display-lg` | 1.26rem | 1.1 |
| `text-headline-sm` | 1.05rem | 1.4 |
| `text-body-md` | 0.875rem | 1.4 |
| `text-label-md` | 0.875rem | 1.4 |
| `text-label-sm` | 0.729rem | 1.4 |

Tracking tokens: `tracking-normal`, `tracking-tight`, `tracking-stamped`.

---

## `sf-*` adoption map

### Usage counts

| Role class | Usages | Files |
|------------|-------:|-------|
| `sf-headline-sm` | 11 | Page titles (settings sub-pages, transcribe, scribe-processing, Modal, StepShell) |
| `sf-label-sm` | 5 | Chip, setting_models table headers |
| `sf-body-md` | 2 | setting_models (error banner, VAD row) |
| `sf-label-md` | 1 | setting_models (selected model name) |
| **`sf-display-lg`** | **0** | **Defined but never used** |

### Files already using `sf-*` (partial)

| File | Roles used | Notes |
|------|------------|-------|
| `setting_models.svelte` | All except display | Best adopter; still mixes inline for progress/active states |
| `settings.svelte` | headline only | Nav descriptions are inline `text-label-sm` |
| `setting_general.svelte` | headline only | All field labels inline |
| `setting_help.svelte` | headline only | **Entire body uses `text-base` / `text-sm`** — off scale |
| `setting_permissions.svelte` | headline only | Permission rows inline |
| `setting_webhook.svelte` | headline + inline body | |
| `scribe-processing.svelte` | headline only | Preview body inline |
| `transcribe.svelte` | headline only | Queue UI inline |
| `Modal.svelte` | headline only | |
| `StepShell.svelte` | headline only | |
| `Chip.svelte` | label-sm only | Color variants added separately |

---

## Off-scale & undefined tokens (fix first)

These bypass the design system entirely and should be eliminated or added to `@theme` deliberately.

| Token | Occurrences | Where | Problem |
|-------|------------:|-------|---------|
| `text-base` | 31 | `setting_help.svelte` (dominant) | 1rem — larger than `text-body-md` (0.875rem) |
| `text-sm` | 36 | `setting_help.svelte` tables, `NavButton.svelte`, `setting_models.svelte` | Default Tailwind scale, not DS tokens |
| `text-headline-lg` | 13 | `routes/design-system/+page.svelte` only | **Not in `@theme`** |
| `text-body-lg` | 1 | `WelcomeStep.svelte` | **Not in `@theme** |
| `tracking-heading` | 13 | `design-system/+page.svelte` | **Not in `@theme`** |
| `text-display-lg` (inline) | 3 | WelcomeStep, design-system | Differs from `.sf-display-lg` (uses `uppercase` + `tracking-stamped`, not role class) |

---

## Inline pattern catalogue

Grouped by how often the same recipe appears. Color/opacity suffixes (`text-fg/45`, `text-fg-dim`, `text-destructive`) are listed separately — roles should own **size + weight + tracking + case**; color stays contextual.

### Tier 1 — high frequency (merge into roles)

| Pattern (size / weight / tracking / case) | ~Count | Maps to | Suggested role |
|-------------------------------------------|-------:|---------|----------------|
| `text-label-sm` (+ color only) | 144 | Partial `sf-label-sm` | Extend `sf-label-sm` or add `sf-label-sm-muted` |
| `text-body-md` (+ `text-fg`) | 103 | `sf-body-md` but **missing `font-light`** in inline | Align inline to `sf-body-md` |
| `text-label-sm tracking-stamped uppercase` | 52 | `sf-label-sm` uses `capitalize` not `uppercase` | **Case mismatch** — pick one |
| `text-label-sm font-normal tracking-stamped uppercase` | 45 | Field label recipe | New: `sf-field-label` |
| `text-label-md` (+ color) | 36 | Partial `sf-label-md` | Tab labels, empty states |
| `text-label-md tracking-stamped uppercase` | 8 | Section headers in history/detail | New: `sf-section-label`? |
| `text-label-sm font-normal tracking-stamped tabular-nums` | 6 | Timestamps | New: `sf-meta-sm` |
| `text-body-md font-medium` | 5 | Emphasis within body | `sf-body-md-emphasis` or utility modifier |

### Tier 2 — contextual (keep as color overrides on roles)

| Pattern | ~Count | Typical use |
|---------|-------:|-------------|
| `text-label-sm` + `text-fg/45` | 42 | Empty states, loading |
| `text-label-sm` + `text-destructive` | 16 | Inline errors |
| `text-label-sm` + `text-fg/55` + stamped + uppercase | 12 | Table column headers (transcribe queue) |
| `text-body-md` + `text-fg-dim` | 21 | Secondary descriptions |
| `text-body-md` + `text-fg/80` or `/90` | 14 | Transcript preview, summaries |
| `text-label-md` + `text-fg/45` | 8 | “Loading…”, “No content” |

### Tier 3 — component-embedded (fix in component, propagates everywhere)

| Component | Current typography | Should become |
|-----------|-------------------|---------------|
| `Button.svelte` | `text-label-md` / `text-label-sm` + `tracking-stamped` on primary | `sf-label-md` / `sf-label-sm` or dedicated `sf-btn-text` |
| `NavButton.svelte` | `text-sm` (off scale) | `text-label-md` or new nav role |
| `ConfigField.svelte` | `labelClass = 'text-label-sm font-normal tracking-stamped text-fg/80 uppercase'` | `sf-field-label` |
| `LabeledTextField.svelte` | Same field label recipe | `sf-field-label` |
| `PathSelectorField.svelte` | Same | `sf-field-label` |
| `HotkeyCaptureField.svelte` | Same | `sf-field-label` |
| `OptionGroup.svelte` | Legend: same recipe | `sf-field-label` |
| `AccordionItem.svelte` | `text-label-md … uppercase` | `sf-section-label` |
| `TabPage.svelte` | `text-label-md` / `text-label-sm` + stamped + uppercase | role + state colors |
| `TimestampLabel.svelte` | `text-label-sm tabular-nums tracking-stamped` | `sf-meta-sm` |
| `RecordingTimer.svelte` | Same family | `sf-meta-sm` |
| `Toast.svelte` | `text-body-md` | `sf-body-md` |

---

## File-by-file: not using `sf-*` roles

### Screens (all inline except headline where noted)

| File | Inline highlights | Priority |
|------|-------------------|----------|
| `scribe.svelte` | Field labels, inputs, timer area, errors — all inline | High (primary workflow) |
| `history.svelte` | Tab bar, empty states | High |
| `dictate.svelte` | HUD result text `text-label-md font-medium` | Medium |
| `setting_replace.svelte` | Field labels + table text | Medium |
| `setting_general.svelte` | Headline only sf-*; 7× field label recipe | High |
| `setting_help.svelte` | **Off-scale `text-base`/`text-sm` throughout** | High |
| `onboarding.svelte` | Error banner only | Low |
| `loading-screen.svelte` | No typography classes | — |

### Feature components

| File | Inline highlights | Priority |
|------|-------------------|----------|
| `history/HistoryListCard.svelte` | Timestamp, title, chips | High |
| `history/HistoryDetailPane.svelte` | Title, metadata, transcript body | High |
| `transcribe/TranscribeQueueList.svelte` | Column headers stamped uppercase | Medium |
| `transcribe/TranscribeQueueRow.svelte` | File name body + metadata | Medium |
| `transcribe/TranscribeProcessingSummary.svelte` | Body + errors | Medium |
| `notes/NoteCard.svelte` | Timestamp + body | Medium |
| `notes/NotesPanel.svelte` | Panel label | Low |
| `notes/NotesList.svelte` | Empty state | Low |
| `notes/NoteComposer.svelte` | Composer `text-body-md` | Low |
| `onboarding/WelcomeStep.svelte` | **Custom display** + undefined `text-body-lg` | High |
| `onboarding/PermissionsStep.svelte` | Permission names + hints | Medium |
| `onboarding/FeatureTourStep.svelte` | Tour labels `text-body-md font-medium` | Medium |
| `onboarding/DictatePracticeStep.svelte` | Instructions + errors | Medium |
| `onboarding/ModelDownloadStep.svelte` | Model names emphasized | Medium |
| `form/*` | Shared field label recipe (5 files) | High (single fix) |
| `audio/AudioLayerLegend.svelte` | `text-label-sm tracking-stamped` | Low |
| `accordion/SettingsSection.svelte` | Section title inline | Medium |

### Layout / chrome

| File | Notes |
|------|-------|
| `NavButton.svelte` | Uses `text-sm` — only nav item off scale |
| `IconButton.svelte` | No text (icon only) |
| `PanelHeader.svelte` | No typography tokens in markup |
| `PanelFooter.svelte` | No typography tokens |
| `FixedFooterBar.svelte` | No typography tokens |
| `PanelShell.svelte`, `SplitPane.svelte` | Structural only |

### Routes

| File | Notes |
|------|-------|
| `routes/design-system/+page.svelte` | Showcase page; uses undefined `text-headline-lg`, `tracking-heading` — treat as spec debt, not product UI |
| `routes/+page.svelte`, `+layout.svelte` | No typography |

---

## Files with no typography classes (17)

These rely on inheritance or are non-text UI. No action needed unless text is added later.

`Accordion.svelte`, `ScrollablePanel.svelte`, `AudioWaveFormVisualizer.svelte`, `RecordingStatusDot.svelte`, `DeviceSelect.svelte`, `ToggleSwitch.svelte`, `FixedFooterBar.svelte`, `PanelFooter.svelte`, `PanelHeader.svelte`, `PanelShell.svelte`, `SplitPane.svelte`, `StepProgress.svelte`, `IconButton.svelte`, `loading-screen.svelte`, `+layout.svelte`, `+page.svelte`

(`NavButton.svelte` has typography in a JS constant — easy to miss in markup-only scans.)

---

## Known inconsistencies in the role classes themselves

1. **`capitalize` vs `uppercase`** — Roles use `capitalize`; the most common field-label recipe uses `uppercase`. Product UI looks stamped/all-caps, not title case.

2. **`sf-body-md` sets `font-light`** — Inline `text-body-md` almost never adds `font-light`; visual drift between adopters and non-adopters.

3. **`sf-label-md` bakes in `text-fg-dim`** — Many `text-label-md` usages need full `text-fg` (tabs, titles, loading states). Role is too opinionated for reuse.

4. **`sf-display-lg` unused** — Welcome hero uses manual `text-display-lg tracking-stamped uppercase` instead.

5. **Color in roles vs color inline** — Roles partially own color (`sf-label-md` → dim). Most call sites need contextual color (destructive, muted, active). **Recommendation:** roles own metrics only; color stays at call site via `text-fg`, `text-fg-dim`, `text-destructive`, etc.

---

## Recommended role set (realistic)

Keep the five names where possible; add 3–4 for gaps. Target **8–9 total**.

| Role | Size token | Weight | Tracking | Case | Color |
|------|------------|--------|----------|------|-------|
| `sf-display-lg` | display-lg | normal | stamped | uppercase | caller |
| `sf-headline-sm` | headline-sm | normal | stamped | uppercase | caller |
| `sf-body-md` | body-md | light | normal | normal | caller |
| `sf-body-md-strong` | body-md | medium | normal | normal | caller *(new)* |
| `sf-label-md` | label-md | normal | stamped | uppercase | caller *(drop baked-in dim)* |
| `sf-label-sm` | label-sm | normal | stamped | uppercase | caller |
| `sf-field-label` | label-sm | normal | stamped | uppercase | `text-fg/80` *(new — replaces 5+ form copies)* |
| `sf-meta-sm` | label-sm | normal | stamped | normal | caller + `tabular-nums` *(new — timestamps, durations)* |

**Do not add** separate roles for every opacity step (`text-fg/45`, `/55`, `/80`). Use 2–3 semantic color tokens (`text-fg`, `text-fg-dim`, `text-fg-muted`) at the call site.

**Retire:** `text-base`, `text-sm` in product UI → map to `text-body-md` or `text-label-sm`.

**Decide:** whether `text-headline-lg` / `text-body-lg` belong in the scale or get deleted with the design-system page updated.

---

## Suggested pass order

1. **Normalize `setting_help.svelte`** — swap `text-base`/`text-sm` → `sf-body-md` / `sf-label-sm` (biggest off-scale offender).
2. **Extract `sf-field-label`** — update `ConfigField`, `LabeledTextField`, `PathSelectorField`, `HotkeyCaptureField`, `OptionGroup`, `setting_general`, `scribe`, `transcribe` field labels (~45 call sites, mostly copy-paste).
3. **Fix role definitions** — resolve capitalize vs uppercase; remove `text-fg-dim` from `sf-label-md`; ensure `sf-body-md` matches majority inline look.
4. **Component primitives** — `Button`, `NavButton`, `TabPage`, `TimestampLabel`, `RecordingTimer`, `Toast`.
5. **Feature screens** — history, scribe-processing, transcribe queue, onboarding welcome.
6. **Design system page** — align with final tokens or mark as experimental.

---

## Quick reference: headline/title elements still inline

These use `sf-headline-sm` today (good): settings sub-pages, transcribe title, scribe-processing title, Modal, StepShell.

These use **inline** headline tokens (should migrate):

| Element | Current classes | File |
|---------|-----------------|------|
| Welcome hero | `text-display-lg tracking-stamped uppercase` | `WelcomeStep.svelte` |
| History list header | `text-label-md tracking-stamped uppercase` | `history.svelte` |
| Scribe notes header | `text-label-md tracking-stamped uppercase` | `scribe.svelte` |
| Transcribe section labels | `text-label-sm uppercase tracking-stamped` | `transcribe.svelte` |

---

## Appendix: raw size-token frequency (inline)

| Token | Count |
|-------|------:|
| `text-label-sm` | 138 |
| `text-body-md` | 103 |
| `text-label-md` | 38 |
| `text-sm` | 36 |
| `text-base` | 31 |
| `text-headline-lg` | 13 |
| `text-display-lg` | 3 |
| `text-headline-sm` | 2 |
| `text-body-lg` | 1 |

*Counts from static scan of quoted class strings in `src/**/*.svelte`; includes JS constants (e.g. `Button.svelte`, `ConfigField.svelte`).*
