# Manual Test Plan

Covers backlog slice: UX cleanup + word replacement prefix (branch `claude/backlog-research-plan-BtymN`).

---

## TC-01 — Processing screen: no custom X button

1. Open Scribe → record → Stop and Save.
2. Observe the processing screen header.

**Expected:** No X/close button in the header. The OS window chrome (red dot on macOS) is the only close control.

---

## TC-02 — OS close works during and after processing

1. During transcription: click OS close → abort modal appears (see TC-04).
2. After done: click OS close → window closes cleanly.

**Expected:** No hang, no error; window closes normally after done.

---

## TC-03 — Scribe recording screen: no custom close button

1. Open Scribe in idle or recording state, inspect header.

**Expected:** Only settings cog icon in header; no X button.

---

## TC-04 — Abort confirmation modal during transcription

1. Open Scribe → record → Stop and Save.
2. While `LOADING_MODEL` or `TRANSCRIBING_AUDIO` is shown, click OS close.

**Expected:** "Abort transcription?" modal appears.
- "Keep Processing" closes modal and resumes.
- "Abort" stops transcription and closes window.

---

## TC-05 — Reopen from tray after done (window previously hidden)

1. Record → stop → wait for done screen → close via OS close.
2. Click tray icon to reopen Scribe.

**Expected:** Idle recording screen with "Start Recording" button, not the stale done screen.

---

## TC-06 — Reopen from tray after done (window already visible)

1. Record → stop → done screen still open on screen.
2. Click tray icon again.

**Expected:** Transitions to idle recording screen.

---

## TC-07 — "Record Again" button still works

1. On done screen, click "Record Again".

**Expected:** Returns to idle recording screen directly.

---

## TC-08 — Keyboard shortcuts at top of Settings

1. Open Settings → General tab.

**Expected:** Open Scribe hotkey and Dictate hotkey info blocks appear at the very top, above Theme selection. Each block is visually distinct (rounded card with `bg-fill` background).

---

## TC-09 — "Open transcripts with" hidden when Markdown off

1. Open Settings → General; confirm "Save transcripts as Markdown" is OFF.

**Expected:** "Open transcripts with" path field is not visible.

---

## TC-10 — "Open transcripts with" appears when Markdown toggled on

1. Toggle "Save transcripts as Markdown" ON (no save needed).

**Expected:** "Open transcripts with" field appears directly below the toggle immediately, without saving.

---

## TC-11 — Speaker label group: macOS without BlackHole

1. On macOS with no BlackHole installed; open Settings → General.

**Expected:** "Speaker capture device name" field NOT visible. "Input label", "Output label", and "Capture speaker by default" all hidden.

---

## TC-12 — Speaker label group: macOS with BlackHole, no device name set

1. macOS + BlackHole installed + no device name in settings; open Settings → General.

**Expected:** "Speaker capture device name" field IS visible (to allow setup). "Input label", "Output label", "Capture speaker by default" hidden until name is saved.

---

## TC-13 — Speaker label group: macOS with BlackHole + device name configured

1. macOS + BlackHole + device name set and saved; open Settings → General.

**Expected:** All four fields visible: device name, input label, output label, capture toggle.

---

## TC-14 — Speaker label group: Windows

1. On Windows, open Settings → General.

**Expected:** "Input label", "Output label", "Capture speaker by default" all visible. No "Speaker capture device name" field.

---

## TC-15 — Default prefix in Replacements tab

1. Fresh install (no `config.json`) or reset config; open Settings → Replacements.

**Expected:** Trigger prefix field shows "float".

---

## TC-16 — Default rules display with prefix

1. Open Settings → Replacements; observe rule list.

**Expected:** Triggers display with prefix prepended, e.g. "float to do", "float dash", "float new line". Matches what the user must say to activate a rule.

---

## TC-17 — Prefix-gated replacement fires

Setup: prefix = "float", rule trigger = "dash" → "-".

1. Dictate "eleven float dash may".

**Expected:** Output "eleven - may".

---

## TC-18 — Unprefixed trigger does NOT fire

Setup: prefix = "float", rule trigger = "dash" → "-".

1. Dictate "eleven dash may".

**Expected:** Output "eleven dash may" (no replacement; prefix is required).

---

## TC-19 — Existing old-format rules work after upgrade (backward compat)

Setup: In `config.json` manually add rule with trigger `"float dash"` → `"-"`; prefix = "float".

1. Dictate "eleven float dash may".

**Expected:** "eleven - may" — trigger already starts with prefix, used as-is, no double-prefix.

---

## TC-20 — Empty prefix disables gate

Setup: Settings → Replacements → clear prefix → Save. Rule trigger = "dash" → "-".

1. Dictate "eleven dash may".

**Expected:** "eleven - may" (no prefix required when field is empty).

---

## TC-21 — Custom prefix

Setup: Set prefix = "scribe"; rule trigger = "dash" → "-".

1. Dictate "eleven scribe dash may".

**Expected:** "eleven - may".

---

## TC-22 — Old prefix no longer matches after prefix change

Setup: Set prefix = "scribe"; rule trigger = "dash" → "-".

1. Dictate "eleven float dash may".

**Expected:** "eleven float dash may" (no match — prefix is "scribe", not "float").
