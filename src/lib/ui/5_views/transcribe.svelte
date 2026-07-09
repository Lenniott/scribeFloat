<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";

  import Button from "@components/controls/Button.svelte";
  import ToggleSwitch from "@components/controls/Toggle.svelte";
  import PathPicker from "@components/controls/PathPicker.svelte";
  import ProgressBar from "@primitives/display/ProgressBar.svelte";
  import AnimatedEllipsis from "@primitives/display/AnimatedEllipsis.svelte";
  import TranscribeQueueList from "@patterns/UploadQueue.svelte";
  import ScrollablePanel from "@primitives/layout/ScrollBody.svelte";
  import PanelFooter from "@primitives/layout/PanelFooter.svelte";
  import type { TranscribeQueueItemView } from "@components/cards/UploadItem.svelte";
  import { UPLOAD_STEPS, type ProcessingStage } from "@utils/processingFeedback";
  import { createCaptureProgress } from "@stores/captureProgress.svelte";

  type TranscribeStatePayload = {
    state: "IDLE" | "TRANSCRIBING" | "DONE" | "ERROR";
    progress?: number;
    processing_stage?: ProcessingStage;
    total_items: number;
    completed_items: number;
    failed_items: number;
    items: TranscribeQueueItemView[];
    error?: string;
  };

  type ItemProgressPayload = {
    item_id: string;
    progress: number;
  };

  type Phase = "idle" | "processing" | "done" | "error";

  let { embedded = false }: { embedded?: boolean } = $props();

  let phase = $state<Phase>("idle");
  let queue = $state<TranscribeQueueItemView[]>([]);
  let outputFolder = $state("");
  let includeTimestamps = $state(true);
  /** Queue-average progress 0..1; display derives via the capture store. */
  let rawProgress = 0;
  let stage: ProcessingStage = "LOADING_MODEL";
  let errorMessage = $state("");
  let isDraggingOverDropZone = $state(false);

  let unlisteners: UnlistenFn[] = [];

  const hasQueue = $derived(queue.length > 0);
  const startDisabled = $derived(
    !hasQueue || !outputFolder || phase === "processing",
  );
  const canAcceptDrop = $derived(phase !== "processing");
  const showProcessingOverlay = $derived(phase === "processing");
  const dropZoneClass = $derived(
    [
      "w-full rounded-md border border-dashed px-4 py-5 text-center transition-[background-color,border-color]",
      isDraggingOverDropZone && canAcceptDrop
        ? "border-active bg-active/15"
        : "border-rim bg-transparent",
    ].join(" "),
  );

  const capture = createCaptureProgress(UPLOAD_STEPS, { batch: true });
  const currentStepLabel = $derived(
    capture.sequence.find((step) => !step.complete)?.label ?? "Processing",
  );

  function uniquePaths(paths: string[]): string[] {
    const out: string[] = [];
    for (const path of paths) {
      const trimmed = path.trim();
      if (!trimmed || out.includes(trimmed)) continue;
      out.push(trimmed);
    }
    return out;
  }

  async function inspectPaths(paths: string[]) {
    const normalized = uniquePaths(paths);
    if (normalized.length === 0) {
      queue = [];
      return;
    }
    queue = await invoke<TranscribeQueueItemView[]>(
      "transcribe_inspect_inputs",
      {
        inputPaths: normalized,
      },
    );
  }

  async function queuePaths(nextPaths: string[]) {
    const merged = uniquePaths([
      ...queue.map((item) => item.source_path),
      ...nextPaths,
    ]);
    try {
      await inspectPaths(merged);
      errorMessage = "";
      if (phase === "error") phase = "idle";
    } catch (error) {
      phase = "error";
      errorMessage = String(error);
    }
  }

  async function addFiles() {
    const picked = await open({
      multiple: true,
      directory: false,
      filters: [
        {
          name: "Audio",
          extensions: ["mp3", "m4a", "wav", "ogg", "flac"],
        },
      ],
    }).catch(() => null);
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    if (paths.length === 0) return;
    await queuePaths(paths);
  }

  async function addFolder() {
    const picked = await open({
      directory: true,
      multiple: false,
    }).catch(() => null);
    if (typeof picked !== "string" || !picked) return;
    await queuePaths([picked]);
  }

  async function removeQueueItem(id: string) {
    const remaining = queue
      .filter((item) => item.id !== id)
      .map((item) => item.source_path);
    try {
      await inspectPaths(remaining);
      if (queue.length === 0 && phase === "done") {
        phase = "idle";
      }
    } catch (error) {
      phase = "error";
      errorMessage = String(error);
    }
  }

  function handleDrop(event: DragEvent) {
    event.preventDefault();
    isDraggingOverDropZone = false;
    if (!canAcceptDrop) return;
    const dropped = Array.from(event.dataTransfer?.files ?? [])
      .map((file) => (file as File & { path?: string }).path ?? "")
      .filter(Boolean);
    if (dropped.length > 0) {
      void queuePaths(dropped);
    }
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    if (canAcceptDrop) {
      isDraggingOverDropZone = true;
    }
  }

  function handleDragEnter(event: DragEvent) {
    event.preventDefault();
    if (canAcceptDrop) {
      isDraggingOverDropZone = true;
    }
  }

  function handleDragLeave(event: DragEvent) {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && event.currentTarget instanceof Node) {
      if (event.currentTarget.contains(nextTarget)) return;
    }
    isDraggingOverDropZone = false;
  }

  async function startTranscribe() {
    try {
      errorMessage = "";
      phase = "processing";
      rawProgress = 0;
      stage = "LOADING_MODEL";
      capture.reset();
      capture.update(stage, rawProgress);
      await invoke("transcribe_start", {
        inputPaths: queue.map((item) => item.source_path),
        outputFolder: outputFolder || null,
        modelId: null,
        includeTimestamps,
      });
    } catch (error) {
      phase = "error";
      errorMessage = String(error);
    }
  }

  async function openTranscript(path: string) {
    await invoke("transcribe_open_output", { filePath: path }).catch(() => {});
  }

  function handleTranscribeState(payload: TranscribeStatePayload) {
    queue = payload.items || queue;
    if (payload.progress != null) {
      rawProgress = Math.max(0, Math.min(1, payload.progress));
    }
    if (payload.processing_stage) {
      stage = payload.processing_stage;
    }
    if (payload.progress != null || payload.processing_stage) {
      capture.update(stage, rawProgress);
    }

    if (payload.state === "TRANSCRIBING") {
      phase = "processing";
      return;
    }
    if (payload.state === "DONE") {
      rawProgress = 1;
      capture.complete();
      phase = "done";
      return;
    }
    if (payload.state === "ERROR") {
      phase = "error";
      capture.reset();
      errorMessage = payload.error || "Transcribe failed.";
      return;
    }
    phase = "idle";
    capture.reset();
  }

  function handleItemProgress(payload: ItemProgressPayload) {
    if (phase !== "processing") return;
    queue = queue.map((item) =>
      item.id === payload.item_id
        ? {
            ...item,
            status: "PROCESSING",
            progress: Math.max(0, Math.min(1, payload.progress)),
          }
        : item,
    );
  }

  function resetForAnotherRun() {
    phase = "idle";
    rawProgress = 0;
    stage = "LOADING_MODEL";
    errorMessage = "";
    queue = [];
  }

  onMount(async () => {
    outputFolder = await invoke<string>("settings_get_output_path").catch(
      () => "",
    );
    includeTimestamps = await invoke<boolean>(
      "scribe_get_include_timestamps",
    ).catch(() => true);

    const ulState = await listen<TranscribeStatePayload>(
      "transcribe://state-changed",
      (event) => handleTranscribeState(event.payload),
    );
    const ulProgress = await listen<ItemProgressPayload>(
      "transcribe://item-progress",
      (event) => handleItemProgress(event.payload),
    );
    const ulNativeDrop = embedded
      ? null
      : await getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === "enter" || event.payload.type === "over") {
        if (canAcceptDrop) {
          isDraggingOverDropZone = true;
        }
        return;
      }
      if (event.payload.type === "leave") {
        isDraggingOverDropZone = false;
        return;
      }
      if (event.payload.type === "drop") {
        isDraggingOverDropZone = false;
        if (!canAcceptDrop) return;
        void queuePaths(event.payload.paths);
      }
    });
    unlisteners = [ulState, ulProgress, ...(ulNativeDrop ? [ulNativeDrop] : [])];
  });

  onDestroy(() => {
    unlisteners.forEach((unlisten) => unlisten());
    capture.reset();
  });
</script>

<section class="flex h-full min-h-0 flex-col overflow-hidden bg-panel text-fg">
  {#if !embedded}
    <header
      class="flex min-h-14 shrink-0 items-center justify-between border-b border-rim px-5"
    >
      <h1 class="sf-headline-sm text-fg">Transcribe</h1>
      {#if phase !== "processing" && queue.length === 0}
        <p class="sf-body-md text-fg-dim">
          Queue files, choose an output path, then start transcription.
        </p>
      {/if}
      {#if phase === "processing"}
        <p class="sf-label-sm text-fg-dim">
          {#if capture.loading}
            {capture.stageLabel}<AnimatedEllipsis />
          {:else}
            Processing {capture.percent}%
          {/if}
        </p>
      {/if}
      {#if errorMessage}
        <p class="sf-label-sm text-destructive">{errorMessage}</p>
      {/if}
    </header>
  {:else}
    <header class="shrink-0 px-6 pt-6">
      <h1 class="sf-headline-sm text-fg">Upload</h1>
      <p class="mt-0.5 sf-body-md text-fg-dim">
        Import audio files and transcribe them into your library.
      </p>
      {#if errorMessage}
        <p class="mt-2 sf-label-sm text-destructive">{errorMessage}</p>
      {/if}
    </header>
  {/if}

  <ScrollablePanel class="relative flex flex-col gap-4 {embedded ? 'px-6 py-4' : 'px-5 py-4'}">
    <div class="flex gap-2 justify-between">
      <div class="space-y-3">
        <PathPicker
          label="Save path"
          bind:path={outputFolder}
          directory={true}
          onChange={(value) => {
            outputFolder = value;
          }}
        />

        <ToggleSwitch
          label="Timestamps"
          labelFirst
          class="flex-col gap-1.5 items-start"
          checked={includeTimestamps}
          onchange={(next) => (includeTimestamps = next)}
        />
      </div>
      <div class="flex gap-1 max-w-xl mt-5">
        <div
          class={dropZoneClass}
          role="region"
          aria-label={isDraggingOverDropZone
            ? "Release to add files"
            : "Drop audio files here"}
          ondragenter={handleDragEnter}
          ondrop={handleDrop}
          ondragover={handleDragOver}
          ondragleave={handleDragLeave}
        >
          <p class="sf-body-md text-fg-dim w-full">
            {isDraggingOverDropZone
              ? "Release to add to queue."
              : "Drag and drop audio files, or use Add files / Add folder."}
          </p>
          <p class="sf-label-sm text-fg-dim w-full">
            {isDraggingOverDropZone
              ? "Files and folders will be inspected before transcription."
              : "Supported: mp3, m4a, wav, ogg, flac."}
          </p>
        </div>
        <div class="flex flex-col gap-2 min-w-24">
          <Button
            variant="normal"
            onclick={addFiles}
            disabled={phase === "processing"}
          >
            Add files
          </Button>
          <Button
            variant="normal"
            onclick={addFolder}
            disabled={phase === "processing"}
          >
            Add folder
          </Button>
        </div>
      </div>
    </div>
    <TranscribeQueueList
      items={queue}
      canRemove={phase !== "processing"}
      onRemove={removeQueueItem}
      onOpenTranscript={openTranscript}
    />

    {#if showProcessingOverlay}
      <div
        class="absolute inset-0 z-10 flex items-center justify-center bg-canvas/70 backdrop-blur-[1px]"
      >
        <div class="w-full max-w-2xl rounded-md border border-rim bg-panel p-4">
          {#if capture.loading}
            <p class="sf-label-sm text-fg-dim">
              {capture.stageLabel}<AnimatedEllipsis />
            </p>
          {:else}
            <p class="mb-3 sf-label-sm text-fg-dim">
              {currentStepLabel} · {capture.percent}%
            </p>
            <ProgressBar progress={capture.percentExact} fluid />
          {/if}
        </div>
      </div>
    {/if}
  </ScrollablePanel>

  <PanelFooter class="gap-2 {embedded ? 'px-6' : 'px-5'}">
    <Button
      variant="normal"
      onclick={() => {
        queue = [];
        errorMessage = "";
        if (phase !== "processing") phase = "idle";
      }}
      disabled={phase === "processing" || queue.length === 0}
    >
      Clear list
    </Button>
    {#if phase === "done"}
      <Button variant="normal" onclick={resetForAnotherRun}
        >Transcribe More</Button
      >
    {/if}
    <Button
      variant="primary"
      onclick={startTranscribe}
      disabled={startDisabled}
    >
      Transcribe
    </Button>
  </PanelFooter>
</section>
