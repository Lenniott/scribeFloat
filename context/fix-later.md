# Fix Later

Deferred work. These are real issues but require more context, wider refactors, or carry non-trivial blast radius. Do them when touching the relevant files, not speculatively.

---

## Architecture

### A1. Move progress emission out of `ModelService`

`ModelService::download_model` takes an `AppHandle` and calls `app.emit()` directly. Services must not know about the UI layer.

**What to do:** Remove `AppHandle` from `download_model`. Accept a `tokio::sync::mpsc::Sender<ModelDownloadEvent>` instead. `ModelController` owns the channel, drains it in a background task, and emits to the UI — the same pattern `ScribeController` already uses for transcription progress.

**Files:** `src-tauri/src/services/model.rs`, `src-tauri/src/controllers/model.rs`

**Prerequisite:** Understand all callers of `download_model`. Run `cargo test` before and after.

**Risk:** Medium — changes the public signature of `download_model`; controller and command must be updated together.

---

### A2. Move window-open calls from commands into controllers

`commands/scribe.rs:59` calls `platform::window_impl::sync_activation_policy` directly. `commands/settings.rs:218` calls `crate::open_settings_window`. Commands should only translate IPC → one controller method.

**What to do:** Add a `destroy` or `close` method to `ScribeController`. Add a `show_window` method to `SettingsController`. Commands call those; window ops live in controllers.

**Files:** `src-tauri/src/commands/scribe.rs`, `src-tauri/src/commands/settings.rs`, `src-tauri/src/controllers/scribe.rs`, `src-tauri/src/controllers/settings.rs`

**Risk:** Low-medium — mostly moving code, but window visibility + activation policy must stay in the right order (see macOS dispatch rules in CLAUDE.md).

---

### A3. Wrap `platform::key_listener` behind a service trait

`DictateController::start_key_listener` calls `crate::platform::key_listener::start_modifier_listener` directly. Controllers should not reach platform code.

**What to do:** Define a `KeyListenerService` trait with a `start(callback)` method. Implement it in `platform/key_listener.rs`. Inject it into `DictateController` in `lib.rs`.

**Files:** `src-tauri/src/controllers/dictate.rs`, `src-tauri/src/platform/key_listener.rs`, `src-tauri/src/lib.rs`

**Risk:** Low — mostly wrapping existing code; the key listener is already tested.

---

### A4. Centralise window management

Window open/close/hide/position logic is spread across `lib.rs` (3 fns), `commands/scribe.rs`, `commands/settings.rs`, `controllers/dictate.rs`. This makes it hard to reason about which windows can be visible simultaneously and what activation policy applies.

**What to do:** Create a `WindowService` (or a `WindowManager` module in `lib.rs`) that owns all `webview_window` operations. Register it as a Tauri state. Controllers and commands call it instead of calling `app.get_webview_window()` directly.

**Files:** `src-tauri/src/lib.rs` + new `src-tauri/src/services/window.rs`

**Risk:** High — touches `lib.rs` setup, window event handlers, and activation policy ordering. Defer until it becomes a pain point. Do A2 first.

---

### A5. Normalise progress-reporting pattern

`ScribeController` uses a channel + drain thread for transcription progress. `DictateController` emits inline. Two patterns for the same concern.

**What to do:** Once A1 is done (ModelService refactored), pick one pattern (channel is more testable) and apply it to DictateController too.

**Risk:** Low, but depends on A1 being settled first.

---

## Reliability

### R1. Windows microphone permission check

`platform/permissions_impl.rs:237-257` checks registry string-contains for `"Allow"`. If the registry key structure changes in a Windows update, the check silently returns `false` and the app blocks mic use.

**What to do:** Add a fallback — if the registry check returns `false`, attempt to open the mic stream anyway and catch the OS error. Surface that error to the user rather than pre-emptively blocking.

**Risk:** Windows-only. No macOS impact. Low blast radius.

---

### R2. Improve IPC error handling in frontend components

Multiple `.catch(() => {})` calls swallow errors silently. The user gets no feedback if an action fails.

| File | Lines | Affected actions |
|---|---|---|
| `src/lib/screens/scribe.svelte` | 185, 217, 218 | settings open, cancel |
| `src/lib/screens/scribe-processing.svelte` | 131, 134, 139 | transcript actions |
| `src/lib/screens/setting_general.svelte` | 39–40 | settings saves |

**What to do:** For user-visible actions, capture the error into a local `$state` variable and render an inline error string. For internal/background calls, at minimum log to `console.error`.

**Risk:** Low — purely additive, no logic changes.

---

## Maintainability

### M1. `scribe.svelte` is 639 lines

The recording state machine, timer, device selection, model management, notes, and 5 event listeners all live in one file.

**What to do — when next modifying this screen:**
1. Extract `<AudioSettingsAccordion>` (mic/speaker device selects + model picker) into its own component
2. Extract recording timer logic into a `useRecordingTimer()` composable
3. Consider splitting model-store subscription into a `<ModelSelector>` component

**Risk:** Medium — any prop/event mismatch breaks the recording flow. Do this incrementally, one extraction at a time, with a full test pass after each.

---

### M2. `HotkeyService` holds `AppHandle`

`TauriHotkeyRegistrar` holds an `AppHandle` to call `app.global_shortcut()`. Services should not depend on the Tauri runtime.

**What to do:** Move `TauriHotkeyRegistrar` out of the `services/hotkeys.rs` module into a Tauri-specific glue layer (e.g. alongside `lib.rs`). The `HotkeyService` receives only the `HotkeyRegistrar` trait object, not the concrete registrar.

**Risk:** Low-medium — `HotkeyService` tests may need updating; the production wiring in `lib.rs` must be updated.

---

### M3. Untyped event name strings

Controllers emit via hardcoded strings like `"scribe://state-changed"`. A typo won't be caught at compile time.

**What to do:** Define event name constants in `types.rs` (or a dedicated `events.rs`):
```rust
pub const SCRIBE_STATE_CHANGED: &str = "scribe://state-changed";
```
Use them everywhere instead of inline strings. Optionally define a type-safe `emit_typed` wrapper.

**Risk:** Low — purely mechanical find-replace.

---

## CSS / Frontend

### C1. `h-8` on `<select>` elements

`<select>` elements in `scribe.svelte:449,472` and `setting_models.svelte:163,190` use `h-8` (32px). The spec defines `h-10` (40px) for normal controls and `h-6` (24px) for small. `h-8` is an in-between value not in the spec.

**What to do:** Decide whether these selects are "small" (switch to `h-6`) or "normal" (switch to `h-10`). They're inline within recording panels so `h-8` may be intentional as a compact variant — in that case, add `h-8` to the design system as an explicit "compact control" size with a note on where it's allowed.

**Risk:** Visual only — measure against neighbouring elements before changing.

---

### C2. Modal shadow ambiguity

`src/lib/components/Modal.svelte:35` uses `shadow-ambient`. The design spec says shadow is for PanelShell only. A modal that appears inside an already-windowed context may not need it.

**What to do:** Decide if Modal is considered a shell-level component. If yes, keep `shadow-ambient`. If no, remove it and rely on `border-card` for separation. Document the decision in `context/componets.md`.

---

## How to prioritise

```
Do first (unblocked, low risk):
  A2 → commands → controllers (small moves, no signature changes)
  A3 → key listener service wrapper
  R1 → Windows permission fallback
  R2 → IPC catch improvements (purely additive)
  M3 → event name constants (mechanical)

Do second (depends on settling A1 first):
  A1 → ModelService progress channel
  A5 → normalise progress pattern

Do last (highest blast radius):
  A4 → WindowService
  M1 → scribe.svelte split
```
