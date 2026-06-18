<script lang="ts">
  import TranscribeQueueRow, {
    type TranscribeQueueItemView,
  } from "../ui/cards/UploadItem.svelte";

  let {
    items,
    canRemove = true,
    onRemove,
    onOpenTranscript,
  }: {
    items: TranscribeQueueItemView[];
    canRemove?: boolean;
    onRemove?: (id: string) => void;
    onOpenTranscript?: (path: string) => void;
  } = $props();
</script>

<div class="rounded-md border border-rim bg-panel min-h-52">
  <div class="grid grid-cols-[2rem_minmax(0,1fr)_8.5rem_8rem_2.5rem] items-center gap-2 border-b border-rim px-2 py-2">
    <span class="sf-label-sm text-fg-dim">#</span>
    <span class="sf-label-sm text-fg-dim">File</span>
    <span class="justify-self-end sf-label-sm text-fg-dim">Duration</span>
    <span class="justify-self-end sf-label-sm text-fg-dim">Status</span>
    <span></span>
  </div>

  {#if items.length === 0}
    <p class="px-3 py-5 sf-body-md text-fg-dim">
      Add audio files or folders to build a transcription queue.
    </p>
  {:else}
    <div class="max-h-56 overflow-y-auto">
      {#each items as item, index (item.id)}
        <TranscribeQueueRow {index} {item} canRemove={canRemove} {onRemove} {onOpenTranscript} />
      {/each}
    </div>
  {/if}
</div>
