# Platform Rules

> Load this when touching macOS threading, audio drain, paste behaviour, or platform/ code.

---

## macOS main-thread / dispatch-queue rules

macOS 13+ strictly enforces which APIs may be called from which thread. Violations produce `dispatch_assert_queue_fail` (SIGTRAP / EXC_BREAKPOINT) and kill the process silently — no panic, no error message.

**APIs that require the main dispatch queue:**
- `enigo` keyboard simulation (`CGEventCreateKeyboardEvent`) — used in `platform/paste_impl.rs`
- Any AppKit window operation: `show()`, `hide()`, `set_focus()`, `set_position()`
- `TSMGetInputSourceProperty` — called internally by `rdev` on macOS for every key event

**Rules:**
1. Never call `paste_text()` or `send_enter()` from a tokio thread or `spawn_blocking`. Always dispatch via `app.run_on_main_thread(|| { ... })`. If you need the result back, bridge with a `std::sync::mpsc::channel`.
2. Never call `window.show()`, `window.hide()`, or `window.set_focus()` from any thread other than the main thread. Use `app.run_on_main_thread` for all window visibility operations outside of Tauri command handlers (which already run on the main thread).
3. **Never use `rdev::listen` on macOS.** rdev calls `TSMGetInputSourceProperty` for every key event on its listener thread, which asserts on macOS 13+. Use the `CGEventTap` implementation in `platform/key_listener.rs` instead — it reads raw keycodes with no string conversion.
4. **`set_focus()` on a window while the app is in `.accessory` activation policy kills the process.** An app enters accessory mode via `setActivationPolicy(.accessory)` or `set_dock_visibility(false)`. Never call `set_focus()` on a window when the app may be in accessory mode. The dictate HUD intentionally never calls `set_focus()` for this reason.
5. **`LSUIElement = true` in `Info.plist` is necessary but not sufficient:** Tao defaults to Regular activation at launch, so the Dock stays visible until `set_dock_visibility(false)` runs. **`setup` ends with `sync_activation_policy`** after prewarming windows so a tray-only start hides the Dock. Do not remove that call.

**Paste focus timing:**
When simulating a paste (Cmd+V) after dictation, the dictate HUD window must be hidden *before* the keypress is simulated. If the HUD is still visible, it holds focus and Cmd+V fires into the HUD rather than the previously active app. Hide first, sleep ~150ms for the OS to restore focus, then paste. See `DictateController::paste_on_main_thread`.

---

## Audio drain / MicSession rules

`MicSession::stop_and_take()` blocks until the audio sender is dropped. On macOS, cpal tears down its CoreAudio stream **asynchronously** — the callback closure (which holds the `Sender`) may not be dropped immediately after `drop(_stream)`. For short recordings (< ~2 s), this causes an indefinite hang.

**Rule:** Always use `recv_timeout` in the drain loop, not `recv`. The current implementation uses a 200ms timeout — if no chunk arrives within 200ms after the stream is dropped, the drain is considered complete. Never change this back to blocking `recv()`.

---

## Platform code rules

- `#[cfg(target_os = "macos")]` and `#[cfg(target_os = "windows")]` belong only inside `src-tauri/src/platform/`.
- If you add macOS-specific code, add a Windows stub (even if it just returns `Ok(())` or `false`) so the project compiles on both platforms.
- macOS FFI (`objc2`, `dispatch2`) uses `unsafe`. Before adding unsafe blocks: check for a safe abstraction in those crates first, and add a comment explaining why unsafe is necessary.
- Verify the other platform still compiles with `cargo check --target x86_64-pc-windows-msvc` before committing.
