<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";

  import Button from "@lib/components/Button.svelte";
  import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
  import PathSelectorField from "@lib/components/form/PathSelectorField.svelte";
  import StackProgressBar from "@lib/components/form/StackProgressBar.svelte";
  import { createModelDownloadStore } from "$lib/stores/modelDownload.svelte";
  import TranscribeQueueList from "@lib/components/transcribe/TranscribeQueueList.svelte";
  import type { TranscribeQueueItemView } from "@lib/components/transcribe/TranscribeQueueRow.svelte";

  type ProcessingStage =
    | "LOADING_MODEL"
    | "TRANSCRIBING_AUDIO"
    | "WRITING_TRANSCRIPT"
    | "CLEANING_UP_AUDIO";

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

  let phase = $state<Phase>("idle");
  let queue = $state<TranscribeQueueItemView[]>([]);
  let outputFolder = $state("");
  let includeTimestamps = $state(true);
  let selectedModelId = $state("");
  let progress = $state(0);
  let stage = $state<ProcessingStage>("LOADING_MODEL");
  let errorMessage = $state("");
  let isDraggingOverDropZone = $state(false);

  const modelStore = createModelDownloadStore();
  let modelUnlisteners: (() => void)[] = [];
  let unlisteners: UnlistenFn[] = [];

  const downloadedModelOptions = $derived(
    modelStore.models
      .filter((m) => m.downloaded)
      .map((m) => ({ value: m.id, label: m.label })),
  );
  const hasQueue = $derived(queue.length > 0);
  const startDisabled = $derived(
    !hasQueue || !outputFolder || phase === "processing" || !selectedModelId,
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

  const progressSequence: { label: string; stage: ProcessingStage }[] = [
    { label: "Loading model", stage: "LOADING_MODEL" },
    { label: "Transcribing audio", stage: "TRANSCRIBING_AUDIO" },
    { label: "Writing transcript", stage: "WRITING_TRANSCRIPT" },
  ];
  const stageOrder = progressSequence.map((step) => step.stage);
  const currentStageIndex = $derived(Math.max(0, stageOrder.indexOf(stage)));
  const sequence = $derived(
    progressSequence.map((step) => ({
      label: step.label,
      complete: stageOrder.indexOf(step.stage) <= currentStageIndex,
    })),
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
      progress = 0;
      stage = "LOADING_MODEL";
      await invoke("transcribe_start", {
        inputPaths: queue.map((item) => item.source_path),
        outputFolder: outputFolder || null,
        modelId: selectedModelId || null,
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
      progress = Math.round(Math.max(0, Math.min(1, payload.progress)) * 100);
    }
    if (payload.processing_stage) {
      stage = payload.processing_stage;
    }

    if (payload.state === "TRANSCRIBING") {
      phase = "processing";
      return;
    }
    if (payload.state === "DONE") {
      progress = 100;
      phase = "done";
      return;
    }
    if (payload.state === "ERROR") {
      phase = "error";
      errorMessage = payload.error || "Transcribe failed.";
      return;
    }
    phase = "idle";
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
    progress = 0;
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

    modelUnlisteners = await modelStore.subscribe();
    await modelStore.refresh();
    const selected = modelStore.models.find(
      (model) => model.downloaded && model.selected,
    );
    selectedModelId = selected?.id ?? "";

    const ulState = await listen<TranscribeStatePayload>(
      "transcribe://state-changed",
      (event) => handleTranscribeState(event.payload),
    );
    const ulProgress = await listen<ItemProgressPayload>(
      "transcribe://item-progress",
      (event) => handleItemProgress(event.payload),
    );
    const ulNativeDrop = await getCurrentWindow().onDragDropEvent((event) => {
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
    unlisteners = [ulState, ulProgress, ulNativeDrop];
  });

  onDestroy(() => {
    unlisteners.forEach((unlisten) => unlisten());
    modelUnlisteners.forEach((unlisten) => unlisten());
  });
</script>

<section class="flex h-screen flex-col overflow-hidden bg-panel text-fg">
  <header
    class="flex min-h-14 items-center justify-between border-b border-rim px-5"
  >
    <h1 class="sf-headline-sm">Transcribe</h1>
    {#if phase === "processing"}
      <p class="font-mono text-label-sm text-fg/55 uppercase tracking-stamped">
        Processing {progress}%
      </p>
    {/if}
  </header>

  <div class="relative flex min-h-0 flex-1 flex-col gap-4 px-5 py-4">
    <div class="flex gap-2 justify-between">
      <div class="space-y-3">
        <PathSelectorField
          label="Save path"
          bind:path={outputFolder}
          directory={true}
          onChange={(value) => {
            outputFolder = value;
          }}
        />

        <div class="flex flex-col gap-1.5">
          <label
            for="transcribe-model"
            class="font-mono text-label-sm text-fg/80 uppercase tracking-stamped"
          >
            Transcription model
          </label>
          <select
            id="transcribe-model"
            bind:value={selectedModelId}
            class="h-10 rounded-md border border-rim bg-panel px-2 text-body-md text-fg"
          >
            <option value="">Select model</option>
            {#each downloadedModelOptions as option (option.value)}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>

        <div class="flex flex-col gap-1.5">
          <span
            class="font-mono text-label-sm text-fg/80 uppercase tracking-stamped"
          >
            Timestamps
          </span>
          <ToggleSwitch
            checked={includeTimestamps}
            onchange={(next) => (includeTimestamps = next)}
          />
        </div>
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
          <p class="text-body-md text-fg/75 w-full">
            {isDraggingOverDropZone
              ? "Release to add to queue."
              : "Drag and drop audio files, or use Add files / Add folder."}
          </p>
          <p class="text-label-sm text-fg/55 w-full">
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

    {#if phase !== "processing" && queue.length === 0}
      <p class="text-body-md text-fg/70">
        Queue files, choose output path and model, then start transcription.
      </p>
    {/if}
    {#if errorMessage}
      <p class="text-label-sm text-destructive">{errorMessage}</p>
    {/if}

    {#if showProcessingOverlay}
      <div
        class="absolute inset-0 z-10 flex items-center justify-center bg-canvas/70 backdrop-blur-[1px]"
      >
        <div class="w-full max-w-2xl rounded-md border border-rim bg-panel p-4">
          <p
            class="mb-3 font-mono text-label-sm text-fg/70 uppercase tracking-stamped"
          >
            Processing {progress}%
          </p>
          <StackProgressBar {progress} {sequence} />
        </div>
      </div>
    {/if}
  </div>

  <footer
    class="flex items-center justify-end gap-2 border-t border-rim px-5 py-3"
  >
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
  </footer>
</section>
