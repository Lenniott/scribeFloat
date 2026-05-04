<script lang="ts">
  import IconButton from "@lib/components/IconButton.svelte";
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
  }: {
    index: number;
    item: TranscribeQueueItemView;
    canRemove?: boolean;
    onRemove?: (id: string) => void;
  } = $props();

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
  <span class="font-mono text-label-sm text-fg/70">{index + 1}</span>
  <div class="min-w-0">
    <p class="truncate text-body-md text-fg" title={item.source_path}>{item.display_name}</p>
    <p class="truncate text-label-sm text-fg/55">{sourceTypeLabel}</p>
  </div>
  <span class="justify-self-end text-label-sm text-fg/75">{durationLabel}</span>
  <span class="justify-self-end text-label-sm text-fg/75">{statusLabel}</span>
  <div class="justify-self-end">
    <IconButton
      variant="normal"
      size="small"
      icon={Trash}
      aria-label="Remove file from queue"
      disabled={!canRemove}
      onclick={() => onRemove?.(item.id)}
    />
  </div>
</div>

{#if item.error}
  <p class="px-2 pb-2 text-label-sm text-destructive">{item.error}</p>
{/if}
