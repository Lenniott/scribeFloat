<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { UpdateCheckResult } from '@utils/types';
  import Button from "@components/controls/Button.svelte";
  import Toast from "@components/indicators/Toast.svelte";
  import { createToast } from '@stores/toast.svelte';
  import SettingsList from "@sections/SettingList.svelte";
  import SettingsRow from "@components/cards/SettingRow.svelte";
  import SettingsSection from "@primitives/form/SettingsSection.svelte";
  import {
    dictateModifierLabel,
    formatHotkeyForDisplay,
    isWindows,
  } from '@utils/platform';


  type UpdateState =
    | "idle"
    | "checking"
    | "up_to_date"
    | "update_available"
    | "error";

  let updateState = $state<UpdateState>("idle");
  let updateResult = $state<UpdateCheckResult | null>(null);
  let openScribeHotkey = $state("CmdOrCtrl+Shift+L");
  let speakerCaptureRequiresDeviceName = $state(false);
  let restartingOnboarding = $state(false);
  const toast = createToast(3000);

  onDestroy(() => toast.dismiss());

  async function restartOnboarding() {
    restartingOnboarding = true;
    await invoke("settings_reset_onboarding").catch(() => {});
    await invoke("settings_show_onboarding_window").catch(() => {});
    restartingOnboarding = false;
  }

  async function checkForUpdates() {
    updateState = "checking";
    updateResult = null;
    try {
      const result = await invoke<UpdateCheckResult>("update_check");
      updateResult = result;
      if (result.update_available) {
        updateState = "update_available";
        toast.show(`Version ${result.latest_version} is available`);
      } else {
        updateState = "up_to_date";
        toast.show("You're on the latest version.", "success");
      }
    } catch (e) {
      const message =
        typeof e === "string" ? e : "Could not reach update server.";
      updateState = "error";
      toast.show(`Could not check for updates: ${message}`, "error");
    }
  }

  async function openDownloadPage() {
    if (updateResult) await openUrl(updateResult.release_url);
  }

  onMount(async () => {
    const [open] = await invoke<[string, string]>("settings_get_hotkeys").catch(
      () => ["", ""],
    );
    openScribeHotkey = open || openScribeHotkey;
    speakerCaptureRequiresDeviceName = await invoke<boolean>(
      "settings_speaker_capture_requires_device_name",
    ).catch(() => false);
  });
</script>

<section class="space-y-8 max-w-2xl">
  <div>
    <h2 class="sf-headline-sm text-fg">Help</h2>
    <p class="mt-1 sf-body-md text-fg-dim">
      How to use ScribeFloat and what every setting does.
    </p>
  </div>

  <SettingsSection title="Setup and updates">
    <SettingsList>
      <SettingsRow
        title="Restart setup wizard"
        description="Re-run the first-time setup to reconfigure permissions and key settings."
      >
        {#snippet control()}
          <Button
            variant="normal"
            onclick={restartOnboarding}
            disabled={restartingOnboarding}
          >
            {restartingOnboarding ? "Opening…" : "Restart Setup Wizard"}
          </Button>
        {/snippet}
      </SettingsRow>

      <SettingsRow
        title="ScribeFloat updates"
        description={updateResult ? `Current version: ${updateResult.current_version}` : undefined}
      >
        {#snippet control()}
          <Button
            variant="normal"
            onclick={checkForUpdates}
            disabled={updateState === "checking"}
          >
            {updateState === "checking" ? "Checking…" : "Check for updates"}
          </Button>
        {/snippet}
        {#if updateState === "update_available" && updateResult}
          <div class="space-y-2 rounded-md border border-fill bg-fill p-3">
            <p class="sf-body-md-strong text-fg">
              Version {updateResult.latest_version} is available
            </p>
            {#if updateResult.release_notes}
              <p class="sf-body-md text-fg-dim">{updateResult.release_notes}</p>
            {/if}
            <Button variant="primary" onclick={openDownloadPage}>
              Open download page
            </Button>
          </div>
        {/if}
      </SettingsRow>
    </SettingsList>
  </SettingsSection>

  <div class="space-y-2">
    <h3 class="sf-section-label text-fg-dim">Scribe</h3>
    <p class="sf-body-md text-fg">
      Scribe records your microphone and transcribes the audio into a
      timestamped Markdown file saved in your save folder.
    </p>
    <ul class="space-y-1 sf-body-md text-fg list-disc pl-5">
      <li>
        Open Scribe from the menu bar icon or press the <strong
          >Open Scribe hotkey</strong
        >
        (shown in General settings —
        <code class="sf-meta-sm bg-fill px-1 rounded"
          >{formatHotkeyForDisplay(openScribeHotkey)}</code
        > by default).
      </li>
      <li>
        Press <strong>Record</strong> to start. Add timestamped notes while recording
        if you like.
      </li>
      <li>
        Press <strong>Stop & Save</strong> — ScribeFloat transcribes the audio
        and saves a <code class="sf-meta-sm bg-fill px-1 rounded">.md</code>
        file in your save folder (same title twice gets
        <code class="sf-meta-sm bg-fill px-1 rounded">_1</code>,
        <code class="sf-meta-sm bg-fill px-1 rounded">_2</code>, … suffixes).
        The recording is deleted once the transcript is confirmed saved unless
        <strong>Keep audio after transcription</strong> is on.
      </li>
      <li>
        Enable <strong>Speaker capture</strong> to also record system audio
        (e.g. for calls and meetings). Mic lines are prefixed
        <code class="sf-meta-sm bg-fill px-1 rounded">in:</code>
        and speaker lines
        <code class="sf-meta-sm bg-fill px-1 rounded">out:</code>
        in the transcript.{#if isWindows}
          On Windows, system audio must be playing for the speaker waveform to
          move — loopback captures what your speakers are outputting.{/if}
      </li>
    </ul>
  </div>

  <div class="space-y-2">
    <h3 class="sf-section-label text-fg-dim">Dictate</h3>
    <p class="sf-body-md text-fg">
      Dictate is a floating hotkey-driven voice input. Audio is streamed to a
      short-lived temp file while you dictate, then deleted after a successful
      transcription (or moved to <code class="sf-meta-sm bg-fill px-1 rounded"
        >dictate_failures/</code
      > in your save folder if transcription fails).
    </p>
    <ul class="space-y-1 sf-body-md text-fg list-disc pl-5">
      <li>
        Tap left <strong>{dictateModifierLabel}</strong>, release — tap left
        <strong>{dictateModifierLabel}</strong>
        again: hold half a second to start push-to-talk and release when done, or
        tap and release quickly to stay in toggle mode and press
        <strong>{dictateModifierLabel}</strong> again after a brief pause to stop.
      </li>
      <li>
        If Accessibility permission is granted, the text is pasted automatically
        via <code class="sf-meta-sm bg-fill px-1 rounded">Cmd/Ctrl+V</code>.
        Otherwise it goes to the clipboard.
      </li>
      <li>
        Enable <strong>Press Enter after dictate</strong> in General settings to
        send an Enter keystroke after the paste — handy for messaging apps.
      </li>
      <li>
        Each successful dictation is appended to <code
          class="sf-meta-sm bg-fill px-1 rounded">dictate.jsonl</code
        > in your save folder.
      </li>
    </ul>
  </div>

  <div class="space-y-2">
    <h3 class="sf-section-label text-fg-dim">Transcribe</h3>
    <p class="sf-body-md text-fg">
      Transcribe converts existing audio files to text. Open it from the menu
      bar icon.
    </p>
    <ul class="space-y-1 sf-body-md text-fg list-disc pl-5">
      <li>
        Drag a <strong>WAV, MP3, M4A, or FLAC</strong> file onto the panel (or use
        the file picker).
      </li>
      <li>Choose an output folder and press <strong>Transcribe</strong>.</li>
      <li>
        If the file is a dual-source Scribe session folder (contains <code
          class="sf-meta-sm bg-fill px-1 rounded">mic.wav</code
        >
        + <code class="sf-meta-sm bg-fill px-1 rounded">session.json</code>),
        the dual-source merge runs automatically.
      </li>
    </ul>
  </div>

  <div class="space-y-2">
    <h3 class="sf-section-label text-fg-dim">Transcription model</h3>
    <p class="sf-body-md text-fg">
      ScribeFloat ships with <strong>OpenAI Whisper</strong> built in, running entirely
      on your device — nothing to download, and it works offline.
    </p>
  </div>

  <div class="space-y-2">
    <h3 class="sf-section-label text-fg-dim">Settings reference</h3>
    <div class="overflow-hidden rounded-md border border-card">
      <table class="w-full">
        <thead class="bg-fill">
          <tr>
            <th class="px-3 py-2 text-left sf-label-sm text-fg-dim">Setting</th>
            <th class="px-3 py-2 text-left sf-label-sm text-fg-dim"
              >What it does</th
            >
          </tr>
        </thead>
        <tbody class="divide-y divide-card">
          <tr>
            <td class="px-3 py-2 sf-body-md-strong text-fg"
              >Default save folder</td
            >
            <td class="px-3 py-2 sf-body-md text-fg-dim"
              >Where transcripts and Dictate history are saved.</td
            >
          </tr>
          <tr>
            <td class="px-3 py-2 sf-body-md-strong text-fg"
              >Open transcripts with</td
            >
            <td class="px-3 py-2 sf-body-md text-fg-dim"
              >App used to open <code class="sf-meta-sm bg-fill px-1 rounded"
                >.md</code
              > files after transcription (Advanced settings). Leave blank to use
              the system default.</td
            >
          </tr>
          <tr>
            <td class="px-3 py-2 sf-body-md-strong text-fg"
              >Open Scribe hotkey</td
            >
            <td class="px-3 py-2 sf-body-md text-fg-dim"
              >Global shortcut to show or bring back the Scribe panel from
              anywhere. Shown in General settings — fixed and not currently
              editable.</td
            >
          </tr>
          <tr>
            <td class="px-3 py-2 sf-body-md-strong text-fg"
              >Capture speaker by default</td
            >
            <td class="px-3 py-2 sf-body-md text-fg-dim"
              >Pre-enables dual-source (mic + speaker) whenever Scribe opens.</td
            >
          </tr>
          <tr>
            <td class="px-3 py-2 sf-body-md-strong text-fg"
              >Press Enter after dictate</td
            >
            <td class="px-3 py-2 sf-body-md text-fg-dim"
              >Sends an Enter keystroke immediately after the dictated text is
              pasted. Useful in messaging and search apps.</td
            >
          </tr>
          {#if speakerCaptureRequiresDeviceName}
            <tr>
              <td class="px-3 py-2 sf-body-md-strong text-fg"
                >Speaker capture device name</td
              >
              <td class="px-3 py-2 sf-body-md text-fg-dim"
                >macOS only — exact BlackHole or Multi-Output device name
                (usually <code class="sf-meta-sm bg-fill px-1 rounded"
                  >BlackHole 2ch</code
                >). Windows uses your default output device automatically.</td
              >
            </tr>
          {/if}
        </tbody>
      </table>
      <div class="mt-4 text-fg-dim">
        {#if isWindows}
          <p class="mb-2 sf-body-md">
            <strong class="text-fg">Windows install:</strong> the installer
            places ScribeFloat in
            <code class="sf-meta-sm bg-fill px-1 rounded"
              >C:\Program Files\ScribeFloat</code
            >. Transcripts default to
            <code class="sf-meta-sm bg-fill px-1 rounded"
              >Documents\transcripts_scribefloat</code
            >.
          </p>
          <p class="mb-2 sf-body-md">
            <strong class="text-fg">Uninstall:</strong> Settings → Apps → Installed
            apps → ScribeFloat → Uninstall. Config, models, and transcripts are kept
            unless you delete them manually.
          </p>
        {/if}
        <p class="sf-body-md">
          Enjoying scribeFloat? <a href="https://buymeacoffee.com/benjamiz"
            >Send a tip</a
          >
        </p>
      </div>
    </div>
  </div>
</section>

<Toast message={toast.message} state={toast.state} position="bottom-center" />
