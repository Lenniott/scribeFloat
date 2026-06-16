<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { browser } from "$app/environment";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import Accordion from "@components/accordion/Accordion.svelte";
  import AccordionItem from "@components/accordion/AccordionItem.svelte";
  import Button from "@components/Button.svelte";
  import IconButton from "@components/IconButton.svelte";
  import Modal from "@components/Modal.svelte";
  import RecordingStatusDot from "@components/audio/RecordingStatusDot.svelte";
  import RecordingTimer from "@components/audio/RecordingTimer.svelte";
  import AudioWaveFormVisualizer from "@lib/components/audio/AudioWaveFormVisualizer.svelte";
  import EditableTitleField from "@components/form/EditableTitleField.svelte";
  import ToggleSwitch from "@components/form/ToggleSwitch.svelte";
  import NoteComposer from "@components/notes/NoteComposer.svelte";
  import NotesList from "@components/notes/NotesList.svelte";
  import { createModelDownloadStore } from "$lib/stores/modelDownload.svelte";
  import Bin from "lucide-svelte/icons/trash-2";
  import Cog from "lucide-svelte/icons/settings-2";
  import type { Note } from "@components/notes/NoteCard.svelte";
  import type { PermissionStatus } from "$lib/types";
  import { isWindows } from "$lib/platform";

  type Props = {
    processingStart?: (title: string) => void;
  };

  let { processingStart }: Props = $props();

  // ── State machine ─────────────────────────────────────────────────────────
  type Phase = "idle" | "recording" | "no_model" | "error";
  let phase = $state<Phase>("idle");
  let errorMessage = $state("");

  // ── Model download ─────────────────────────────────────────────────────────
  const modelStore = createModelDownloadStore();
  let modelUnlisteners: (() => void)[] = [];
  let discardConfirmOpen = $state(false);
  let discardInProgress = $state(false);
  let startInProgress = false;

  const downloadedModelOptions = $derived(
    modelStore.models
      .filter((m) => m.downloaded)
      .map((m) => ({ value: m.id, label: m.label })),
  );

  // ── Recording ─────────────────────────────────────────────────────────────
  let elapsedSeconds = $state(0);
  let timerInterval: ReturnType<typeof setInterval> | null = null;

  function startTimer() {
    stopTimer();
    const start = Date.now();
    elapsedSeconds = 0;
    timerInterval = setInterval(() => {
      elapsedSeconds = Math.floor((Date.now() - start) / 1000);
    }, 1000);
  }

  function stopTimer() {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  // ── Session metadata ──────────────────────────────────────────────────────
  function defaultTitle() {
    const now = new Date();
    const pad = (n: number) => n.toString().padStart(2, "0");
    return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}-${pad(now.getMinutes())}`;
  }

  let fileName = $state(defaultTitle());
  let selectedMic = $state("");
  let selectedSpeakerSource = $state("");
  let selectedModelId = $state("");
  let noteDraft = $state("");
  let notes = $state<Note[]>([]);
  let selectedNoteId = $state<string | null>(null);
  let includeTimestamps = $state(true);
  /** Raw RMS from backend; only shown while `phase === 'recording'`. */
  let micLevelRaw = $state(0);
  let speakerLevelRaw = $state(0);
  const micLevel = $derived(phase === "recording" ? micLevelRaw : 0);
  const speakerLevel = $derived(phase === "recording" ? speakerLevelRaw : 0);
  let captureSpeaker = $state(false);
  let speakerWarning = $state("");
  let speakerCaptureRequiresDeviceName = $state(false);
  let blackholeDetected = $state(false);
  /** Persisted device name only — updated on load and after Settings save, not while typing. */
  let savedSpeakerDeviceName = $state("");
  const speakerCaptureAvailable = $derived(
    !speakerCaptureRequiresDeviceName ||
      (blackholeDetected && savedSpeakerDeviceName.trim().length > 0),
  );
  const speakerEnabledForWaveform = $derived(
    speakerCaptureAvailable && captureSpeaker,
  );
  let saveFolder = $state("");
  let micOptions = $state([{ value: "", label: "System Default" }]);
  type RecoverySessionInfo = {
    session_dir: string;
    mic_wav: string;
    state: string;
  };
  let recoverySessions = $state<RecoverySessionInfo[]>([]);
  let dismissedRecoveryDirs = $state<string[]>([]);

  async function loadRecoverySessions() {
    const sessions = await invoke<RecoverySessionInfo[]>(
      "scribe_list_recovery_sessions",
    ).catch(() => []);
    const dismissed = new Set(dismissedRecoveryDirs);
    recoverySessions = sessions.filter((s) => !dismissed.has(s.session_dir));
  }

  function dismissRecoveryBanner() {
    dismissedRecoveryDirs = [
      ...dismissedRecoveryDirs,
      ...recoverySessions.map((s) => s.session_dir),
    ];
    recoverySessions = [];
  }

  /** Enumerate input devices if mic permission is currently granted.
   * Called on mount and on focus — needed because the window is prewarmed before
   * permission may have been granted, so the first run can leave micOptions empty. */
  async function refreshMicDevices() {
    const perms = await invoke<PermissionStatus[]>(
      "settings_permissions_status",
    ).catch(() => [] as PermissionStatus[]);
    if (!(perms.find((p) => p.kind === "microphone")?.granted ?? false)) return;
    const devices = await invoke<string[]>("scribe_list_input_devices").catch(
      () => [],
    );
    micOptions = [
      { value: "", label: "System Default" },
      ...devices.map((d) => ({ value: d, label: d })),
    ];
  }

  /** Re-scan disk when Scribe is shown — prewarm runs `loadRecoverySessions` once at startup and would otherwise stay stale. */
  async function refreshRecoverySessionsIfVisible() {
    const visible = await getCurrentWindow().isVisible().catch(() => false);
    if (!visible) return;
    await loadRecoverySessions();
  }

  // ── Backend events ────────────────────────────────────────────────────────
  type ScribePayload = {
    state: string;
    progress?: number;
    transcript_path?: string;
    wav_path?: string;
    error?: string;
  };

  type BackendNote = {
    id: string;
    text: string;
    recorded_at_ms: number;
  };

  function handleScribeEvent(p: ScribePayload) {
    switch (p.state) {
      case "IDLE":
        phase = "idle";
        stopTimer();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        void reloadSpeakerCaptureSettings();
        break;
      case "RECORDING":
        phase = "recording";
        startTimer();
        break;
      case "TRANSCRIBING":
        phase = "idle";
        stopTimer();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        void reloadSpeakerCaptureSettings();
        break;
      case "DONE":
        phase = "idle";
        stopTimer();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        void reloadSpeakerCaptureSettings();
        break;
      case "NO_MODEL":
        phase = "no_model";
        stopTimer();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        break;
      case "ERROR":
        phase = "error";
        errorMessage = p.error ?? "Unknown error";
        stopTimer();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        break;
    }
  }

  async function reloadSpeakerCaptureSettings() {
    const [preferredInputDevice, preferredSpeakerDevice] = await invoke<
      [string | null, string | null]
    >("settings_get_preferred_audio_devices").catch(() => [null, null]);
    selectedMic = preferredInputDevice ?? selectedMic;
    savedSpeakerDeviceName = preferredSpeakerDevice ?? "";
    selectedSpeakerSource = savedSpeakerDeviceName;
    speakerCaptureRequiresDeviceName = await invoke<boolean>(
      "settings_speaker_capture_requires_device_name",
    ).catch(() => false);
    blackholeDetected = await invoke<boolean>("settings_blackhole_detected").catch(
      () => false,
    );
    const available =
      !speakerCaptureRequiresDeviceName ||
      (blackholeDetected && savedSpeakerDeviceName.trim().length > 0);
    if (!available) {
      captureSpeaker = false;
    } else {
      captureSpeaker = await invoke<boolean>(
        "settings_get_scribe_capture_speaker",
      ).catch(() => captureSpeaker);
    }
  }

  // ── Actions ───────────────────────────────────────────────────────────────
  async function startRecording() {
    if (startInProgress || phase === "recording") return;
    startInProgress = true;
    try {
      await reloadSpeakerCaptureSettings();

      const perms = await invoke<PermissionStatus[]>(
        "settings_permissions_status",
      ).catch(() => []);
      const mic = perms.find((p) => p.kind === "microphone");
      if (mic && !mic.granted) {
        if (isWindows) {
          await invoke("settings_permissions_request", { kind: "microphone" }).catch(
            () => {},
          );
        } else {
          phase = "error";
          errorMessage =
            "Microphone access is required. Grant it under Settings → Permissions, then try again.";
          return;
        }
      }

      await invoke("scribe_start", {
        preferredMic: selectedMic || null,
        preferredSpeaker: selectedSpeakerSource || null,
        captureSpeaker: speakerCaptureAvailable && captureSpeaker,
      });
      phase = "recording";
      startTimer();
    } catch (e) {
      phase = "error";
      errorMessage = String(e);
    } finally {
      startInProgress = false;
    }
  }

  async function openSettingsWindow() {
    await invoke("settings_show_window").catch(() => {});
  }

  /** Backend hides the Scribe webview (does not destroy it) so the tray app keeps running. */
  async function destroyScribeWindow() {
    if (!browser) return;
    await invoke("scribe_cancel").catch(() => {});
    await invoke("scribe_abort_transcription").catch(() => {});
    await invoke("scribe_destroy_window").catch(() => {});
  }

  async function handleNativeCloseRequested() {
    if (!browser) return;

    if (phase === "recording") {
      discardConfirmOpen = true;
      return;
    }
    await destroyScribeWindow();
  }

  async function stopAndSave() {
    stopTimer();
    micLevelRaw = 0;
    speakerLevelRaw = 0;
    processingStart?.(fileName || "Recording");
  }

  /** Stops capture on the backend; throws if the backend was not recording. */
  async function cancel() {
    stopTimer();
    notes = [];
    elapsedSeconds = 0;
    micLevelRaw = 0;
    speakerLevelRaw = 0;
    await invoke("scribe_cancel");
    phase = "idle";
  }

  async function discardRecording() {
    discardInProgress = true;
    try {
      await cancel();
      discardConfirmOpen = false;
      await destroyScribeWindow();
    } catch (e) {
      phase = "error";
      errorMessage = "Failed to discard recording: " + String(e);
      discardConfirmOpen = false;
    } finally {
      discardInProgress = false;
    }
  }

  async function recordAgain() {
    notes = [];
    elapsedSeconds = 0;
    errorMessage = "";
    micLevelRaw = 0;
    speakerLevelRaw = 0;
    await startRecording();
  }

  async function addNote() {
    const text = noteDraft.trim();
    if (!text) return;
    const draft = noteDraft;
    noteDraft = "";
    const created = await invoke<BackendNote>("scribe_add_note", {
      text: draft,
    }).catch(() => {
      noteDraft = draft; // restore so the user can retry
      return null;
    });
    if (!created) return;
    notes = [
      ...notes,
      {
        id: created.id,
        text: created.text,
        recordedAtMs: created.recorded_at_ms,
      },
    ];
  }

  // ── Lifecycle ─────────────────────────────────────────────────────────────
  let unlisteners: UnlistenFn[] = [];
  let unlistenFocus: (() => void) | undefined;

  onMount(async () => {
    includeTimestamps = await invoke<boolean>(
      "scribe_get_include_timestamps",
    ).catch(() => true);
    saveFolder = await invoke<string>("settings_get_output_path").catch(
      () => "",
    );
    modelUnlisteners = await modelStore.subscribe();
    await modelStore.refresh();

    // Only enumerate input devices if mic permission is already granted.
    // On macOS 14+ calling input_devices() triggers the permission dialog when
    // status is NotDetermined — avoid that on prewarm / before the user asks.
    // refreshMicDevices is also called on focus so a prewarm-before-grant never
    // leaves the list permanently empty.
    await refreshMicDevices();
    const [preferredInputDevice, preferredSpeakerDevice] = await invoke<
      [string | null, string | null]
    >("settings_get_preferred_audio_devices").catch(() => [null, null]);
    selectedMic = preferredInputDevice ?? "";
    selectedSpeakerSource = preferredSpeakerDevice ?? "";
    await reloadSpeakerCaptureSettings();
    await loadRecoverySessions();

    // Sync model selector with the currently selected model
    const sel = modelStore.models.find((m) => m.selected && m.downloaded);
    if (sel) selectedModelId = sel.id;

    const ul1 = await listen<ScribePayload>("scribe://state-changed", (e) =>
      handleScribeEvent(e.payload),
    );
    const ul2 = await listen<number>("scribe://audio-level", (e) => {
      micLevelRaw = e.payload ?? 0;
    });
    const ulSpeaker = await listen<number>("scribe://speaker-level", (e) => {
      speakerLevelRaw = e.payload ?? 0;
    });
    const ulSpeakerUnavailable = await listen<{
      reason?: string;
      requestedSpeakerDevice?: string;
    }>("scribe://speaker-capture-unavailable", (e) => {
      captureSpeaker = false;
      speakerLevelRaw = 0;
      const device = e.payload?.requestedSpeakerDevice;
      speakerWarning = device
        ? `Speaker capture unavailable for "${device}". Recording mic only.`
        : "Speaker capture unavailable. Recording mic only.";
      void invoke("settings_set_scribe_capture_speaker", {
        enabled: false,
      }).catch(() => {});
    });
    const ul3 = await listen("scribe://native-close-requested", () => {
      void handleNativeCloseRequested();
    });
    const ulSpeakerSaved = await listen("settings://speaker-capture-saved", () => {
      void reloadSpeakerCaptureSettings();
    });
    unlisteners = [ul1, ul2, ulSpeaker, ulSpeakerUnavailable, ul3, ulSpeakerSaved];
    unlistenFocus = await getCurrentWindow().onFocusChanged(
      ({ payload: focused }) => {
        if (!focused) return;
        void refreshMicDevices();
        void reloadSpeakerCaptureSettings();
        void refreshRecoverySessionsIfVisible();
      },
    );
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    unlistenFocus?.();
    modelUnlisteners.forEach((u) => u());
    stopTimer();
  });
</script>

<div class="mx-auto flex flex-col gap-4 text-fg">
  <section class="flex h-screen flex-col overflow-hidden bg-panel">
    <!-- Header -->
    <header
      class="flex min-h-14 items-end justify-between border-b border-b-rim px-5 py-2"
    >
      <div class="min-w-0 flex-1">
        <EditableTitleField bind:value={fileName} />
      </div>
      <div class="ml-4 flex items-center gap-2">
        <IconButton
          variant="normal"
          size="small"
          icon={Cog}
          aria-label="Open settings window"
          onclick={openSettingsWindow}
        />
        {#if modelStore.activeDownloadModelId}
          <span class="sf-meta-sm text-fg-dim">
            Model {Math.round(
              (modelStore.progressByModel[modelStore.activeDownloadModelId] ??
                0) * 100,
            )}%
          </span>
        {/if}
      </div>
    </header>

    {#if recoverySessions.length > 0 && phase === "idle"}
      <div class="border-b border-warning bg-warning/15 px-5 py-2 sf-body-md text-fg">
        <p>
          {recoverySessions.length === 1
            ? "An interrupted recording was found."
            : `${recoverySessions.length} interrupted recordings were found.`}
          Open <strong>Transcribe</strong> from the menu bar and drop the session folder
          (contains <code class="rounded bg-fill px-1 sf-label-sm">mic.wav</code>) to recover it.
        </p>
        {#if saveFolder}
          <p class="mt-1 truncate sf-body-md text-fg-dim" title={saveFolder}>
            Scanned folder: {saveFolder}
          </p>
        {/if}
        <Button variant="ghost" size="small" class="mt-1" onclick={dismissRecoveryBanner}>
          Dismiss
        </Button>
      </div>
    {/if}

    <!-- Body -->
    <div class="grid min-h-0 flex-1 grid-cols-[0.45fr_0.99fr] items-stretch">
      <!-- Left: visualizer + settings -->
      <div class="flex min-h-0 flex-col px-4 py-3">
        <AudioWaveFormVisualizer
          micLevel={micLevel}
          speakerLevel={speakerLevel}
          speakerEnabled={speakerEnabledForWaveform}
          size="normal"
        />

        <div class="min-h-0 flex-1 overflow-y-auto">
          <Accordion>
            <AccordionItem id="basic" title="Basic">
              <div class="space-y-4">
                <div class="flex flex-col gap-1.5 text-left">
                  <label class="sf-field-label" for="mic-select">Selected mic</label>
                  <select
                    id="mic-select"
                    bind:value={selectedMic}
                    onchange={async () => {
                      await invoke("settings_set_preferred_audio_devices", {
                        preferredInputDevice: selectedMic || null,
                        preferredSpeakerDevice: selectedSpeakerSource || null,
                      }).catch(() => {});
                      if (phase === "recording") {
                        stopTimer();
                        notes = [];
                        elapsedSeconds = 0;
                        micLevelRaw = 0;
                        speakerLevelRaw = 0;
                        try {
                          await invoke("scribe_cancel");
                        } catch (_) {}
                        phase = "idle";
                        await startRecording();
                      }
                    }}
                    class="h-8 rounded-md border-0 border-b border-transparent bg-panel py-2 pr-8 pl-2 sf-body-md text-fg"
                  >
                    {#each micOptions as opt (opt.value)}
                      <option value={opt.value}>{opt.label}</option>
                    {/each}
                  </select>
                </div>
                {#if downloadedModelOptions.length > 0}
                  <div class="flex flex-col gap-1.5 text-left">
                    <label class="sf-field-label" for="model-select">Model</label>
                    <select
                      id="model-select"
                      value={selectedModelId}
                      onchange={async (e) => {
                        const id = (e.currentTarget as HTMLSelectElement).value;
                        selectedModelId = id;
                        await modelStore.select(id);
                      }}
                      class="h-8 rounded-md border-0 border-b border-transparent bg-panel py-2 pr-8 pl-2 sf-body-md text-fg"
                    >
                      {#each downloadedModelOptions as opt (opt.value)}
                        <option value={opt.value}>{opt.label}</option>
                      {/each}
                    </select>
                  </div>
                {/if}
                {#if speakerCaptureAvailable}
                <ToggleSwitch
                  label="Capture speaker"
                  labelFirst
                  class="w-full justify-between gap-3"
                  bind:checked={captureSpeaker}
                  onchange={async (next) => {
                      if (phase === "recording") {
                        // Session-only: does not change the persistent default.
                        // The toggle resets to the saved default when the session ends.
                        try {
                          await invoke("scribe_toggle_speaker_capture", { enabled: next });
                        } catch (_) {
                          captureSpeaker = !next;
                        }
                      } else {
                        // Idle: update the persistent default for future sessions.
                        await invoke("settings_set_scribe_capture_speaker", {
                          enabled: next,
                        }).catch(() => {
                          captureSpeaker = !next;
                        });
                      }
                    }}
                  />
                {/if}
                <ToggleSwitch
                  label="Transcript timestamps"
                  labelFirst
                  class="w-full justify-between gap-3"
                  checked={includeTimestamps}
                  onchange={async (next) => {
                      const prev = includeTimestamps;
                      includeTimestamps = next;
                      await invoke("scribe_set_include_timestamps", {
                        enabled: next,
                      }).catch(() => {
                        includeTimestamps = prev;
                      });
                    }}
                  />
              </div>
            </AccordionItem>
          </Accordion>
        </div>

        <!-- Footer -->
        <footer class="flex flex-col gap-2 py-3">
          {#if speakerWarning}
            <p class="sf-label-sm text-fg-dim">{speakerWarning}</p>
          {/if}
          {#if saveFolder}
            <p class="truncate sf-label-sm text-fg-muted" title={saveFolder}>
              Saving to {saveFolder}
            </p>
          {/if}
          <div class="flex items-center gap-3">
            {#if phase === "idle"}
              <Button variant="primary" onclick={startRecording}
                >Start Recording</Button
              >
            {:else if phase === "recording"}
              <div class="flex items-center gap-2 w-full">
                <Button variant="primary" onclick={stopAndSave}
                  >Stop and Save</Button
                >
                <IconButton
                  variant="destructive"
                  size="normal"
                  icon={Bin}
                  aria-label="Discard recording"
                  onclick={() => (discardConfirmOpen = true)}
                />
                <div class="ml-auto flex items-center gap-2">
                  <RecordingStatusDot status="recording" />
                  <RecordingTimer class="text-md" {elapsedSeconds} />
                </div>
              </div>
            {:else if phase === "no_model"}
              <div class="flex flex-col gap-2">
                <p class="sf-body-md text-fg-dim">
                  No transcription model selected. Install and select one in
                  Settings → Models.
                </p>
                <Button variant="normal" onclick={openSettingsWindow}
                  >Open Settings</Button
                >
              </div>
            {:else if phase === "error"}
              <div class="flex flex-col gap-2">
                <p class="sf-body-md text-destructive">{errorMessage}</p>
                <div class="flex flex-wrap gap-2">
                  {#if errorMessage.includes("Microphone")}
                    <Button variant="normal" onclick={openSettingsWindow}
                      >Open Settings</Button
                    >
                  {/if}
                  <Button variant="normal" onclick={recordAgain}
                    >Try Again</Button
                  >
                </div>
              </div>
            {/if}
          </div>
        </footer>
      </div>

      <!-- Right: notes -->
      <div class="flex min-h-0 flex-col border-l border-l-rim bg-panel p-3">
        <p class="sf-section-label mb-2 text-fg-dim">Add notes</p>
        <div class="min-h-0 flex-1 overflow-y-auto">
          <div class="h-full rounded-md">
            <NotesList {notes} bind:selectedId={selectedNoteId} />
          </div>
        </div>
        {#if phase === "recording"}
          <NoteComposer bind:value={noteDraft} onSubmit={addNote} />
        {/if}
      </div>
    </div>
  </section>
</div>

<Modal
  open={discardConfirmOpen}
  title="Discard recording?"
  description="Are you sure you want to discard this recording? This cannot be undone."
  maxWidthClass="max-w-md"
  closeDisabled={discardInProgress}
  onClose={() => (discardConfirmOpen = false)}
>
  {#snippet footer()}
    <div class="flex gap-2">
      <Button
        variant="normal"
        disabled={discardInProgress}
        onclick={() => (discardConfirmOpen = false)}
      >
        Cancel
      </Button>
      <Button
        variant="destructive"
        disabled={discardInProgress}
        onclick={discardRecording}
      >
        Discard
      </Button>
    </div>
  {/snippet}
</Modal>
