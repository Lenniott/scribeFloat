<script lang="ts">
  import { getContext } from "svelte";
  import type { Snippet } from "svelte";
  import {
    ACCORDION_KEY,
    type AccordionContextState,
  } from "@patterns/accordion-context.js";
  import { Minus, Plus } from "lucide-svelte";

  let {
    id,
    title,
    defaultOpen = false,
    children,
  }: {
    id: string;
    title: string;
    /** When true, this item opens on first render (unless Accordion sets defaultOpenId). */
    defaultOpen?: boolean;
    children?: Snippet;
  } = $props();

  const ctx = getContext<AccordionContextState>(ACCORDION_KEY);
  if (!ctx) {
    throw new Error("AccordionItem must be used inside Accordion");
  }

  // svelte-ignore state_referenced_locally
  const initialDefaultOpen = defaultOpen;
  // svelte-ignore state_referenced_locally
  const itemId = id;

  if (initialDefaultOpen) {
    ctx.claimDefaultOpen(itemId);
  }

  const isOpen = $derived(ctx.openId === id);
</script>

<div class="mb-1">
  <h3 class="bg-card border-0">
    <button
      type="button"
      class="sf-section-label text-fg flex w-full items-center justify-between px-3 py-2.5 text-left hover:bg-rim"
      aria-expanded={isOpen}
      aria-controls={`panel-${id}`}
      id={`header-${id}`}
      onclick={() => ctx.toggle(id)}
    >
      {title}
      <span class="text-fg-dim" aria-hidden="true"
        >{#if isOpen}
          <Minus class="size-4" />
        {:else}<Plus class="size-4" />
        {/if}
      </span>
    </button>
  </h3>
  <div
    id={`panel-${id}`}
    role="region"
    aria-labelledby={`header-${id}`}
    class="grid transition-[grid-template-rows] duration-200 ease-out {isOpen
      ? 'grid-rows-[1fr]'
      : 'grid-rows-[0fr]'}"
  >
    <div class={`min-h-0 overflow-hidden bg-canvas ${isOpen? "my-2" : "my-0"} `}>
      <div class="p-3">
        {@render children?.()}
      </div>
    </div>
  </div>
</div>
