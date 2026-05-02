<script lang="ts">
  import { getContext } from "svelte";
  import type { Snippet } from "svelte";
  import {
    ACCORDION_KEY,
    type AccordionContextState,
  } from "./accordion-context.js";
  import { Minimize, Minus, Plus } from "lucide-svelte";

  let {
    id,
    title,
    children,
  }: {
    id: string;
    title: string;
    children?: Snippet;
  } = $props();

  const ctx = getContext<AccordionContextState>(ACCORDION_KEY);
  if (!ctx) {
    throw new Error("AccordionItem must be used inside Accordion");
  }

  const isOpen = $derived(ctx.openId === id);
</script>

<div class="mb-1">
  <h3 class="bg-card border-0">
    <button
      type="button"
      class="font-mono text-label-md flex w-full items-center justify-between px-3 py-2.5 text-left font-normal tracking-stamped text-fg uppercase hover:bg-fillest"
      aria-expanded={isOpen}
      aria-controls={`panel-${id}`}
      id={`header-${id}`}
      onclick={() => ctx.toggle(id)}
    >
      {title}
      <span class="text-fg/50" aria-hidden="true"
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
