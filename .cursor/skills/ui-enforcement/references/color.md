# Color

> Tokens in `src/app.css`. Migration inventory: `docs/color-audit.md`.

## Decision: Option A (locked)

Use **semantic foreground tokens only** — no `text-fg/{opacity}` in product UI.

```bash
python3 context/design-skill/query.py ds get rules.colors
```

## Foreground mapping

| Meaning | Class | Replaces |
|---------|-------|----------|
| Primary | `text-fg` | `text-fg/90`, `text-fg/85`, bare primary body |
| Secondary (must stay readable) | `text-fg-dim` | `text-fg/80`–`/50`, icons, chevrons, captions, metadata on **any** surface |
| Recessed copy only | `text-fg-muted` | helper/empty/disabled **prose** on canvas or panel — **not** icons or card-row chrome |

### Contrast rule (do not map opacity mechanically)

Old `text-fg/{opacity}` **blended with the surface**. On dark `bg-card`, `text-fg/50` lands near **L≈0.60** — the same band as `text-fg-dim`, not `text-fg-muted` (L≈0.40, only ~0.12 above card → fails contrast).

| Job | Class | Never use |
|-----|-------|-----------|
| Expand/collapse icon, row chevron | `text-fg-dim` | `text-fg-muted` |
| Secondary label next to primary | `text-fg-dim` | |
| Fine-print helper under a field | `text-fg-muted` | on `bg-card` without checking |
| Empty state / "optional" / disabled hint | `text-fg-muted` | on icons or controls |

**Banned in new `.svelte` code:** `text-fg/\d+`, `bg-fg/\d+`, `bg-black/\d+`.

## Surfaces

| Level | Class |
|-------|-------|
| Page | `bg-canvas` |
| Panel | `bg-panel` / `sf-panel` |
| Card | `bg-card` / `sf-card` |
| Fill | `bg-fill` |
| Border | `border-rim`, `border-fill`, `border-card` |
| Modal scrim | `sf-scrim` or `bg-overlay` |

## State colors (meaning only)

| Token | Use |
|-------|-----|
| `brand` + `on-brand` | Primary CTA |
| `active` + `bg-active/15` | Selected tab/row |
| `destructive` | Errors, delete, recording dot |
| `warning` | Caution banners, links |
| `success` | Granted / transient OK |
| `focus` | Keyboard ring only |

## Recipes

```svelte
<!-- Selected tab -->
class="border-b-2 border-active bg-active/15 text-fg"

<!-- Warning banner -->
class="border-b border-warning bg-warning/15 text-fg"

<!-- Error banner -->
class="border border-destructive/40 bg-fill text-destructive"

<!-- Modal backdrop -->
class="fixed inset-0 sf-scrim"

<!-- Input -->
class="sf-input"
```

## Foundation vs migration

**Done (do not re-litigate):** `--sf-overlay`, `bg-overlay`, `.sf-scrim`, `sf-field-label` → `text-fg-dim`, `check:ds` warnings, hook deny on new opacity in `.svelte`.

**Not started:** Component file migration (~173 opacity usages). See pass order in `docs/color-audit.md`.

## Pass order (components only)

1. Modal + settings scrim (`bg-black/50` → `sf-scrim`)
2. High-churn files (history detail, scribe-processing, settings, transcribe)
3. Semantic misuse (models `text-active`, Welcome `text-brand`)
4. Primitives (`sf-input`, `sf-divider`)
5. Promote `check:ds` to error
