# Color consistency audit

> Scope: `src/**/*.svelte`, `src/app.css`. Design source: `context/design-skill/design-system.json` → `rules.colors`.
>
> **Agent guidance:** `.cursor/skills/ui-enforcement/references/color.md`
>
> ## Decision (locked)
>
> **Option A — semantic foreground tokens only.** Retire `text-fg/{opacity}` in product UI. Use `text-fg`, `text-fg-dim`, `text-fg-muted` (no Option B `sf-text-*` role classes).
>
> ## Foundation status
>
> | Item | Status | Location |
> |------|--------|----------|
> | `--sf-overlay` + `bg-overlay` / `--color-overlay` | **Done** | `src/app.css` |
> | `.sf-scrim` shortcut | **Done** | `src/app.css` |
> | `sf-field-label` uses `text-fg-dim` (not `/80`) | **Done** | `src/app.css` |
> | `check:ds` warn on `text-fg/\d+`, `bg-black/\d+` | **Done** | `scripts/check-design-system.mjs` |
> | Hook deny on new opacity/scrim in `.svelte` | **Done** | `.cursor/hooks/ui-enforcement-lib.mjs` |
> | Cursor rule + color.md cheat sheet | **Done** | `.cursor/rules/`, skill references |
> | **Component migration** | **Not started** | ~173 opacity usages remain |
> | Elevate `check:ds` warn → error | **Deferred** | After migration pass |
>
> ## Migration pass order (components — do not start until typography pass underway)
>
> 1. **Modal + settings scrim** — `bg-black/50` → `sf-scrim` or `bg-overlay` (2 files)
> 2. **High-churn fg opacity files** — history detail, scribe-processing, setting_general, setting_replace, transcribe, scribe (see table below)
> 3. **Semantic misuse** — `setting_models` `text-active`, WelcomeStep decorative `text-brand`
> 4. **Primitive adoption** — `sf-input`, `sf-divider` in form components
> 5. **Border recipe cleanup** — replace ad-hoc `border-card/60`, `border-rim/30` with documented recipes
> 6. **Tighten lint** — `check:ds` opacity rules: warn → error; hook already denies new usage in `.svelte`

---

## Executive summary

| Metric | Finding |
|--------|---------|
| Files with color classes | 59 / 64 svelte |
| Hardcoded hex in components | **0** |
| Legacy Tailwind palette | **0** (except `bg-black/50` ×2) |
| `sf-*` surface shortcuts adoption | **0** in components (`.sf-scrim` ready) |
| Semantic `text-fg` | `text-fg` 146 · `text-fg-dim` 63 · `text-fg-muted` **8** |
| Ad-hoc `text-fg/{opacity}` | **10 steps**, ~173 uses |
| State text | `destructive` 29 · `success` 7 · `brand` 7 · `active` 6 |

**Good news:** Palette is almost entirely DS tokens — no hex, no `gray-*`.

**Hard problem:** Two parallel foreground systems — semantic tokens vs 10-step opacity ladder.

---

## Canonical tokens

### Surfaces

| Token | Class | Role |
|-------|-------|------|
| canvas | `bg-canvas` | Window / page only |
| panel | `bg-panel` | Main panel, modal body |
| card | `bg-card` | Cards, list rows |
| fill | `bg-fill` | Inputs, hover |
| rim | `border-rim` | Default borders |
| **overlay** | **`bg-overlay`** / **`.sf-scrim`** | Modal scrim (theme-aware) |

### Foreground (Option A mapping)

| Meaning | Token | Replaces |
|---------|-------|----------|
| Primary | `text-fg` | `text-fg`, `text-fg/90`, `text-fg/85` |
| Secondary (readable on any surface) | `text-fg-dim` | `text-fg/80`–`/50`, icons, chevrons, captions on card |
| Recessed copy only | `text-fg-muted` | helper/empty/disabled **prose** — not icons or card-row controls |

**Contrast trap:** `text-fg/50` on dark `bg-card` blended to ~L 0.60 (≈ `text-fg-dim`). Absolute `text-fg-muted` (L 0.40) is only ~0.12 above card — fails contrast. Do not map `/50` → muted for affordances.

### State colors

| Token | Job |
|-------|-----|
| brand / on-brand | Primary CTA fill only |
| active | Selected tab, toggle on (`bg-active/15`) |
| destructive | Delete, recording dot, inline errors |
| warning | Caution banners, links (`app.css` `a`) |
| success | Granted check, transient OK |
| focus | `ring-focus` only |

---

## Opacity ladder inventory

| Step | Count | Maps to (Option A) |
|------|------:|--------------------|
| `/80` | 69 | `text-fg-dim` |
| `/70` | 36 | `text-fg-dim` |
| `/55` | 27 | `text-fg-dim` |
| `/60` | 21 | `text-fg-dim` |
| `/50` | 20 | `text-fg-dim` (icons/controls); `text-fg-muted` only for recessed helper copy |
| `/45` | 22 | `text-fg-dim` or `text-fg-muted` — check surface contrast |
| `/90` | 10 | `text-fg` |
| `/40` | 8 | `text-fg-muted` (prose only) |
| `/75` | 8 | `text-fg-dim` |
| `/85` | 4 | `text-fg` |

---

## Documented surface recipes

| Recipe | Classes |
|--------|---------|
| Panel divider | `border-t border-card` or `sf-divider` |
| Tab bar | `border-b border-card/60 bg-panel/70` *(pick one — document before mass migration)* |
| Input | `sf-input` or `bg-panel border border-rim` |
| Selected tab | `border-b-2 border-active bg-active/15` |
| Warning banner | `border-b border-warning bg-warning/15 text-fg` |
| Error banner | `border border-destructive/40 bg-fill text-destructive` |
| Modal scrim | `fixed inset-0 sf-scrim` or `bg-overlay` |

---

## Priority migration files

| File | Opacity steps | Notes |
|------|---------------|-------|
| `history/HistoryDetailPane.svelte` | 40, 45, 50, 80, 90 | Highest complexity |
| `history.svelte` | 45, 60, 70, 80 | |
| `scribe-processing.svelte` | 45, 55, 80, 90 | |
| `setting_general.svelte` | 50, 60, 70, 80 | |
| `setting_replace.svelte` | 40, 50, 70, 80 | |
| `transcribe.svelte` | 55, 70, 75, 80 | |
| `scribe.svelte` | 40, 60, 80 | |
| `Modal.svelte` | — | `bg-black/50` → `sf-scrim` |
| `settings.svelte` | — | `bg-black/50` → `sf-scrim` |

---

## Semantic misuse (fix during migration)

| Issue | File | Fix |
|-------|------|-----|
| Decorative `text-brand` | WelcomeStep | `text-fg-dim` or `text-fg` |
| `text-active` as progress | setting_models | `text-fg` or `text-fg-dim` |
| `bg-black/50` scrim | Modal, settings | `sf-scrim` |

---

## Enforcement

| Layer | Behavior |
|-------|----------|
| `check:ds` | **warn** on `text-fg/\d+`, `bg-fg/\d+`, `bg-black/\d+` (exempt design-system page) |
| `preToolUse` hook | **deny** new opacity/scrim in `.svelte` writes |
| `src/app.css` | Exempt from hook (token source) |
| Post-migration | Promote `check:ds` to **error** |

*Counts from static scan; March 2026.*
