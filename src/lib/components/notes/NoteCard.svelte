<script lang="ts">
  import { Copy, SquareArrowOutUpRight } from "lucide-svelte";
  import IconButton from "@lib/components/IconButton.svelte";
  import Chip, { type ChipVariant } from "@lib/components/Chip.svelte";
  import TimestampLabel from "./TimestampLabel.svelte";

  export type Note = {
    id: string;
    text: string;
    recordedAtMs: number;
  };

  let {
    note,
    selected = false,
    timestampLabel,
    chip,
    onselect,
    oncopy,
    onopen,
  }: {
    note: Note;
    selected?: boolean;
    /** Wall-clock label string (e.g. "2:23 PM"). When omitted, uses TimestampLabel with recordedAtMs. */
    timestampLabel?: string;
    /** Optional badge shown inline with the timestamp. */
    chip?: { label: string; variant: ChipVariant };
    onselect?: (id: string) => void;
    oncopy?: () => void;
    onopen?: () => void;
  } = $props();

  const hasActions = $derived(!!(oncopy || onopen));
</script>

{#snippet header()}
  <div class="flex w-full justify-between">
    <div class="flex items-center gap-4">
      {#if chip}
        <Chip variant={chip.variant}>{chip.label}</Chip>
      {/if}
      {#if timestampLabel}
        <span
          class="font-mono text-label-sm font-normal tabular-nums tracking-stamped text-fg/55"
        >
          {timestampLabel}
        </span>
      {:else}
        <TimestampLabel at={note.recordedAtMs} />
      {/if}
    </div>
    <div class="flex shrink-0 items-center gap-0.5">
      {#if oncopy}
        <IconButton
          aria-label="Copy to clipboard"
          icon={Copy}
          size="small"
          variant="normal"
          onclick={(e) => {
            e.stopPropagation();
            oncopy?.();
          }}
        />
      {/if}
      {#if onopen}
        <IconButton
          aria-label="Open file"
          icon={SquareArrowOutUpRight}
          size="small"
          variant="normal"
          onclick={(e) => {
            e.stopPropagation();
            onopen?.();
          }}
        />
      {/if}
    </div>
  </div>
{/snippet}

<article
  class="rounded-md px-3 py-2 text-left transition-colors {selected
    ? 'bg-fill'
    : 'bg-card hover:bg-fill/80'}"
>
  {#if hasActions}
    <div class="flex flex-col items-start gap-2">
      {@render header()}
      <p class="text-body-md whitespace-pre-wrap wrap-break-word text-fg">
        {note.text}
      </p>
    </div>
  {:else}
    <button
      type="button"
      class="w-full text-left"
      onclick={() => onselect?.(note.id)}
    >
      {@render header()}
      <p class="text-body-md whitespace-pre-wrap text-fg">{note.text}</p>
    </button>
  {/if}
</article>
