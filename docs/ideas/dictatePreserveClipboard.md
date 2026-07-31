# Dictate: Preserve Pre-Existing Clipboard Content

## Summary
- Difficulty: **medium**, and not uniformly so — straightforward for the manual-paste path, genuinely tricky for auto-paste because of a timing race.
- Problem: Dictate unconditionally overwrites the clipboard with the transcription. If the user had something copied that they meant to paste somewhere (e.g. into a model chat) but paused to dictate a message first, that original clipboard content is gone by the time they go back for it — even though Dictate's own paste happened in a totally different place.
- Idea: snapshot clipboard content right before Dictate writes the transcription, and restore it once Dictate's own paste is done (or once the transcription has served its purpose).

## Where clipboard is touched today
- `src-tauri/src/controllers/dictate.rs:943` — `self.app.clipboard().write_text(text.clone())` overwrites the clipboard unconditionally on every Dictate completion, regardless of auto-paste setting.
- `src-tauri/src/controllers/dictate.rs:963-990` — if `config.dictate_auto_paste` is true, `paste_on_main_thread` re-verifies clipboard content still matches the expected text (existing hijack guard, `dictate.rs:1148-1160`) and then simulates Cmd/Ctrl+V via `src-tauri/src/platform/paste_impl.rs`.
- If auto-paste is off, the transcription is left on the clipboard indefinitely by design, so the user can paste it manually whenever they're ready.

## Feasibility notes
- **Manual-paste (auto-paste off) doesn't fit a simple restore.** The whole point of leaving the text on the clipboard is for a later, user-initiated paste — sometimes much later. Restoring the old clipboard content right after Dictate finishes would defeat that path entirely. Any restore has to be scoped to auto-paste, or gated on the transcription actually being consumed rather than a fixed delay.
- **Auto-paste has a real race condition — this is probably the "unknown issue."** `paste_text()` synthesizes a Cmd+V keypress via `enigo` and returns as soon as the OS event is dispatched; it does not wait for the target app to actually read the clipboard. If we restore the original content immediately after `paste_text()` returns, there's no guarantee the target app has consumed the dictated text yet — it could read the restored old content instead and paste the wrong thing. There's no OS-level signal for "the target app finished reading the clipboard" after a synthesized keystroke, so this needs either a deliberately conservative delay (with the UX cost of feeling slow) or some other mitigation. The existing hijack guard is the mirror-image problem (verify our text is still there *before* pasting) — restore needs an equivalent safeguard in the other direction.
- **Auto-Enter compounds the timing risk.** When `dictate_auto_enter` is also on, Dictate sends Enter right after paste. A restore would need to wait until after both paste and Enter land, with the same race at each step.
- **Failure paths intentionally leave text on the clipboard.** If paste fails (`paste_failed` branch, `dictate.rs:992-1012`), the transcription is deliberately left on the clipboard as a fallback so the user can paste it manually. Restoring on failure would silently destroy that fallback and lose the transcription.
- **Only plain text is handled today.** `src/lib/services/clipboard.ts` and the Rust clipboard calls only use `read_text`/`write_text`. If the user's original clipboard held an image, file reference, or rich HTML, a save/restore built on text alone would drop it (or flatten it to plain text). Scoping v1 to text-only clipboard content is probably fine, but should be a stated decision, not an accident.

## Rough shape of a v1 (not committed to — for later scoping)
- Read `app.clipboard().read_text()` right before the existing write at `dictate.rs:943`; carry it alongside the dictated text through the paste pipeline.
- Only attempt restore when `config.dictate_auto_paste` is true and the paste (and Enter, if enabled) completed successfully.
- Restore after a short settle delay following paste/Enter, not immediately — timing needs manual verification per target app; the existing 150ms settle sleep runs *before* paste today, so this would need a new one *after*.
- Skip restore entirely when paste failed, or when the pre-Dictate clipboard read failed or was empty.

## Open questions
- Text-only scope acceptable for v1, or does this need to cover images/HTML too?
- What's a safe restore delay — fixed sleep, or does paste-completion timing vary enough across target apps to need something smarter?
- Always-on behavior, or a Settings toggle (in case someone relies on Dictate's current "leaves clipboard set to the transcription" side effect)?

## Assumptions
- Not scoped to a build yet. This captures the problem and initial feasibility findings from a first look at `dictate.rs` / `paste_impl.rs`, logged per user request — no code changes made.
