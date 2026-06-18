<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { browser } from "$app/environment";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { copyTranscript } from "$lib/services/clipboard";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { loadTranscriptPreview } from "$lib/services/historyTranscript";
  import { openHistoryMarkdown } from "$lib/services/historyActions";

  import Button from "@components/controls/Button.svelte";
  import Modal from "@primitives/layout/Modal.svelte";
  import StackProgressBar from "@primitives/display/ProgressBar.svelte";
  import ScrollablePanel from "@primitives/layout/ScrollBody.svelte";
  import Toast from "@components/indicators/Toast.svelte";
  import type { ToastState } from "@components/indicators/Toast.svelte";
  import { Copy, SquareArrowOutUpRight } from "lucide-svelte";
  import IconButton from "@components/controls/IconButton.svelte";

  type Phase = "transcribing" | "done" | "no_model" | "error";
  type ProcessingStage =
    | "LOADING_MODEL"
    | "TRANSCRIBING_AUDIO"
    | "WRITING_TRANSCRIPT"
    | "CLEANING_UP_AUDIO";

  type ScribePayload = {
    state: string;
    progress?: number;
    processing_stage?: ProcessingStage;
    transcript_path?: string;
    wav_path?: string;
    error?: string;
    history_record_id?: string;
  };

  let {
    title,
    onRecordAgain,
    embedded = false,
    onOpenSettings,
    registerLeaveHandler,
  }: {
    title: string;
    onRecordAgain?: () => void;
    embedded?: boolean;
    onOpenSettings?: () => void;
    registerLeaveHandler?: (handler: (proceed: () => void) => void) => void;
  } = $props();

  let phase = $state<Phase>("transcribing");
  let progress = $state(0);
  let transcriptPath = $state("");
  let recordId = $state("");
  let wavPath = $state("");
  let errorMessage = $state("");
  let processingStage = $state<ProcessingStage>("LOADING_MODEL");
  let started = false;
  let unlisteners: UnlistenFn[] = [];

  let bodyText = $state("");
  let previewError = $state("");
  let loadingPreview = $state(false);

  let toastMessage = $state("");
  let toastState = $state<ToastState>("normal");
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

  let abortConfirmOpen = $state(false);
  let abortInProgress = $state(false);
  let pendingLeave: (() => void) | null = null;

  function requestLeave(proceed: () => void) {
    if (phase === "transcribing") {
      pendingLeave = proceed;
      abortConfirmOpen = true;
      return;
    }
    proceed();
  }

  function openSettings() {
    if (onOpenSettings) {
      onOpenSettings();
      return;
    }
    void invoke("settings_show_window").catch(() => {});
  }

  function showToast(msg: string, state: ToastState = "normal") {
    if (toastTimeout) clearTimeout(toastTimeout);
    toastMessage = msg;
    toastState = state;
    toastTimeout = setTimeout(() => {
      toastMessage = "";
      toastTimeout = null;
    }, 3000);
  }

  const progressSequence: { label: string; stage: ProcessingStage }[] = [
    { label: "Loading model", stage: "LOADING_MODEL" },
    { label: "Transcribing audio", stage: "TRANSCRIBING_AUDIO" },
    { label: "Writing transcript", stage: "WRITING_TRANSCRIPT" },
    { label: "Cleaning up audio", stage: "CLEANING_UP_AUDIO" },
  ];
  const stageOrder = progressSequence.map((step) => step.stage);

  const displayPath = $derived(transcriptPath || wavPath);
  const canCopy = $derived(!!(transcriptPath || recordId));
  const currentStageIndex = $derived(stageOrder.indexOf(processingStage));

  const sequence = $derived(
    progressSequence.map((step) => ({
      label: step.label,
      complete: stageOrder.indexOf(step.stage) <= currentStageIndex,
    })),
  );

  async function loadDonePreview() {
    if (!recordId) {
      bodyText = "";
      previewError = "";
      return;
    }
    loadingPreview = true;
    previewError = "";
    try {
      bodyText = await loadTranscriptPreview(recordId);
    } catch (e) {
      bodyText = "";
      previewError = String(e);
    } finally {
      loadingPreview = false;
    }
  }

  function handleScribeEvent(payload: ScribePayload) {
    switch (payload.state) {
      case "TRANSCRIBING":
        phase = "transcribing";
        if (payload.processing_stage)
          processingStage = payload.processing_stage;
        if (payload.progress != null) {
          progress = Math.round(
            Math.max(0, Math.min(1, payload.progress)) * 100,
          );
        }
        break;
      case "DONE": {
        transcriptPath = payload.transcript_path ?? "";
        recordId = payload.history_record_id ?? "";
        processingStage = "CLEANING_UP_AUDIO";
        progress = 100;
        setTimeout(() => {
          phase = "done";
          void loadDonePreview();
        }, 800);
        break;
      }
      case "NO_MODEL":
        phase = "no_model";
        progress = 0;
        wavPath = payload.wav_path ?? "";
        break;
      case "ERROR":
        phase = "error";
        progress = 0;
        errorMessage = payload.error ?? "Unknown error";
        break;
    }
  }

  async function startProcessing() {
    if (started) return;
    started = true;

    try {
      await invoke("scribe_stop_and_save", { title: title || "Recording" });
    } catch (error) {
      phase = "error";
      progress = 0;
      errorMessage = String(error);
    }
  }

  async function closeScribeWindowCompletely(): Promise<void> {
    if (!browser) return;
    if (phase === "transcribing") {
      await invoke("scribe_abort_transcription").catch(() => {});
    }
    await invoke("scribe_cancel").catch(() => {});
    if (phase !== "transcribing") {
      onRecordAgain?.();
    }
    if (!embedded) {
      await invoke("scribe_destroy_window").catch(() => {});
    }
  }

  async function handleCloseRequest(): Promise<void> {
    if (phase === "transcribing") {
      abortConfirmOpen = true;
      return;
    }
    await closeScribeWindowCompletely();
  }

  async function confirmAbort(): Promise<void> {
    abortInProgress = true;
    try {
      abortConfirmOpen = false;
      if (pendingLeave) {
        const go = pendingLeave;
        pendingLeave = null;
        if (phase === "transcribing") {
          await invoke("scribe_abort_transcription").catch(() => {});
        }
        await invoke("scribe_cancel").catch(() => {});
        go();
      } else {
        await closeScribeWindowCompletely();
      }
    } finally {
      abortInProgress = false;
    }
  }

  async function openTranscript() {
    if (transcriptPath) {
      await openHistoryMarkdown(transcriptPath);
    }
  }

  async function copyContent() {
    try {
      if (transcriptPath) {
        await copyTranscript(transcriptPath);
      } else if (recordId) {
        const text = bodyText || (await loadTranscriptPreview(recordId));
        await writeText(text);
      }
      showToast("Copied to clipboard", "success");
    } catch (e) {
      showToast("Copy failed: " + String(e), "error");
    }
  }

  onMount(async () => {
    const ulState = await listen<ScribePayload>(
      "scribe://state-changed",
      (event) => handleScribeEvent(event.payload),
    );
    const ulClose = embedded
      ? null
      : await listen("scribe://native-close-requested", () => {
          void handleCloseRequest();
        });
    const ulOpened = await listen("scribe://opened", () => {
      if (phase === "done" || phase === "no_model" || phase === "error") {
        onRecordAgain?.();
      }
    });
    unlisteners = ulClose ? [ulState, ulClose, ulOpened] : [ulState, ulOpened];
    registerLeaveHandler?.(requestLeave);
    await startProcessing();
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
  });
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden text-fg">
  <section class="flex {embedded ? 'h-full min-h-0' : 'h-screen'} flex-col overflow-hidden bg-panel">
    <header class="flex min-h-14 items-end justify-between border-b border-card px-5 py-2 shrink-0">
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <p class="sf-section-label text-fg-dim">
          {title || "Recording"}
        </p>
        <h1 class="sf-headline-sm text-fg">
          {#if phase === "transcribing"}
            Processing...
          {:else if phase === "done"}
            Complete
          {:else if phase === "no_model"}
            Model Needed
          {:else}
            Processing Failed
          {/if}
        </h1>
      </div>
    </header>

    <div class="flex min-h-0 flex-1 flex-col px-5 py-4">
      {#if phase === "transcribing"}
        <div class="flex flex-1 flex-col justify-center">
          <StackProgressBar
            {progress}
            {sequence}
            indeterminate={processingStage === "LOADING_MODEL"}
          />
        </div>
      {:else if phase === "done"}
        <div class="flex min-h-0 flex-1 flex-col gap-3">
          <div class="flex shrink-0 items-center justify-between gap-2">
            <p class="sf-body-md text-fg">Transcript saved.</p>
            <div class="flex gap-2">
              {#if canCopy}
                <IconButton
                  aria-label="copy transcript to clipboard"
                  variant="normal"
                  icon={Copy}
                  onclick={copyContent}
                />
              {/if}
              {#if transcriptPath}
                <IconButton
                  aria-label="Open Transcript"
                  icon={SquareArrowOutUpRight}
                  variant="normal"
                  onclick={openTranscript}
                />
              {/if}
            </div>
          </div>
          {#if !recordId && !transcriptPath}
            <p class="sf-label-md text-destructive">
              Transcript could not be saved to history.
            </p>
          {:else if loadingPreview}
            <p class="sf-label-md text-fg-muted">Loading…</p>
          {:else if previewError}
            <p class="sf-label-md text-destructive">Could not load transcript.</p>
            <p class="sf-label-sm text-fg-muted">{previewError}</p>
          {:else if bodyText}
            <ScrollablePanel class="px-0 py-0">
              <p class="sf-body-md whitespace-pre-wrap wrap-break-word text-fg">
                {bodyText}
              </p>
            </ScrollablePanel>
          {:else if transcriptPath}
            <button
              type="button"
              class="cursor-pointer group p-0 text-left"
              onclick={openTranscript}
            >
              <p
                class="truncate sf-body-md text-fg underline decoration-fg-muted group-hover:underline-offset-2"
                title={transcriptPath}
              >
                {transcriptPath}
              </p>
            </button>
          {:else}
            <p class="sf-label-md text-fg-muted">No content available.</p>
          {/if}
        </div>
      {:else}
        <div class="flex flex-1 flex-col justify-center gap-4">
          {#if phase === "no_model"}
            <p class="sf-body-md text-fg-dim">
              No transcription model is installed. Your recording was saved as
              a WAV file and can be transcribed once a model is downloaded.
            </p>
            <Button variant="normal" onclick={openSettings}>
              Open Settings
            </Button>
          {:else}
            <p class="sf-body-md text-destructive">{errorMessage}</p>
          {/if}

          {#if displayPath}
            <button
              type="button"
              class="cursor-pointer group p-0 text-left"
              onclick={() =>
                transcriptPath ? openTranscript() : undefined}
            >
              <p
                class="truncate sf-body-md text-fg underline decoration-fg-muted group-hover:underline-offset-2"
                title={displayPath}
              >
                {displayPath}
              </p>
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <footer class="flex shrink-0 flex-wrap justify-end gap-3 border-t border-card px-5 py-3">
      {#if phase === "done"}
        <Button variant="normal" onclick={onRecordAgain}>Record Again</Button>
      {:else if phase === "no_model"}
        <Button
          variant="normal"
          disabled={!displayPath}
          onclick={() => displayPath && navigator.clipboard.writeText(displayPath)}
        >Copy WAV Path</Button>
        <Button variant="primary" onclick={onRecordAgain}>Record Again</Button>
      {:else if phase === "error"}
        <Button variant="primary" onclick={onRecordAgain}>Try Again</Button>
      {/if}
    </footer>
  </section>
</div>

<Modal
  open={abortConfirmOpen}
  title="Abort transcription?"
  description="Transcription is still running. Your audio file has been saved and can be transcribed later via the Transcribe tab."
  maxWidthClass="max-w-md"
  closeDisabled={abortInProgress}
  onClose={() => {
    abortConfirmOpen = false;
    pendingLeave = null;
  }}
>
  {#snippet footer()}
    <div class="flex gap-2">
      <Button
        variant="normal"
        disabled={abortInProgress}
        onclick={() => (abortConfirmOpen = false)}
      >Keep Processing</Button>
      <Button
        variant="destructive"
        disabled={abortInProgress}
        onclick={confirmAbort}
      >Abort</Button>
    </div>
  {/snippet}
</Modal>

<Toast message={toastMessage} state={toastState} />
