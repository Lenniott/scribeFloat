<script lang="ts">
  import TranscribeQueueRow, {
    type TranscribeQueueItemView,
  } from "./TranscribeQueueRow.svelte";

  let {
    items,
    canRemove = true,
    onRemove,
  }: {
    items: TranscribeQueueItemView[];
    canRemove?: boolean;
    onRemove?: (id: string) => void;
  } = $props();
</script>

<div class="rounded-md border border-rim bg-panel">
  <div class="grid grid-cols-[2rem_minmax(0,1fr)_8.5rem_8rem_2.5rem] items-center gap-2 border-b border-rim px-2 py-2">
    <span class="font-mono text-label-sm text-fg/55 uppercase tracking-stamped">#</span>
    <span class="font-mono text-label-sm text-fg/55 uppercase tracking-stamped">File</span>
    <span class="justify-self-end font-mono text-label-sm text-fg/55 uppercase tracking-stamped">Duration</span>
    <span class="justify-self-end font-mono text-label-sm text-fg/55 uppercase tracking-stamped">Status</span>
    <span></span>
  </div>

  {#if items.length === 0}
    <p class="px-3 py-5 text-body-md text-fg/60">
      Add audio files or folders to build a transcription queue.
    </p>
  {:else}
    <div class="max-h-56 overflow-y-auto">
      {#each items as item, index (item.id)}
        <TranscribeQueueRow {index} {item} canRemove={canRemove} {onRemove} />
      {/each}
    </div>
  {/if}
</div>
