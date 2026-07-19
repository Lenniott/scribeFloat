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

## Speaker rename edge cases

- **Seen:** Sorting session 2026-07-19; feature exists in Notes
- **Notes:** Renaming a speaker should update all their turns in that note. We have not proven every awkward case yet.
- **Later:** Exercise in Silicon ship-bar smoke; fix only if something real breaks.

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
- **Notes:** Very old voiceprint-era notes could still have embedding fields on disk until startup compact rewrites the file. New notes do not write those fields.
- **Later:** Smoke-check after launch, or wipe test data.

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
