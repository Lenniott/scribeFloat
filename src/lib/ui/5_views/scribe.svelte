<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { browser } from "$app/environment";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import Accordion from "@patterns/Accordion.svelte";
  import AccordionItem from "@components/nav/AccordionRow.svelte";
  import Button from "@components/controls/Button.svelte";
  import IconButton from "@components/controls/IconButton.svelte";
  import TextField from "@primitives/form/TextField.svelte";
  import Modal from "@primitives/layout/Modal.svelte";
  import RecordingStatusDot from "@primitives/display/StatusDot.svelte";
  import RecordingTimer from "@primitives/display/RecordingTimer.svelte";
  import Waveform from "@components/indicators/Waveform.svelte";
  import EditableTitle from "@components/controls/EditableTitle.svelte";
  import ToggleSwitch from "@components/controls/Toggle.svelte";
  import NoteComposer from "@patterns/NoteComposer.svelte";
  import NotesList from "@patterns/NoteList.svelte";
  import ScrollablePanel from "@primitives/layout/ScrollBody.svelte";
  import PanelFooter from "@primitives/layout/PanelFooter.svelte";
  import { createModelDownloadStore } from "$lib/stores/modelDownload.svelte";
  import Bin from "lucide-svelte/icons/trash-2";
  import CheckCircle from "lucide-svelte/icons/check-circle-2";
  import Cog from "lucide-svelte/icons/settings-2";
  import Clock from "lucide-svelte/icons/clock-3";
  import MicPlus from "lucide-svelte/icons/mic-vocal";
  import type { Note } from "@components/cards/InlineNote.svelte";
  import { appErrorMessage, type PermissionStatus } from '@utils/types';
  import { isWindows } from '@utils/platform';

  type Props = {
    processingStart?: (title: string) => void;
    embedded?: boolean;
    onOpenSettings?: () => void;
    /** Bumped when the shell navigates to Scribe so device lists refresh. */
    visitKey?: number;
    registerLeaveHandler?: (handler: (proceed: () => void) => void) => void;
  };

  let {
    processingStart,
    embedded = false,
    onOpenSettings,
    visitKey = 0,
    registerLeaveHandler,
  }: Props = $props();

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
  let pendingLeave: (() => void) | null = null;

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

  type CaptureState = "idle" | "recording" | "failed" | "saved";
  type CaptureQualityState = "pending" | "recording" | "safe" | "optimal" | "failed";
  type CaptureStatus = {
    clip_id?: string;
    capture_id?: string;
    speech_s: number;
    purity: number;
    state: CaptureQualityState;
  };
  type CaptureStart = {
    capture_id: string;
  };
  type CaptureResult = {
    duration_s: number;
    speech_s: number;
    purity: number;
    accepted: boolean;
  };
  let captureState = $state<CaptureState>("idle");
  let captureQualityState = $state<CaptureQualityState>("pending");
  let captureId = $state("");
  let captureSpeechS = $state(0);
  let capturePurity = $state(0);
  let captureProfileName = $state("Other");
  let captureProfileNames = $state<string[]>([]);
  let captureError = $state("");
  let captureSaving = $state(false);
  let captureResetTimer: ReturnType<typeof setTimeout> | null = null;
  const capturePurityPct = $derived(Math.round(capturePurity * 100));
  const captureProgressPct = $derived(Math.min(100, Math.round((captureSpeechS / 10) * 100)));
  const captureSafeToStop = $derived(captureSpeechS >= 5 && capturePurity >= 0.5);
  const captureStatusText = $derived(
    captureQualityState === "optimal"
      ? "Optimal"
      : captureQualityState === "safe"
        ? "Safe to stop"
        : captureSpeechS > 0
          ? "Keep listening"
          : "Waiting for speech",
  );

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

  /** Re-scan disk when Scribe is shown. */
  async function refreshRecoverySessionsIfVisible() {
    if (embedded) {
      await loadRecoverySessions();
      return;
    }
    const visible = await getCurrentWindow().isVisible().catch(() => false);
    if (!visible) return;
    await loadRecoverySessions();
  }

  function requestLeave(proceed: () => void) {
    if (phase === "recording") {
      pendingLeave = proceed;
      discardConfirmOpen = true;
      return;
    }
    proceed();
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
        void cancelActiveCapture();
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
        void cancelActiveCapture();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        void reloadSpeakerCaptureSettings();
        break;
      case "DONE":
        phase = "idle";
        stopTimer();
        void cancelActiveCapture();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        void reloadSpeakerCaptureSettings();
        break;
      case "NO_MODEL":
        phase = "no_model";
        stopTimer();
        void cancelActiveCapture();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
        break;
      case "ERROR":
        phase = "error";
        errorMessage = p.error ?? "Unknown error";
        stopTimer();
        void cancelActiveCapture();
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
    if (onOpenSettings) {
      onOpenSettings();
      return;
    }
    await invoke("settings_show_window").catch(() => {});
  }

  /** Backend hides the Scribe webview (standalone window only). */
  async function destroyScribeWindow() {
    if (embedded) return;
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
    await cancelActiveCapture();
    stopTimer();
    micLevelRaw = 0;
    speakerLevelRaw = 0;
    processingStart?.(fileName || "Recording");
  }

  /** Stops capture on the backend; throws if the backend was not recording. */
  async function cancel() {
    await cancelActiveCapture();
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
      if (pendingLeave) {
        const go = pendingLeave;
        pendingLeave = null;
        go();
      } else {
        await destroyScribeWindow();
      }
    } catch (e) {
      phase = "error";
      errorMessage = "Failed to discard recording: " + String(e);
      discardConfirmOpen = false;
    } finally {
      discardInProgress = false;
    }
  }

  async function recordAgain() {
    await cancelActiveCapture();
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

  function applyCaptureStatus(status: CaptureStatus) {
    const eventId = status.capture_id ?? status.clip_id;
    if (!eventId || eventId !== captureId) return;
    captureSpeechS = status.speech_s;
    capturePurity = status.purity;
    captureQualityState = status.state;
  }

  function resetCaptureState() {
    if (captureResetTimer) {
      clearTimeout(captureResetTimer);
      captureResetTimer = null;
    }
    captureState = "idle";
    captureQualityState = "pending";
    captureId = "";
    captureSpeechS = 0;
    capturePurity = 0;
    captureError = "";
    captureSaving = false;
  }

  async function cancelActiveCapture() {
    if (!captureId) {
      resetCaptureState();
      return;
    }
    // Don't cancel while stop-and-save is in progress; commit_clip would lose the pending clip.
    if (captureSaving) {
      return;
    }
    const id = captureId;
    resetCaptureState();
    await invoke("session_capture_cancel", { captureId: id }).catch(() => {});
  }

  async function startSpeakerCapture() {
    if (phase !== "recording" || captureState === "recording") return;
    if (captureId) {
      await invoke("session_capture_cancel", { captureId }).catch(() => {});
      captureId = "";
    }
    captureError = "";
    captureSpeechS = 0;
    capturePurity = 0;
    captureQualityState = "pending";
    captureProfileNames = await invoke<string[]>("voiceprint_list_profile_names").catch(() => []);
    captureProfileName = captureProfileNames.at(-1) ?? "Other";
    try {
      const started = await invoke<CaptureStart>("session_capture_start");
      captureId = started.capture_id;
      captureState = "recording";
    } catch (e) {
      captureState = "failed";
      captureError = `Could not start capture: ${appErrorMessage(e)}`;
    }
  }

  async function stopSpeakerCapture() {
    const name = captureProfileName.trim();
    if (!captureId || !name) {
      captureError = "Speaker name is required.";
      return;
    }
    captureSaving = true;
    captureError = "";
    try {
      const result = await invoke<CaptureResult>("session_capture_stop", {
        captureId,
        profileName: name,
      });
      captureSpeechS = result.speech_s;
      capturePurity = result.purity;
      if (!result.accepted) {
        captureState = "failed";
        captureError = "Too noisy or too short. Try again.";
        captureId = "";
        return;
      }
      captureState = "saved";
      captureId = "";
      captureResetTimer = setTimeout(resetCaptureState, 3000);
    } catch (e) {
      captureState = "failed";
      captureError = `Could not save capture: ${appErrorMessage(e)}`;
    } finally {
      captureSaving = false;
    }
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
    const ulSpeakerSaved = await listen("settings://speaker-capture-saved", () => {
      void reloadSpeakerCaptureSettings();
    });
    const ulCaptureStatus = await listen<CaptureStatus>("voiceprint://clip-status", (e) => {
      applyCaptureStatus(e.payload);
    });
    unlisteners = [ul1, ul2, ulSpeaker, ulSpeakerUnavailable, ulSpeakerSaved, ulCaptureStatus];
    if (!embedded) {
      const ulClose = await listen("scribe://native-close-requested", () => {
        void handleNativeCloseRequested();
      });
      unlisteners.push(ulClose);
      unlistenFocus = await getCurrentWindow().onFocusChanged(
        ({ payload: focused }) => {
          if (!focused) return;
          void refreshMicDevices();
          void reloadSpeakerCaptureSettings();
          void refreshRecoverySessionsIfVisible();
        },
      );
    }
    registerLeaveHandler?.(requestLeave);
  });

  $effect(() => {
    if (!embedded) return;
    visitKey;
    void refreshMicDevices();
    void reloadSpeakerCaptureSettings();
    void refreshRecoverySessionsIfVisible();
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
    unlistenFocus?.();
    modelUnlisteners.forEach((u) => u());
    void cancelActiveCapture();
    stopTimer();
  });
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden text-fg">
  <section class="flex {embedded ? 'h-full min-h-0' : 'h-screen'} flex-col overflow-hidden bg-panel">
    <!-- Header -->
    <header
      class="flex min-h-14 shrink-0 items-end justify-between border-b border-b-rim px-5 py-2"
    >
      <div class="min-w-0 flex-1">
        <EditableTitle bind:value={fileName} />
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
      <div class="shrink-0 border-b border-warning bg-warning/15 px-5 py-2 sf-body-md text-fg">
        <p>
          {recoverySessions.length === 1
            ? "An interrupted recording was found."
            : `${recoverySessions.length} interrupted recordings were found.`}
          Open <strong>Upload</strong> in the sidebar and drop the session folder
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
        <div class="shrink-0">
          <Waveform
            micLevel={micLevel}
            speakerLevel={speakerLevel}
            speakerEnabled={speakerEnabledForWaveform}
            size="normal"
          />
        </div>

        <ScrollablePanel class="px-0 py-0">
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
        </ScrollablePanel>

        <!-- Footer -->
        <footer class="flex shrink-0 flex-col gap-2 py-3">
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
                <div class="relative">
                  <IconButton
                    variant="normal"
                    size="normal"
                    icon={captureState === "saved" ? CheckCircle : captureState === "recording" ? Clock : MicPlus}
                    iconExtraClass={captureState === "saved" ? "text-success" : captureState === "recording" ? "animate-pulse" : ""}
                    aria-label="Capture speaker voiceprint"
                    disabled={captureSaving}
                    onclick={() => {
                      if (captureState === "idle" || captureState === "failed") {
                        void startSpeakerCapture();
                      }
                    }}
                  />
                  {#if captureState === "recording" || captureState === "failed"}
                    <div class="absolute bottom-12 left-0 z-20 w-72 rounded-md border border-rim bg-panel p-3 shadow-lg">
                      <div class="mb-3 flex items-center justify-between gap-3">
                        <p class="sf-label-md text-fg">
                          {captureState === "failed" ? "Capture failed" : "Capturing voiceprint"}
                        </p>
                        <span class="sf-label-sm text-fg-dim">{captureStatusText}</span>
                      </div>

                      {#if captureError}
                        <p class="mb-3 rounded-md border border-destructive/40 bg-fill px-2 py-1.5 sf-label-sm text-destructive">
                          {captureError}
                        </p>
                      {/if}

                      <div class="space-y-3">
                        <div>
                          <div class="mb-1 flex justify-between sf-label-sm text-fg-dim">
                            <span>VAD purity</span>
                            <span>{capturePurityPct}%</span>
                          </div>
                          <div class="h-1.5 overflow-hidden rounded-sm bg-fill">
                            <div
                              class="h-full bg-brand transition-[width] duration-200"
                              style={`width:${capturePurityPct}%`}
                            ></div>
                          </div>
                        </div>

                        <div>
                          <div class="mb-1 flex justify-between sf-label-sm text-fg-dim">
                            <span>Speech</span>
                            <span>{captureSpeechS.toFixed(1)}s</span>
                          </div>
                          <div class="h-1.5 overflow-hidden rounded-sm bg-fill">
                            <div
                              class="h-full bg-success transition-[width] duration-200"
                              style={`width:${captureProgressPct}%`}
                            ></div>
                          </div>
                          <div class="mt-1 grid grid-cols-3 sf-label-sm text-fg-muted">
                            <span>0s</span>
                            <span class="text-center">5s safe</span>
                            <span class="text-right">10s optimal</span>
                          </div>
                        </div>

                        <TextField
                          label="Speaker name"
                          bind:value={captureProfileName}
                          placeholder="Other"
                          disabled={captureSaving}
                        />
                        {#if captureProfileNames.length > 0}
                          <div class="flex flex-wrap gap-1">
                            {#each captureProfileNames as name (name)}
                              <button
                                type="button"
                                class="rounded-md border border-rim px-2 py-1 sf-label-sm text-fg-dim hover:bg-fill"
                                onclick={() => (captureProfileName = name)}
                              >
                                {name}
                              </button>
                            {/each}
                          </div>
                        {/if}
                      </div>

                      <div class="mt-3 flex justify-between gap-2">
                        <Button
                          variant="ghost"
                          size="small"
                          disabled={captureSaving}
                          onclick={cancelActiveCapture}
                        >
                          Cancel
                        </Button>
                        {#if captureState === "failed"}
                          <Button
                            variant="normal"
                            size="small"
                            disabled={captureSaving}
                            onclick={startSpeakerCapture}
                          >
                            Retry
                          </Button>
                        {:else}
                          <Button
                            variant="primary"
                            size="small"
                            disabled={captureSaving || !captureSafeToStop}
                            onclick={stopSpeakerCapture}
                          >
                            Stop & Save
                          </Button>
                        {/if}
                      </div>
                    </div>
                  {/if}
                </div>
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
        <p class="sf-section-label mb-2 shrink-0 text-fg-dim">Add notes</p>
        <ScrollablePanel class="px-0 py-0">
          <div class="h-full rounded-md">
            <NotesList {notes} bind:selectedId={selectedNoteId} />
          </div>
        </ScrollablePanel>
        {#if phase === "recording"}
          <div class="shrink-0 pt-3">
            <NoteComposer bind:value={noteDraft} onSubmit={addNote} />
          </div>
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
  onClose={() => {
    discardConfirmOpen = false;
    pendingLeave = null;
  }}
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
