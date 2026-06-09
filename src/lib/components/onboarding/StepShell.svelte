<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    subtitle = "",
    children,
    footer,
  }: {
    title: string;
    subtitle?: string;
    children?: Snippet;
    footer?: Snippet;
  } = $props();
</script>

<div class="flex flex-col w-full h-full justify-between">
  <div class="min-h-0 flex flex-col">
    <div class="flex flex-col gap-1 mb-5">
      <h1 class="sf-headline-sm text-fg">{title}</h1>
      {#if subtitle}
        <p class="text-body-md text-fg-dim">{subtitle}</p>
      {/if}
    </div>
  </div>
  <div class="flex-1 min-h-0 flex flex-col">
    {@render children?.()}
  </div>

  {#if footer}
    <footer class="mt-8 flex items-center justify-between gap-3 shrink-0">
      {@render footer()}
    </footer>
  {/if}
</div>
