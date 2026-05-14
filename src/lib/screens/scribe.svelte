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

  type Props = {
    processingStart?: (title: string) => void;
    /** One-shot: parent sets true when user opens Scribe; cleared after a successful `scribe_start`. */
    autoStart?: boolean;
  };

  let { processingStart, autoStart = $bindable(false) }: Props = $props();

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
  let saveFolder = $state("");
  let micOptions = $state([{ value: "", label: "System Default" }]);

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
        break;
      case "DONE":
        phase = "idle";
        stopTimer();
        micLevelRaw = 0;
        speakerLevelRaw = 0;
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

  // ── Actions ───────────────────────────────────────────────────────────────
  async function startRecording() {
    if (startInProgress || phase === "recording") return;
    startInProgress = true;
    try {
      const perms = await invoke<PermissionStatus[]>(
        "settings_permissions_status",
      ).catch(() => []);
      const mic = perms.find((p) => p.kind === "microphone");
      if (mic && !mic.granted) {
        phase = "error";
        errorMessage =
          "Microphone access is required. Grant it under Settings → Permissions, then try again.";
        return;
      }

      await invoke("scribe_start", {
        preferredMic: selectedMic || null,
        preferredSpeaker: selectedSpeakerSource || null,
        captureSpeaker,
      });
      phase = "recording";
      autoStart = false;
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

  async function maybeAutoStartRecording() {
    if (!autoStart || discardConfirmOpen || discardInProgress) {
      return;
    }
    const visible = await getCurrentWindow().isVisible().catch(() => true);
    if (!visible) return;
    if (phase === "idle") {
      await startRecording();
    }
  }

  /** Arm auto-start when parent sets `autoStart` (tray / hotkey open); do not rely on default-true at webview load. */
  $effect(() => {
    if (!browser) return;
    if (
      !autoStart ||
      phase !== "idle" ||
      discardConfirmOpen ||
      discardInProgress
    ) {
      return;
    }
    void maybeAutoStartRecording();
  });

  /** Backend hides the Scribe webview (does not destroy it) so the tray app keeps running. */
  async function destroyScribeWindow() {
    if (!browser) return;
    autoStart = false;
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
    autoStart = false;
    try {
      await cancel();
      discardConfirmOpen = false;
      // Keep discardInProgress until the window is hidden; closing the modal + focus can fire
      // maybeAutoStartRecording() — isVisible/autoStart guards plus Rust cancel-in-hide cover that.
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
    const permsOnMount = await invoke<PermissionStatus[]>(
      "settings_permissions_status",
    ).catch(() => [] as PermissionStatus[]);
    const micGrantedOnMount =
      permsOnMount.find((p) => p.kind === "microphone")?.granted ?? false;
    const devices = micGrantedOnMount
      ? await invoke<string[]>("scribe_list_input_devices").catch(() => [])
      : [];
    micOptions = [
      { value: "", label: "System Default" },
      ...devices.map((d) => ({ value: d, label: d })),
    ];
    const [preferredInputDevice, preferredSpeakerDevice] = await invoke<
      [string | null, string | null]
    >("settings_get_preferred_audio_devices").catch(() => [null, null]);
    selectedMic = preferredInputDevice ?? "";
    selectedSpeakerSource = preferredSpeakerDevice ?? "";
    captureSpeaker = await invoke<boolean>(
      "settings_get_scribe_capture_speaker",
    ).catch(() => false);

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
    unlisteners = [ul1, ul2, ulSpeaker, ulSpeakerUnavailable, ul3];
    unlistenFocus = await getCurrentWindow().onFocusChanged(
      ({ payload: focused }) => {
        if (focused) void maybeAutoStartRecording();
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
          <span
            class="font-mono text-label-sm text-fg/60 uppercase tracking-stamped"
          >
            Model {Math.round(
              (modelStore.progressByModel[modelStore.activeDownloadModelId] ??
                0) * 100,
            )}%
          </span>
        {/if}
      </div>
    </header>

    <!-- Body -->
    <div class="grid min-h-0 flex-1 grid-cols-[0.45fr_0.99fr] items-stretch">
      <!-- Left: visualizer + settings -->
      <div class="flex min-h-0 flex-col px-4 py-3">
        <AudioWaveFormVisualizer
          micLevel={micLevel}
          speakerLevel={speakerLevel}
          speakerEnabled={captureSpeaker}
          size="normal"
        />

        <div class="min-h-0 flex-1 overflow-y-auto">
          <Accordion defaultOpenId="basic">
            <AccordionItem id="basic" title="Basic">
              <div class="space-y-4">
                <div class="flex flex-col gap-1.5 text-left">
                  <label
                    for="mic-select"
                    class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase"
                  >
                    Selected mic
                  </label>
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
                    class="h-8 rounded-md border-0 border-b border-transparent bg-panel py-2 pr-8 pl-2 text-body-md text-fg"
                  >
                    {#each micOptions as opt (opt.value)}
                      <option value={opt.value}>{opt.label}</option>
                    {/each}
                  </select>
                </div>
                {#if downloadedModelOptions.length > 0}
                  <div class="flex flex-col gap-1.5 text-left">
                    <label
                      for="model-select"
                      class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase"
                    >
                      Model
                    </label>
                    <select
                      id="model-select"
                      value={selectedModelId}
                      onchange={async (e) => {
                        const id = (e.currentTarget as HTMLSelectElement).value;
                        selectedModelId = id;
                        await modelStore.select(id);
                      }}
                      class="h-8 rounded-md border-0 border-b border-transparent bg-panel py-2 pr-8 pl-2 text-body-md text-fg"
                    >
                      {#each downloadedModelOptions as opt (opt.value)}
                        <option value={opt.value}>{opt.label}</option>
                      {/each}
                    </select>
                  </div>
                {/if}
                <div class="flex items-center justify-between">
                  <span
                    class="font-mono text-label-sm font-normal tracking-stamped uppercase"
                  >
                    Capture speaker
                  </span>
                  <ToggleSwitch
                    checked={captureSpeaker}
                    aria-label="Toggle speaker capture"
                    onchange={async (next) => {
                      captureSpeaker = next;
                      await invoke("settings_set_scribe_capture_speaker", {
                        enabled: next,
                      }).catch(() => {
                        captureSpeaker = !next;
                      });
                    }}
                  />
                </div>
                <div class="flex items-center justify-between">
                  <span
                    class="font-mono text-label-sm font-normal tracking-stamped uppercase"
                  >
                    Transcript timestamps
                  </span>
                  <ToggleSwitch
                    checked={includeTimestamps}
                    aria-label="Toggle transcript timestamps"
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
              </div>
            </AccordionItem>
          </Accordion>
        </div>

        <!-- Footer -->
        <footer class="flex flex-col gap-2 py-3">
          {#if speakerWarning}
            <p class="text-label-sm text-fg/60">{speakerWarning}</p>
          {/if}
          {#if saveFolder}
            <p class="truncate text-label-sm text-fg/40" title={saveFolder}>
              Saving to {saveFolder}
            </p>
          {/if}
          <div class="flex items-center gap-3">
            {#if phase === "idle"}
              {#if autoStart}
                <span
                  class="font-mono text-label-sm text-fg/50 uppercase tracking-stamped"
                >
                  Starting…
                </span>
              {:else}
                <Button variant="primary" onclick={startRecording}
                  >Start Recording</Button
                >
              {/if}
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
                <p class="text-label-sm text-fg/80">
                  No transcription model selected. Install and select one in
                  Settings → Models.
                </p>
                <Button variant="normal" onclick={openSettingsWindow}
                  >Open Settings</Button
                >
              </div>
            {:else if phase === "error"}
              <div class="flex flex-col gap-2">
                <p class="text-label-sm text-destructive">{errorMessage}</p>
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
        <p
          class="mb-2 font-mono text-label-md tracking-stamped text-fg/80 uppercase"
        >
          add notes
        </p>
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
