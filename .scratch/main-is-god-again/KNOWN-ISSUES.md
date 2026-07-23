# Known issues — Main is God again

Non-blocking debt dump. Capture anything that should not redefine the merge destination.

Format per item:

```markdown
## <short name>
- **Seen:** <where / when>
- **Notes:** <what’s wrong>
- **Later:** <vague idea of fix, optional>
```

<!-- items go below -->

## TCC prompts fire too early (Input Monitoring + Documents)

- **Seen:** Silicon ship-bar smoke on installed `.app` (2026-07-19); reconfirmed 2026-07-21 cold onboarding; human OK park; **Keystroke-under-Welcome reconfirmed 2026-07-23**
- **Notes:** ~~On launch, `start_key_listener` opens a `CGEventTap` → macOS “Keystroke Receiving” / Input Monitoring dialog before onboarding asks. Dialog often **stacks under** the Setup window.~~ **Fixed 2026-07-23:** Dictate key listener deferred until Input Monitoring is already granted (`CGPreflightListenEventAccess`); starts after Permissions Grant / status poll / returning-user launch. Documents access can still prompt early because default save folder is under `~/Documents/…`. Separate closed merge-blocker: [Persist onboarding step across quit](./issues/19-persist-onboarding-step-across-quit.md).
- **Later:** Defer Documents touch until after permissions / explicit folder setup. Keystroke-on-load path closed.

## Onboarding Dictate practice pays cold Whisper load

- **Seen:** Silicon ship-bar smoke onboarding “Try Dictate” (2026-07-19); human OK park
- **Notes:** Whisper preload only starts when recording starts (`spawn_record_start_preload`). First practice capture waits ~15s for model load — bad first impression of Dictate speed. Idle time after mic is granted could warm the model in the background.
- **Later:** Preload Whisper after permissions granted (before / during Try Dictate). Same idea helps post-onboarding first Dictate.

## Onboarding Try Dictate shows nonsense timestamps

- **Seen:** Silicon ship-bar smoke Try Dictate (2026-07-21) — e.g. `495740:07:43`
- **Notes:** Practice history cards show impossible hour counts. Capture text itself looked fine. Does not block finishing Setup once Continue is reachable.
- **Later:** Format relative/absolute time from a real wall-clock or session offset; fix whatever unit mix produces multi-hundred-thousand “hours.”

## Onboarding should teach double-tap and tap-and-hold

- **Seen:** Human product intent during 2026-07-21 smoke; Try Dictate step today is mostly “double-tap Control”
- **Notes:** Want the practice step to **gamify** both activation styles (double-tap and tap-and-hold) so first-run users learn the real Dictate muscle memory. Polish / pedagogy — not a ship-bar fail if one path works.
- **Later:** Practice UI that prompts both gestures and celebrates each; keep Continue honest (see merge-blocker [Onboarding Try Dictate Continue reachable](./issues/20-onboarding-try-dictate-continue-reachable.md)).

## Onboarding “You’re All Set” tray mockup is stale

- **Seen:** Silicon ship-bar smoke final onboarding screen vs live menu bar (2026-07-21)
- **Notes:** Mockup still shows Scribe / Transcribe / History / Settings. Live tray is Dictate, New note, Open ScribeFloat, Settings, Quit (and Quit casing is inconsistent). Mild honesty debt — real tray works; Setup picture lies. Related to Record vs Dictate naming Known issues, but this is specifically the onboarding illustration.
- **Later:** Redraw mockup to match the live tray; fix Quit label casing while there.

## Record vs Dictate naming / dual-controller honesty

- **Seen:** Sort A3 elevated to merge-blocker; ticket 17; human 2026-07-19 demoted
- **Notes:** Product says Record; much code/UI still says Scribe. Dual `ScribeController` / `DictateController` is evolution debt (ADR-0003 deferred unify). Deeper than a quick S/M/F cut — needs its own wayfinder later. Spec draft retained on [Unify Record and Dictate naming and seams](./issues/17-unify-record-dictate-naming-and-seams.md).
- **Later:** Post-merge wayfinder; do not block Silicon smoke or untagged main merge.

## Speaker rename edge cases

- **Seen:** Sorting session 2026-07-19; Silicon smoke Note transcript 2026-07-21 (labels like `[Laura]` / `[Ben]` present)
- **Notes:** Renaming a speaker should update all their turns in that note (ship-bar cascade) — **proven 2026-07-21**. Product also wants an explicit choice: **this label only** vs **all turns with this name** (e.g. every “Speaker 1”). Use case: Sortformer got it *mostly* right — fix a few mislabeled turns without renaming the whole speaker. Today rename-all is what works; single-occurrence fix is missing.
- **Later:** UI affordance for single-occurrence vs rename-all. Cascade path itself is not a merge-blocker.

## Note written pane does not fill editor height

- **Seen:** Silicon smoke Note editor Written tab (2026-07-21)
- **Notes:** Written area sits in a short bordered box instead of filling the vertical space beside Transcript. Content editable, just cramped layout.
- **Later:** Make the written editor stretch to the available pane height.

## Dictate overlay flaky in macOS full-screen / other Spaces

- **Seen:** Silicon smoke Dictate while a full-screen app is frontmost (2026-07-21)
- **Notes:** Overlay often missing on the full-screen Space; sometimes “follows” when switching back to the main desktop, sometimes not. **Capture still works** (double-tap and hold both OK). Visibility / Space affinity only.
- **Later:** Pin or re-parent the Dictate panel to the active Space / full-screen context so the HUD is visible where the user is typing.

## Opening main window from tray lands on full-screen Space

- **Seen:** Silicon smoke — open app from menu-bar (tray) while another app is full-screen (2026-07-21)
- **Notes:** Main window can appear on top of / inside the full-screen Space instead of the primary desktop. Weird macOS Space behaviour; Capture/Notes content OK once you find the window.
- **Later:** Force “Open ScribeFloat” / main window onto the primary desktop Space, not over a full-screened foreign app.

## Record button context: new note vs continue in note

- **Seen:** Human product note during Silicon smoke (2026-07-21); marked Known issue
- **Notes:** From Home or Upload, Record creates a **new** Note and starts recording. From inside an open Note, the same control only starts recording **into that Note**. Easy to surprise users who expect one behaviour everywhere. Intentional-ish dual meaning; document or clarify in UI later.
- **Later:** Distinct labels / confirmations, or a single explicit “New note & record” vs “Record into this note.”

## Focus ring hidden or overridden by styling

- **Seen:** Silicon smoke Settings / inputs (2026-07-21) — thick orange focus border on fields; human notes focus “hidden by styling in places”
- **Notes:** Keyboard/focus affordance inconsistent; some controls may not show a clear focus state. Accessibility / polish, not a ship-bar content fail.
- **Later:** Align focus rings with design tokens; ensure visible `:focus-visible` on interactive controls.

## Dual audio vs how many speakers we can get

- **Seen:** Human thought while sorting — Record with speaker capture (mic file + system/speaker file)
- **Notes:** Sortformer is a “up to 4 speakers” model. With two audio files, it is natural to wonder if we could get up to 4 per file (8 total). Today, when both mic and speaker audio are used, the code path tends to label by **channel** (mic vs speaker) rather than running the 4-speaker model on each file and combining. Worth reading the merge path carefully before changing anything.
- **Later:** Decide if dual-source should stay “two channels”, or diarize each file then merge labels — product call, not merge-critical.

## Start transcript work before Dictate fully stops

- **Seen:** Human thought while sorting — long Dictate (~5 min); waiting for the whole stop→transcribe feels slow
- **Notes:** Right now transcription runs after you finish. Idea: treat a long silence as “this chunk is done” and run Whisper on finished chunks while recording continues, so at the end you only wait on the last unfinished bit. Unknown how that affects accuracy and glue between chunks.
- **Later:** Explore after main merge; compare accuracy vs “one shot at the end.”

## Unfinished Notes UI leftovers

- **Seen:** Note editor metadata / tags copy
- **Notes:** Bits that say tags/keywords are not wired yet. Not lying about Float in the sidebar anymore, but still unfinished chrome.
- **Later:** Strip or build when Notes product work resumes.

## Skills / plans still mention deleted Models screen

- **Seen:** e.g. `skills/ui-taxonomy/SKILL.md` and old plans
- **Notes:** Agent-facing leftovers only; users never see them. Can confuse agents who read those files.
- **Later:** Clean when editing those skills/plans.

## Old biometric fields in history.jsonl until compact finishes

- **Seen:** Security review S3
- **Notes:** Exploration-only voiceprint (never released — human’s machine / branch fog only). Old test notes could still have embedding fields on disk until startup compact rewrites the file. New notes do not write those fields. Not a multi-user upgrade issue.
- **Later:** Smoke-check after launch, or wipe test data; then stop talking about voiceprint.

## Upload can read any audio path you pass it

- **Seen:** Security review S7
- **Notes:** Needed for “pick a file.” Risk is higher if the web UI is compromised. Fixing transcript HTML + IPC lockdown reduces blast radius.
- **Later:** Dialog-only tokens if Upload is redesigned (out of scope this map).

## Opening transcript output allows any .md path

- **Seen:** Security review S8
- **Notes:** Other open helpers stay inside the save folder; this one only checks `.md`.
- **Later:** Confine to save folder like the other open helpers.

## Broad opener / dialog plugin permissions

- **Seen:** Security review S9
- **Notes:** Update check can open a release URL from GitHub JSON. Fine when honest; wide if metadata is bad.
- **Later:** Tighter URL allowlist.

## Windows file-open / “open with” app path

- **Seen:** Security review S10
- **Notes:** Silicon-first map. Windows open helpers are a later hardening pass.
- **Later:** After main is clean / when Windows care returns.

## Accessibility + auto-paste on by default

- **Seen:** Security review S11
- **Notes:** Needed for Dictate paste. Documented. Optional later: default auto-paste off for a stricter bar.
- **Later:** Product call.

## Heavy native ML / dependency surface

- **Seen:** Security review S12
- **Notes:** Whisper, ONNX, input simulation, etc. Normal for this app; no stack rewrite this map.
- **Later:** Advisory CI if we want it as a habit.

## Bring back spoken triggers as Dictate prompt / insert text

- **Seen:** Human 2026-07-19 — product intent after merge; not a merge-blocker
- **Notes:** We removed the old “text replacement” / word-replacement engine (`0f35959` — dropped `services/output/replacements.rs`, Replacements settings tab, and call sites in Dictate/Record/history/export). Human wants something like that engine back, but **reshaped**: Dictate-only, used to insert prompts or extra text into dictation — not a Record/Scribe feature and not the old general replacements product surface. Engine bones still exist in git history and backup branches (e.g. parent of `0f35959`, `backup/feature-0.3-embeds-pre-cleanup-20260717`); need to find the best recover point and decide what to keep vs redesign.
- **Later:** After main is clean — recover from a known branch/commit, cut scope to Dictate insert/prompt behaviour, leave Record alone.

## CSP: style-src left unmodified by Tauri (`dangerousDisableAssetCspModification`)

- **Seen:** 2026-07-23 — release-build styling broke (CodeMirror theme, skeleton) while dev looked fine
- **Notes:** Tauri's build-time CSP rewrite adds nonces/hashes to `style-src`, and per the CSP spec any nonce/hash makes `'unsafe-inline'` ignored — so every runtime-injected `<style>` (CodeMirror style-mod, CSS-in-JS) was blocked in the packaged app only. Fix: `"dangerousDisableAssetCspModification": ["style-src"]` in `tauri.conf.json`. `script-src` keeps its nonces, so ticket 13's sanitization posture is unchanged. Residual risk is injected *styles* only (low: `connect-src`/`img-src` stay locked). Do not "clean up" this flag — removing it re-breaks release styling.
- **Later:** If CSP is ever tightened, move CodeMirror styling to `adoptedStyleSheets` or hashed static CSS first.
