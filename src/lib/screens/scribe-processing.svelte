<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { openPath } from "@tauri-apps/plugin-opener";

  import Button from "@components/Button.svelte";
  import StackProgressBar from "@components/form/StackProgressBar.svelte";
  import { Copy, SquareArrowOutUpLeftIcon, X } from "lucide-svelte";
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
    onClose,
    onRecordAgain,
  }: {
    title: string;
    onClose?: () => void;
    onRecordAgain?: () => void;
  } = $props();

  let phase = $state<Phase>("transcribing");
  let progress = $state(0);
  let transcriptPath = $state("");
  let wavPath = $state("");
  let errorMessage = $state("");
  let processingStage = $state<ProcessingStage>("LOADING_MODEL");
  let started = false;
  let unlisten: UnlistenFn | null = null;

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
      complete: stageOrder.indexOf(step.stage) < currentStageIndex,
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
      case "DONE":
        phase = "done";
        progress = 100;
        transcriptPath = payload.transcript_path ?? "";
        break;
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

  async function openTranscript() {
    if (transcriptPath) await openPath(transcriptPath);
  }

  async function copyPath() {
    if (displayPath) await navigator.clipboard.writeText(displayPath);
  }

  onMount(async () => {
    unlisten = await listen<ScribePayload>("scribe://state-changed", (event) =>
      handleScribeEvent(event.payload),
    );
    await startProcessing();
  });

  onDestroy(() => {
    unlisten?.();
  });
</script>

<div
  class="mx-auto flex min-h-screen w-full max-w-3xl items-center justify-center p-6 text-on-surface"
>
  <section
    class="flex w-full flex-col gap-8 rounded-xl border border-surface-container-low bg-surface-container-lowest p-8 shadow-lg"
  >
    <header class="flex gap-4 w-full justify-between items-center">
      <div class="flex flex-col gap-2">
        <p
          class="font-data text-label-sm tracking-stamped text-on-surface/55 uppercase"
        >
          {title || "Recording"}
        </p>
        <h1 class="text-display-sm font-semibold">
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
	  onclick={onClose}
	  disabled={phase === "transcribing"}
	  icon={X}
	  />
    </header>

    {#if phase === "transcribing"}
      <StackProgressBar {progress} {sequence} />
    {:else}
      <div class="flex flex-col gap-4">
        {#if phase === "done"}
          <div class="flex items-center justify-between">
            <p class="text-body-md text-on-surface/80">Transcript saved.</p>
            {#if phase === "done"}
              <div class="flex gap-2">
                <IconButton
                  aria-label="copy transcript to clipboard"
                  variant="normal"
                  icon={Copy}
                  onclick={copyPath}
                />
                <IconButton
                  aria-label="Open Transcript"
                  icon={SquareArrowOutUpLeftIcon}
                  variant="normal"
                  onclick={openTranscript}
                />
              </div>
            {/if}
          </div>
        {:else if phase === "no_model"}
          <p class="text-body-md text-on-surface/80">
            No downloaded model was available. The WAV file was kept so this
            recording can be transcribed later.
          </p>
        {:else}
          <p class="text-body-md text-error">{errorMessage}</p>
        {/if}

        {#if displayPath}
          <button class="cursor-pointer group" onclick={openTranscript}>
            <p
              class="truncate font-data text-body-sm text-primary underline decoration-primary/60 group-hover:underline-offset-2"
              title={displayPath}
            >
              {displayPath}
            </p>
          </button>
        {/if}
      </div>
    {/if}

    <footer class="flex flex-wrap justify-end gap-3">
      {#if phase === "done"}
        <Button variant="secondary" onclick={onRecordAgain}>Record Again</Button
        >
      {:else if phase === "no_model"}
        <Button variant="secondary" onclick={copyPath} disabled={!displayPath}
          >Copy WAV Path</Button
        >
        <Button variant="primary" onclick={onRecordAgain}>Record Again</Button>
      {:else if phase === "error"}
        <Button variant="primary" onclick={onRecordAgain}>Try Again</Button>
      {/if}
    </footer>
  </section>
</div>
