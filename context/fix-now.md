# Fix Now

Low-risk, high-confidence changes. Each is self-contained and safe to land immediately.
Verify with `npm run check:ds` after completing the CSS fixes — it should exit 0.

---

## 1. CSS token violations (15 errors across 8 files)

Run `npm run check:ds` to see the current list at any time.

### 1a. `surface-low` → `card` (borders and inline styles)

These files use `border-*-surface-low` or `var(--color-surface-low)` — neither exists in `app.css`. The correct token for a low-contrast divider line is `card`.

| File | Line | Old | New |
|---|---|---|---|
| `src/lib/screens/loading-screen.svelte` | 2 | `border-b-surface-low` | `border-card` |
| `src/lib/screens/loading-screen.svelte` | 16 | `border-l-surface-low` | `border-card` |
| `src/lib/screens/scribe-processing.svelte` | 171 | `border-b-surface-low` | `border-card` |
| `src/lib/screens/scribe-processing.svelte` | 248 | `border-t-surface-low` | `border-card` |
| `src/lib/components/layout/FixedFooterBar.svelte` | 9 | `var(--color-surface-low)` in inline style | `var(--sf-card)` |
| `src/lib/components/notes/NotesPanel.svelte` | 40 | `var(--color-surface-low)` in inline style | `var(--sf-card)` |

### 1b. `on-surface-dim` / `on-surface` → `fg-muted` / `fg`

| File | Line | Old | New |
|---|---|---|---|
| `src/lib/components/audio/AudioLayerLegend.svelte` | 14 | `bg-on-surface-dim` | `bg-fg-muted` |
| `src/lib/components/audio/AudioLayerLegend.svelte` | 19 | `bg-on-surface/40` | `bg-fg/40` |
| `src/lib/screens/scribe-processing.svelte` | 237 | `decoration-on-surface-dim` | `decoration-fg-muted` |

### 1c. `text-void` → `text-on-brand`

| File | Line | Old | New |
|---|---|---|---|
| `src/lib/components/form/StackProgressBar.svelte` | 120 | `text-void` | `text-on-brand` |

### 1d. `rounded-lg` → `rounded-md`

| File | Line | Old | New |
|---|---|---|---|
| `src/lib/screens/dictate.svelte` | 144 | `rounded-lg` | `rounded-md` |

### 1e. `shadow-lg` → `shadow-ambient` or remove

Design rule: shadow only on PanelShell. The dictate HUD (`dictate.svelte`) IS the floating shell so `shadow-ambient` is correct. The settings panel (`settings.svelte`) sits inside a window that already has chrome — remove it.

| File | Line | Old | New |
|---|---|---|---|
| `src/lib/screens/dictate.svelte` | 144 | `shadow-lg` | `shadow-ambient` |
| `src/lib/screens/settings.svelte` | 48 | `shadow-lg` | *(remove)* |

---

## 2. Reliability: dictate history write failure is silent

When writing the dictate history entry fails (disk full, bad permissions), the user sees a successful `DONE` state and the transcription is silently dropped from the log.

**Files to change:**
- `src-tauri/src/types.rs` — add `history_write_failed: bool` to `DictateStateEvent` (with `#[serde(default)]`)
- `src-tauri/src/controllers/dictate.rs` — set `history_write_failed: true` in the event when `write_dictate_history_entry` returns `Err`; currently line ~488 only does `eprintln!`
- `src/lib/screens/dictate.svelte` — show inline warning when `history_write_failed` is true in the `DONE` state (e.g. "Pasted. History entry could not be saved — check save folder.")

---

## 3. Reliability: clipboard write failure not guarded before paste

If `clipboard().write_text()` fails, the paste simulation still fires and the user gets silence. The text is lost.

**File:** `src-tauri/src/controllers/dictate.rs` ~line 492

Change:
```rust
if let Err(e) = self.app.clipboard().write_text(text.clone()) {
    eprintln!("[dictate] failed to write clipboard: {e}");
}
// ... paste proceeds regardless
```

To: return an error state early if clipboard write fails, before attempting paste. Include the text in the error payload so the UI can display it.

---

## 4. Reliability: orphaned `.tmp` on model download failure

A failed download leaves a partial `.tmp` file with no cleanup.

**File:** `src-tauri/src/services/model.rs` — in the error path after the download loop, add:
```rust
let _ = tokio::fs::remove_file(&tmp).await;
```
One line, zero risk.

---

## How to verify

```bash
# CSS violations — should exit 0 after fixes
npm run check:ds

# Rust — should pass after reliability fixes  
cargo test

# Visual smoke test (both themes)
cargo tauri dev
# Open dictate HUD in light mode — check no shadow/radius oddities
# Open settings — check no shadow on the panel
# Trigger a dictate — check DONE state renders correctly
```
