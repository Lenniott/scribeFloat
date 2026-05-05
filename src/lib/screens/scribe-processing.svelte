<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { browser } from "$app/environment";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { copyTranscript } from "$lib/services/clipboard";

  import Button from "@components/Button.svelte";
  import StackProgressBar from "@components/form/StackProgressBar.svelte";
  import Toast from "@components/Toast.svelte";
  import type { ToastState } from "@components/Toast.svelte";
  import { Copy, SquareArrowOutUpRight, X } from "lucide-svelte";
  import IconButton from "@lib/components/IconButton.svelte";

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
  };

  let {
    title,
    onRecordAgain,
  }: {
    title: string;
    onRecordAgain?: () => void;
  } = $props();

  let phase = $state<Phase>("transcribing");
  let progress = $state(0);
  let transcriptPath = $state("");
  let wavPath = $state("");
  let errorMessage = $state("");
  let processingStage = $state<ProcessingStage>("LOADING_MODEL");
  let started = false;
  let unlisteners: UnlistenFn[] = [];

  let toastMessage = $state("");
  let toastState = $state<ToastState>("normal");
  let toastTimeout: ReturnType<typeof setTimeout> | null = null;

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
  const currentStageIndex = $derived(stageOrder.indexOf(processingStage));

  const sequence = $derived(
    progressSequence.map((step) => ({
      label: step.label,
      complete: stageOrder.indexOf(step.stage) <= currentStageIndex,
    })),
  );

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
        const path = payload.transcript_path ?? "";
        transcriptPath = path;
        processingStage = "CLEANING_UP_AUDIO";
        progress = 100;
        setTimeout(() => {
          phase = "done";
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
    await invoke("scribe_destroy_window").catch(() => {});
  }

  async function openTranscript() {
    if (transcriptPath)
      await invoke("settings_open_transcript", { filePath: transcriptPath });
  }

  async function copyContent() {
    if (!transcriptPath) return;
    try {
      await copyTranscript(transcriptPath);
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
    const ulClose = await listen("scribe://native-close-requested", () => {
      void closeScribeWindowCompletely();
    });
    unlisteners = [ulState, ulClose];
    await startProcessing();
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
  });
</script>

<div class="mx-auto flex flex-col text-fg">
  <section class="flex h-screen flex-col overflow-hidden bg-panel">
    <header class="flex min-h-14 items-end justify-between border-b border-card px-5 py-2">
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <p class="font-mono text-label-sm tracking-stamped text-fg/55 uppercase">
          {title || "Recording"}
        </p>
        <h1 class="sf-headline-sm">
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
      <IconButton
        variant="normal"
        aria-label="close panel"
        onclick={() => void closeScribeWindowCompletely()}
        icon={X}
      />
    </header>

    <div class="flex min-h-0 flex-1 flex-col justify-center gap-8 px-5 py-6">
      {#if phase === "transcribing"}
        <StackProgressBar {progress} {sequence} />
      {:else}
        <div class="flex flex-col gap-4">
          {#if phase === "done"}
            <div class="flex items-center justify-between">
              <p class="text-body-md text-fg/80">Transcript saved.</p>
              <div class="flex gap-2">
                <IconButton
                  aria-label="copy transcript to clipboard"
                  variant="normal"
                  icon={Copy}
                  onclick={copyContent}
                />
                <IconButton
                  aria-label="Open Transcript"
                  icon={SquareArrowOutUpRight}
                  variant="normal"
                  onclick={openTranscript}
                />
              </div>
            </div>
          {:else if phase === "no_model"}
            <p class="text-body-md text-fg/80">
              No transcription model is installed. Your recording was saved as
              a WAV file and can be transcribed once a model is downloaded.
            </p>
            <Button
              variant="normal"
              onclick={() => void invoke("settings_show_window").catch(() => {})}
            >
              Open Settings
            </Button>
          {:else}
            <p class="text-body-md text-destructive">{errorMessage}</p>
          {/if}

          {#if displayPath}
            <button class="cursor-pointer group p-0 text-left" onclick={openTranscript}>
              <p
                class="truncate font-mono text-body-md text-fg underline decoration-fg-muted group-hover:underline-offset-2"
                title={displayPath}
              >
                {displayPath}
              </p>
            </button>
          {/if}
        </div>
      {/if}
    </div>

    <footer class="flex flex-wrap justify-end gap-3 border-t border-card px-5 py-3">
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

<Toast message={toastMessage} state={toastState} />
