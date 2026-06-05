# Manual Test Plan

## Scope

Tests covering the onboarding workflow and all settings/features it touches: permissions, model installation, speaker capture, Scribe setup, Dictate setup, History/Markdown output, and the "Restart Setup Wizard" in Help.

---

## OB-01: First Run — Onboarding Window Opens

**Precondition:** Delete `config.json` from the app data directory (or a fresh install).

**Steps:**
1. Launch ScribeFloat.

**Expected:**
- Onboarding window (680 × 560, centered, non-resizable) opens.
- Settings window does NOT open.
- Step 1 (Welcome) is shown with "Get started" primary button and "Skip to Settings" ghost link.

---

## OB-02: Skip to Settings

**Precondition:** OB-01 complete.

**Steps:**
1. On Welcome step, click "Skip to Settings".

**Expected:**
- Settings window opens.
- Onboarding window closes.
- `onboarding_complete` is `true` in config (check: Settings → Help → "Restart Setup Wizard" button is visible).
- Re-launching the app does NOT show onboarding again.

---

## OB-03: Step Navigation — Back and Forward

**Steps:**
1. Complete step 1 (click "Get started").
2. On step 2, click "Continue" (skip permission granting if needed).
3. Click "Back" on step 3.

**Expected:**
- Correct step number shown.
- Progress dots update (current dot is larger / filled).
- Answers from completed steps are preserved when navigating back and forward.

---

## OB-04: Permissions — Microphone Required Gate

**Steps:**
1. Reach step 2 (Permissions).
2. Do NOT grant microphone permission.

**Expected:**
- "Continue" button is disabled.
- Warning copy is shown: "Microphone access is required to continue."

**Then:**
3. Grant microphone permission.

**Expected:**
- "Continue" becomes enabled.
- Microphone row shows CircleCheckBig icon and "Granted" text.

---

## OB-05: Permissions — Accessibility Optional

**Steps:**
1. Reach step 2 (Permissions).
2. Grant microphone but NOT accessibility.
3. Click "Continue".

**Expected:**
- Navigation proceeds to step 3 without blocking.
- Accessibility row shows the "Grant" button still available.

---

## OB-06: Questions — Speaker Capture Persisted

**Steps:**
1. Reach step 3 (Quick Setup).
2. Select "Mic + computer audio" for the speaker capture question.
3. Click "Continue".
4. After completing onboarding, open Settings → General.

**Expected:**
- "Capture speaker by default" toggle is ON.

**Repeat with "Microphone only":**
- Toggle is OFF.

---

## OB-07: Questions — Accuracy vs Speed Affects Model Recommendation

**Steps:**
1. Reach step 3, select "Best accuracy".
2. Click "Continue" to reach step 4 (Model).

**Expected:**
- Recommended model shown is "Small" (~460 MB).

**Repeat with "Fastest speed":**
- Recommended model shown is "Base" (~145 MB).

---

## OB-08: Model — Download with Progress

**Precondition:** No models installed.

**Steps:**
1. Reach step 4 (Model).
2. Click "Install model".

**Expected:**
- StackProgressBar appears with animated fill.
- "Install model" button is hidden during download.
- On completion: model row shows CircleCheckBig and "Installed". "Continue" button appears.

---

## OB-09: Model — Already Installed Fast-Path

**Precondition:** A model is already installed.

**Steps:**
1. Reach step 4 (Model).

**Expected:**
- CircleCheckBig shown immediately.
- "Installed" label visible.
- "Continue" button is active; no install prompt.

---

## OB-10: Model — Skip for Now

**Steps:**
1. Reach step 4 with no model installed.
2. Click "Skip for now".

**Expected:**
- Proceeds to step 5.
- `selectedModelId` in answers is `null`.
- Complete step (step 8) shows warning chip: "No model installed — download one in Settings → Models".

---

## OB-11: Scribe Setup — Speaker Capture Toggle (macOS, BlackHole absent)

**Platform:** macOS, BlackHole NOT installed.

**Steps:**
1. Reach step 5 (Scribe Setup).
2. Ensure speaker capture is toggled ON.

**Expected:**
- Yellow warning box: "BlackHole not detected".
- Instructions to install BlackHole 2ch with explanation.

---

## OB-12: Scribe Setup — Speaker Capture Toggle (macOS, BlackHole present)

**Platform:** macOS, BlackHole installed.

**Steps:**
1. Reach step 5.
2. Toggle speaker capture ON.

**Expected:**
- Green text: "BlackHole detected — speaker capture ready."

---

## OB-13: Scribe Setup — Speaker Capture Toggle (Windows)

**Platform:** Windows.

**Steps:**
1. Reach step 5.
2. Toggle speaker capture ON.

**Expected:**
- Copy reads: "Windows captures system audio automatically — no setup needed."
- No BlackHole mention.

---

## OB-14: Dictate Setup — Hotkey Change

**Steps:**
1. Reach step 6 (Dictate Setup).
2. Click "Capture" in the hotkey field.
3. Press a new key combination.
4. Click "Save hotkey".

**Expected:**
- New hotkey shown in the field.
- After completing onboarding, Settings → General shows the same hotkey.

---

## OB-15: Dictate Setup — Toggles Persist

**Steps:**
1. Reach step 6.
2. Turn auto-paste OFF.
3. Turn auto-enter ON.
4. Complete onboarding.
5. Open Settings → General.

**Expected:**
- "Auto-paste" is OFF.
- "Press Enter after dictate" is ON.

---

## OB-16: History Step — Markdown Toggle

**Steps:**
1. Reach step 7 (History & Output).
2. Toggle "Save as Markdown files" ON.
3. Complete onboarding.

**Expected:**
- In Settings → General, "Save transcripts as Markdown" is ON.

**Repeat with toggle OFF:**
- Setting is OFF.

---

## OB-17: History Step — Dictate Markdown Note

**Steps:**
1. Reach step 7.
2. Turn "Save as Markdown files" ON.

**Expected:**
- Note appears: "Dictate sessions are always stored in History but are never written as Markdown files."

---

## OB-18: Complete Step — Summary Chips Accuracy

**Steps:**
1. Complete all steps with:
   - Microphone granted
   - Small model installed
   - Speaker capture ON
   - Markdown export ON
2. Reach step 8.

**Expected:**
- Four chips shown with CircleCheckBig icons:
  - "Microphone permission granted"
  - "Small model installed" (or similar label)
  - "Speaker capture on"
  - "Markdown export on"

---

## OB-19: Complete — Finish Closes Window

**Steps:**
1. Reach step 8.
2. Click "Start using ScribeFloat".

**Expected:**
- Onboarding window closes.
- No Settings window opens.
- `onboarding_complete` is `true` in config.

---

## OB-20: Return User — No Onboarding on Re-launch

**Precondition:** OB-19 complete.

**Steps:**
1. Quit and relaunch ScribeFloat.

**Expected:**
- Onboarding window does NOT appear.
- Only the tray icon is added.

---

## OB-21: Restart Setup Wizard from Help

**Steps:**
1. Open Settings → Help.
2. Click "Restart Setup Wizard".

**Expected:**
- Onboarding window opens.
- `onboarding_complete` is `false` in config.
- The wizard starts at step 1 (Welcome).

---

## OB-22: Restart — No Duplicate Windows

**Steps:**
1. Open Settings → Help.
2. Click "Restart Setup Wizard" twice quickly.

**Expected:**
- Only one onboarding window opens (second click focuses the existing one).

---

## OB-23: Onboarding While Scribe is Open

**Steps:**
1. Open Scribe from tray.
2. Open onboarding via Help → Restart.

**Expected:**
- Both windows coexist without conflict.
- Scribe stays in its current state.

---

## OB-24: Config File Integrity After Onboarding

**Steps:**
1. Complete onboarding with specific choices.
2. Inspect `config.json` in the app data directory.

**Expected:**
- `onboarding_complete: true`
- `scribe_capture_speaker` matches step 3 choice
- `save_transcripts_as_markdown` matches step 7 choice
- `selected_model_id` or `scribe_model_path` reflects installed model
- `dictate_auto_paste` and `dictate_auto_enter` match step 6 choices

---

## OB-25: Offline / No Network — Model Download Fails Gracefully

**Precondition:** Disconnect from network.

**Steps:**
1. Reach step 4 (Model).
2. Click "Install model".

**Expected:**
- Error message shown: "Install failed. Check your connection and try again."
- "Install model" button reappears.
- "Skip for now" still available.
