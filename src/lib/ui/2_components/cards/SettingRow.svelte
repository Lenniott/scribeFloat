<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    direction = "vertical",
    title,
    description,
    disabled = false,
    class: className = "",
    control,
    children,
  }: {
    title: string;
    description?: string;
    disabled?: boolean;
    direction?: "horizontal" | "vertical";
    class?: string;
    control?: Snippet;
    children?: Snippet;
  } = $props();
</script>

<div
  class={`flex flex-col gap-2 transition sm:flex-row sm:items-center sm:justify-between sm:gap-4 ${disabled ? "opacity-50" : ""} ${className}`.trim()}
>
  <div
    class={`min-w-0 flex-1 ${direction === "vertical" ? "flex flex-col gap-2" : "flex items-center gap-2"}`}
  >
    <p class="sf-label-md text-fg-dim">{title}</p>
    {#if description}
      <div class="flex flex-col">
        {#each description.split("\n") as line (line)}
          <p class="sf-label-sm text-fg-dim">{line}</p>
        {/each}
      </div>
    {/if}
    {#if children}
      <div class="w-full">
        {@render children()}
      </div>
    {/if}
  </div>
  {#if control}
    <div class="flex shrink-0 items-center justify-start gap-2 sm:justify-end">
      {@render control()}
    </div>
  {/if}
</div>
