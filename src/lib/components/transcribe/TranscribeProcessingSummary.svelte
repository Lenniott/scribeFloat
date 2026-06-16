<script lang="ts">
  import Button from "@lib/components/Button.svelte";
  import type { TranscribeQueueItemView } from "./TranscribeQueueRow.svelte";

  let {
    items,
    onOpenTranscript,
  }: {
    items: TranscribeQueueItemView[];
    onOpenTranscript?: (path: string) => void;
  } = $props();

  const completed = $derived(items.filter((item) => item.status === "DONE"));
  const failed = $derived(items.filter((item) => item.status === "ERROR"));
</script>

<div class="space-y-3">
  <div class="flex items-center gap-4 sf-body-md text-fg-dim">
    <span>{completed.length} completed</span>
    <span>{failed.length} failed</span>
  </div>

  {#if completed.length > 0}
    <div class="space-y-2">
      {#each completed as item (item.id)}
        <div class="flex items-center justify-between gap-3 rounded-md border border-rim px-3 py-2">
          <p class="min-w-0 truncate sf-body-md text-fg" title={item.transcript_path || item.display_name}>
            {item.display_name}
          </p>
          {#if item.transcript_path}
            <Button variant="normal" size="small" onclick={() => onOpenTranscript?.(item.transcript_path!)}>
              Open
            </Button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if failed.length > 0}
    <div class="space-y-2">
      {#each failed as item (item.id)}
        <div class="rounded-md border border-rim px-3 py-2">
          <p class="sf-body-md text-fg">{item.display_name}</p>
          <p class="sf-label-sm text-destructive">{item.error || "Failed to transcribe this item."}</p>
        </div>
      {/each}
    </div>
  {/if}
</div>
