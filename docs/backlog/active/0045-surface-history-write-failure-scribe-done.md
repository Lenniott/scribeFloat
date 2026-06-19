---
id: "0045"
title: Surface history write failure in Scribe Done event
status: active
priority: medium
---

# Surface history write failure in Scribe Done event

## Problem

In `src-tauri/src/controllers/scribe.rs`, the `write_outputs` method (around line 835) silently swallows a `HistoryService::append` failure:

```rust
let record_id = match self.history.append(&config.save_folder, record) {
    Ok(id) => {
        self.app.emit("note://item-added", ()).ok();
        Some(id)
    }
    Err(e) => {
        tracing::warn!(error = %e, "failed to append scribe history record");
        None  // execution continues — Done event fires with record_id: None
    }
};
// ... later:
self.emit_done(record_id);  // frontend sees success even though the record was lost
```

The `Done` event fires with `record_id: None`, which the frontend treats as success with no note. The user sees the recording complete normally but their note never appears in History. There is no error feedback.

This violates the **error-handling contract**: a failure to persist the primary artifact of a Scribe session must be surfaced to the user, not silently discarded.

## Fix

Propagate the history append failure up through `write_outputs` → caller and emit an error event instead of Done when history cannot be written. Follow the existing error-event pattern already used elsewhere in the controller.

### Step 1 — Change `write_outputs` return type

`write_outputs` currently returns `()` (or swallows errors internally). Change it to return `Result<String, AppError>` where the `String` is the `record_id`. The caller (`stop_and_save` / `do_transcription`) already handles `Result`.

### Step 2 — Propagate the error

```rust
// In write_outputs:
let record_id = self.history.append(&config.save_folder, record)
    .map_err(|e| AppError::Internal(format!("failed to persist scribe session: {e}")))?;
self.app.emit("note://item-added", ()).ok();
Ok(record_id)
```

### Step 3 — Emit error on failure

In the calling site, if `write_outputs` returns `Err`, emit a Scribe error event and set the state machine to `Failed` (matching the pattern for transcription failures). Do **not** emit `Done`.

```rust
match self.write_outputs(config, wav_path, segments) {
    Ok(record_id) => self.emit_done(Some(record_id)),
    Err(e) => {
        tracing::error!(error = %e, "scribe session lost — history write failed");
        self.emit_error(e);
    }
}
```

## Acceptance criteria

- [ ] When `HistoryService::append` fails (simulate by making `save_folder` unwritable), the Scribe controller emits an error event, not `Done`.
- [ ] The error event carries an `AppError::Internal` with a message describing the failure.
- [ ] When `append` succeeds, behaviour is unchanged: `Done` fires with the `record_id`.
- [ ] No history write path silently returns `None` for `record_id` — the type returned from `write_outputs` is `Result<String, AppError>`, not `Option<String>`.
- [ ] `cargo test -p scribefloat` passes.
- [ ] `cargo clippy -- -D warnings` passes.

## Notes

- Check whether `DictateController` has the same silent-swallow pattern in its completion path and apply the same fix if so (see story 0021 for context on Dictate's history write).
- Do not change the `Done` event payload shape beyond what is necessary — the frontend decodes this event.
- The WAV file and transcript file should still be written even if history append fails (they are separate operations). Only the history record append failure should trigger the error path.
