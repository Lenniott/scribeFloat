<script lang="ts">
  import IconButton from "@lib/components/IconButton.svelte";
  import { SquareArrowOutUpRight } from "lucide-svelte";
  import Trash from "lucide-svelte/icons/trash-2";

  export type TranscribeQueueItemView = {
    id: string;
    source_path: string;
    display_name: string;
    source_type: "single_audio" | "dual_source_session";
    duration_ms: number;
    status: "QUEUED" | "PROCESSING" | "DONE" | "ERROR";
    progress: number;
    transcript_path?: string;
    error?: string;
  };

  let {
    index,
    item,
    canRemove = true,
    onRemove,
    onOpenTranscript,
  }: {
    index: number;
    item: TranscribeQueueItemView;
    canRemove?: boolean;
    onRemove?: (id: string) => void;
    onOpenTranscript?: (path: string) => void;
  } = $props();

  const canOpenTranscript = $derived(item.status === "DONE" && Boolean(item.transcript_path));

  const statusLabel = $derived(
    item.status === "PROCESSING"
      ? `Processing ${Math.round((item.progress || 0) * 100)}%`
      : item.status === "DONE"
        ? "Done"
        : item.status === "ERROR"
          ? "Failed"
          : "Queued",
  );

  const sourceTypeLabel = $derived(
    item.source_type === "dual_source_session" ? "Dual source session" : "Single audio",
  );

  const durationLabel = $derived(formatDuration(item.duration_ms ?? 0));

  function formatDuration(durationMs: number): string {
    const totalSeconds = Math.max(0, Math.round(durationMs / 1000));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    if (hours > 0) {
      return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
    }
    return `${minutes}:${seconds.toString().padStart(2, "0")}`;
  }
</script>

<div class="grid grid-cols-[2rem_minmax(0,1fr)_8.5rem_8rem_2.5rem] items-center gap-2 border-b border-rim px-2 py-2">
  <span class="sf-meta-sm text-fg-dim">{index + 1}</span>
  <div class="min-w-0">
    <p class="truncate sf-body-md text-fg" title={item.source_path}>{item.display_name}</p>
    <p class="truncate sf-label-sm text-fg-dim">{sourceTypeLabel}</p>
  </div>
  <span class="justify-self-end sf-meta-sm text-fg-dim">{durationLabel}</span>
  <span class="justify-self-end sf-meta-sm text-fg-dim">{statusLabel}</span>
  <div class="justify-self-end">
    {#if canOpenTranscript}
      <IconButton
        variant="normal"
        size="small"
        icon={SquareArrowOutUpRight}
        aria-label="Open transcript"
        onclick={() => onOpenTranscript?.(item.transcript_path!)}
      />
    {:else}
      <IconButton
        variant="normal"
        size="small"
        icon={Trash}
        aria-label="Remove file from queue"
        disabled={!canRemove}
        onclick={() => onRemove?.(item.id)}
      />
    {/if}
  </div>
</div>

{#if item.error}
  <p class="px-2 pb-2 sf-label-sm text-destructive">{item.error}</p>
{/if}
