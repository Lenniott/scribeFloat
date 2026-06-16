# Typography

> Role classes live in `src/app.css` (`@layer components`). Migration inventory: `docs/typography-audit.md`.

## Principles

Query before guessing:

```bash
python3 context/design-skill/query.py ds get rules.typography
python3 context/design-skill/query.py ds get tokens.typography.scale
```

- Hierarchy via **size, case, tracking, and opacity** — not bold (`font-bold` / `font-semibold` banned by `check:ds`).
- `font-medium` is the strongest emphasis for body text (`sf-body-md-strong`).
- **Case (locked):** labels and headers → `capitalize` via `sf-*` roles. Body and meta → **sentence case** (no `uppercase`, no `capitalize`). Never `uppercase` in product UI.
- **`tracking-stamped`** on labels and headers only — not on body copy.
- **Geist** for UI text; **Geist Mono** only for instrument-style readouts.
- **Roles own metrics** (size, weight, tracking, case). **Color stays at call site** (`text-fg`, `text-fg-dim`, `text-fg-muted`, …).

## Role class table

Use these instead of composing `text-label-*` / `text-body-*` inline.

| Class | Use for | Color |
|-------|---------|-------|
| `sf-display-lg` | Hero / welcome titles | caller |
| `sf-headline-sm` | Page & panel titles (`h1`, `h2`, modal titles) | caller |
| `sf-section-label` | In-panel section headers (notes, history, queue groups) | caller |
| `sf-body-md` | Body copy, descriptions, input text | caller |
| `sf-body-md-strong` | Emphasized body (feature names, auto-enter toggle) | caller |
| `sf-label-md` | Tabs, buttons, chips, compact UI labels | caller |
| `sf-label-sm` | Stamped small labels, column headers, badges | caller |
| `sf-field-label` | Form field labels | baked `text-fg-dim` |
| `sf-meta-sm` | Timestamps, durations, numeric metadata | caller + `tabular-nums` |

### Examples

```svelte
<h1 class="sf-headline-sm text-fg">Transcribe</h1>
<p class="sf-body-md text-fg-dim">Drop audio files to queue.</p>
<label class="sf-field-label" for="save-folder">Save folder</label>
<span class="sf-meta-sm text-fg-dim">{elapsed}</span>
<p class="sf-body-md-strong text-fg">Auto enter</p>
```

## Banned in product UI

| Pattern | Use instead |
|---------|-------------|
| `uppercase` | Remove — `sf-headline-sm` / `sf-section-label` / `sf-label-md` / `sf-field-label` use `capitalize` |
| `text-base`, `text-sm` | `sf-body-md` or `sf-label-sm` |
| `text-headline-lg`, `text-body-lg` | Not in `@theme` — use `sf-headline-sm` / `sf-body-md` |
| `tracking-heading` | Not in `@theme` — use `tracking-stamped` |
| Inline `text-label-sm font-normal tracking-stamped uppercase` | `sf-field-label` or `sf-label-sm` |
| Inline `text-body-md text-fg` | `sf-body-md text-fg` |
| `font-bold`, `font-semibold` | `sf-body-md-strong` or size/opacity |

`routes/design-system/+page.svelte` is exempt until aligned — do not copy its patterns into product screens.

## Primitive components (migrate first)

Fix these once; screens inherit correct typography:

| Component | Target |
|-----------|--------|
| `ConfigField`, `LabeledTextField`, `PathSelectorField`, `HotkeyCaptureField`, `OptionGroup` | `sf-field-label` |
| `Button` | `sf-label-md` / `sf-label-sm` (or future `sf-btn-*` alignment) |
| `NavButton` | `sf-label-md` (currently off-scale `text-sm`) |
| `TabPage` | `sf-label-md` / `sf-label-sm` (capitalize, no uppercase) |
| `TimestampLabel`, `RecordingTimer` | `sf-meta-sm` |
| `Toast` | `sf-body-md` |
| `AccordionItem` | `sf-section-label` |

## Migration pass order

1. `setting_help.svelte` — retire `text-base` / `text-sm`
2. Form primitives — `sf-field-label`
3. `Button`, `NavButton`, `TabPage`, timestamp components
4. Feature screens: history, scribe, transcribe, onboarding
5. Tighten `check:ds` from warn → error on inline size tokens

Full file list: `docs/typography-audit.md`.
