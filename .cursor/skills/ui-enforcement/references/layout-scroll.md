# Layout & scroll

> Chrome stays visible; one body region scrolls. Primitives: `ScrollBody`, `PanelHeader`, `PanelFooter`.

## Mental model: chrome + body

Most screens are a **bounded flex column** with three zones:

```
┌─────────────────────────────────────┐
│  CHROME (shrink-0)                  │  title, actions, tabs, banners — height varies
├─────────────────────────────────────┤
│  BODY (min-h-0 flex-1, scrolls)     │  lists, transcript text, settings forms
├─────────────────────────────────────┤
│  FOOTER (shrink-0, optional)        │  primary actions — PanelFooter
└─────────────────────────────────────┘
```

- **Chrome** = anything that must stay reachable while the user scrolls long content.
- **Body** = the **only** vertical scroll container in the pane.
- **Footer** = optional bottom actions; also `shrink-0`, not `position: fixed` or `sticky`.

Chrome height is **intrinsic** (content + padding). Multiple chrome blocks stack as sibling `shrink-0` elements (header + warning banner + tab row). Optional `min-h-*` sets a floor only — not a fixed height.

## Two approved screen patterns

Pick **one** per screen or pane.

### A — Page scroll

Use when there is **no persistent chrome** (no tabs, filters, or actions that must stay on screen).

```svelte
<div class="flex h-full flex-col overflow-y-auto p-6">
  <!-- title scrolls away with content -->
</div>
```

**Examples:** `dashboard.svelte`.

### B — Region scroll (default for app panes)

Use when chrome must stay visible (lists, detail, settings, recording panes, filter columns).

```svelte
<div class="flex h-full min-h-0 flex-col overflow-hidden">
  <header class="shrink-0 ...">...</header>
  <!-- optional: more shrink-0 chrome (banner, tabs, chip row) -->
  <ScrollBody class="px-6 pb-6">
    ...
  </ScrollBody>
  <!-- optional -->
  <PanelFooter>...</PanelFooter>
</div>
```

**Examples:** `SettingsPanel`, `NoteDetailPane`, `FilterPanel`, `scribe.svelte` settings column, `transcripts.svelte` (list mode — structure is correct; needs height chain).

## Height chain (required for pattern B)

Scrolling fails when **any link** in the ancestor chain is missing a bound. Trace from the scroll container up to `h-screen`:

| Layer | Required classes | Notes |
|-------|------------------|-------|
| Window / shell root | `h-screen flex flex-col overflow-hidden` | `app-shell.svelte` |
| Shell body row | `flex min-h-0 flex-1 overflow-hidden` | sidebar + main |
| `<main>` (app shell) | `flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden` | must be a **flex host** |
| Screen root | `h-full min-h-0 flex flex-col overflow-hidden` | **not** bare `flex-1` on a block parent |
| Chrome siblings | `shrink-0` | any count, any natural height |
| Scroll body | `min-h-0 flex-1 overflow-y-auto` | use `ScrollBody` |

### Why `min-h-0` matters

Flex items default to `min-height: auto` (size to content). Without `min-h-0`, the scroll region grows with its children and `overflow-y-auto` never activates.

### Why `flex-1` on screen roots fails

`flex-1` only sizes against a **flex parent**. `<main>` without `display: flex` ignores child `flex-1`; the screen grows to content height and gets **clipped** by `overflow-hidden` on `<main>`. Use **`h-full`** on screen roots, or make `<main>` a flex column.

## Primitives (use these — do not re-type overflow recipes)

| Component | Role |
|-----------|------|
| `ScrollBody` | Body slot: `min-h-0 flex-1 overflow-y-auto overscroll-contain` |
| `PanelHeader` | Top chrome: title + left/center/right action slots, `shrink-0` |
| `PanelFooter` | Bottom chrome: actions, `shrink-0` (not fixed/sticky) |

`ScrollBody` source:

```svelte
<div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-1 {className}">
```

Pass padding via `class` on `ScrollBody` (e.g. `class="px-6 pb-6"`).

## Horizontal overflow

- Tab bars and chip rows: `overflow-x-auto` on the **tab list only**, not the page root.
- Page/pane roots: `overflow-hidden` (vertical scroll lives in the body slot).

## App shell contract

Routes inside `app-shell.svelte` assume:

1. `<main>` is `flex flex-col` with bounded height.
2. Each screen root uses `h-full min-h-0 flex flex-col overflow-hidden` (pattern B) or `h-full overflow-y-auto` (pattern A).

New shell screens must close the height chain — do not rely on `flex-1` alone.

## Side columns (filter panel, notes, settings nav)

Same chrome + body rules inside the column:

```svelte
<aside class="flex h-full min-h-0 shrink-0 flex-col ...">
  <div class="shrink-0">...</div>
  <ScrollBody class="p-4">...</ScrollBody>
  <div class="shrink-0">...</div>
</aside>
```

Parent row must be `flex` with bounded height (`min-h-0 flex-1 overflow-hidden`).

## Banned / avoid

| Anti-pattern | Why |
|--------------|-----|
| `flex-1` on screen root when parent is not `display: flex` | No height bound → clip or no scroll |
| `flex-1 overflow-y-auto` without `min-h-0` | Flex child won't shrink → no scroll |
| Multiple nested `overflow-y-auto` in one pane | Unclear scroll target; one body only |
| `position: sticky` / `fixed` for pane chrome | Use flex `shrink-0` siblings; see `PanelFooter` comment |
| Sprinkling `overflow-hidden` on every wrapper without a chain plan | Masks broken layout; fix the chain instead |
| `overflow-y-scroll` on panes | Prefer `overflow-y-auto` (or `ScrollBody`) unless scrollbar gutter is intentional |

## Checklist before shipping a new screen

- [ ] Picked pattern A or B
- [ ] Screen root has `h-full` (or parent is flex and child uses `flex-1` **with** `min-h-0`)
- [ ] Pattern B: chrome is `shrink-0`; body is `ScrollBody` or equivalent with `min-h-0 flex-1`
- [ ] Exactly **one** vertical scroll container per pane
- [ ] App shell / parent panes pass bounded height (`h-screen` → … → `h-full`)

## Reference implementations

| File | Pattern | Notes |
|------|---------|-------|
| `dashboard.svelte` | A — page scroll | Simple overview |
| `SettingsPanel.svelte` | B — header + banner chrome + scroll section | Variable-height chrome stack |
| `NoteDetailPane.svelte` | B — `PanelHeader` + chips + `ScrollBody` | Detail reference |
| `FilterPanel.svelte` | B — header + scroll + footer | Side column |
| `scribe.svelte` | B — grid columns each with own scroll body | Multi-region |
| `transcripts.svelte` | B — title/tabs chrome + list body | Fix height chain + `min-h-0` on body |

## Related docs

- `context/components.md` — `ScrollBody`, `PanelHeader`, `PanelFooter` catalogue
- `docs/history-ui-review.md` — History detail footer uses `PanelFooter`, not `FixedFooterBar`
