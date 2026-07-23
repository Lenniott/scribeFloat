<script lang="ts">
  import { Copy, SquareArrowOutUpRight } from "lucide-svelte";
  import IconButton from "../controls/IconButton.svelte";
  import Chip, { type ChipVariant } from "@primitives/display/Chip.svelte";
  import TimestampLabel from "@primitives/display/Timestamp.svelte";

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
    /** When set, body text is visually clipped to this many lines (ellipsis). */
    maxLines,
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
    maxLines?: number;
    onselect?: (id: string) => void;
    oncopy?: () => void;
    onopen?: () => void;
  } = $props();

  const hasActions = $derived(!!(oncopy || onopen));
  const clamped = $derived(maxLines === 2);
  // line-clamp needs overflow + no pre-wrap; keep pre-wrap only for unclamped cards.
  const bodyClass = $derived(
    clamped
      ? 'sf-body-md line-clamp-2 wrap-break-word text-fg'
      : 'sf-body-md whitespace-pre-wrap wrap-break-word text-fg',
  );
</script>

{#snippet header()}
  <div class="flex w-full justify-between">
    <div class="flex items-center gap-4">
      {#if chip}
        <Chip variant={chip.variant}>{chip.label}</Chip>
      {/if}
      {#if timestampLabel}
        <span class="sf-meta-sm text-fg-dim">
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

{#snippet body()}
  <div class="flex flex-col items-stretch gap-1">
    {@render header()}
    <p class={bodyClass}>{note.text}</p>
  </div>
{/snippet}

<!-- h-auto / shrink-0: never stretch to fill a tall parent (practice list bug). -->
<article
  class="h-auto w-full shrink-0 rounded-md px-3 py-2 text-left transition-colors {selected
    ? 'bg-fill'
    : 'bg-card hover:bg-fill/80'}"
>
  {#if hasActions}
    {@render body()}
  {:else if onselect}
    <button
      type="button"
      class="block h-auto w-full text-left"
      onclick={() => onselect?.(note.id)}
    >
      {@render body()}
    </button>
  {:else}
    {@render body()}
  {/if}
</article>
