<script lang="ts">
  export type StackProgressStep = {
    label: string;
    complete?: boolean;
  };
  type Variant = "small" | "large";

  const variants: Record<
    Variant,
    {
      rootGap: string;
      barHeight: string;
      barWidth: string;
      blockCount: number;
      showCurrentStep: boolean;
      showSequence: boolean;
      showPercent: boolean;
      padding: string;
      blockWidth: string;
    }
  > = {
    small: {
      rootGap: "gap-2",
      barHeight: "h-8",
      barWidth: "118px",
      blockCount: 28,
      showCurrentStep: true,
      showSequence: false,
      showPercent: false,
      padding: "p-1",
      blockWidth: "0.125rem",
    },
    large: {
      rootGap: "gap-5",
      barHeight: "h-14",
      barWidth: "454px",
      blockCount: 44,
      showCurrentStep: false,
      showSequence: true,
      showPercent: true,
      padding: "p-2",
      blockWidth: "0.5rem",
    },
  };

  let {
    progress = 0,
    sequence = [],
    variant = "large",
    blockCount,
    barWidth,
    blockGap = "0.125rem",
    class: className = "",
  }: {
    progress?: number;
    sequence?: StackProgressStep[];
    variant?: Variant;
    blockCount?: number;
    barWidth?: string;
    blockWidth?: string;
    blockGap?: string;
    class?: string;
  } = $props();

  const safeProgress = $derived(Math.max(0, Math.min(100, progress)));
  const variantConfig = $derived(variants[variant] ?? variants.large);
  const resolvedBlockCount = $derived(blockCount ?? variantConfig.blockCount);
  const resolvedBarWidth = $derived(barWidth ?? variantConfig.barWidth);
  const blocks = $derived(
    Array.from(
      { length: Math.max(1, resolvedBlockCount) },
      (_, index) => index,
    ),
  );
  const activeBlockCount = $derived(
    Math.round((safeProgress / 100) * blocks.length),
  );
  const currentStep = $derived(
    sequence.find((step) => !step.complete) ??
      sequence[sequence.length - 1] ??
      null,
  );
</script>

<div
  class={`flex items-center ${variantConfig.rootGap} ${className} w-full`.trim()}
  role="status"
  aria-label={`Processing ${Math.round(safeProgress)}% complete`}
>
  <div
    class={`flex relative ${variantConfig.barHeight} rounded-xs bg-surface-low ${variantConfig.padding}`}
    style={`width: ${resolvedBarWidth}; gap: ${blockGap};`}
  >
    {#each blocks as block (block)}
      <span
        class={`h-full shrink-0 transition-colors duration-200 ${
          block < activeBlockCount
            ? "bg-on-surface"
            : "bg-surface-highest"
        }`}
        style={`width: ${variantConfig.blockWidth};`}
        aria-hidden="true"
      ></span>
    {/each}
    {#if variantConfig.showPercent}
      <p
        class="absolute -bottom-6 left-0 font-mono text-label-sm tracking-stamped text-on-surface/55"
      >
        {Math.round(safeProgress)}%
      </p>
    {/if}
  </div>
  {#if variantConfig.showSequence && sequence.length > 0}
    <div class="flex flex-col items-start justify-between gap-3">
      {#each sequence as step (step.label)}
        <div class="flex min-w-0 flex-1 items-center gap-2">
          <span
            class={`grid size-5 shrink-0 place-items-center rounded border text-label-sm font-normal ${
              step.complete
                ? "border-on-surface bg-on-surface text-void"
                : "border-on-surface/70 text-transparent"
            }`}
            aria-hidden="true"
          >
            ✓
          </span>
          <span
            class={`truncate text-label-md ${step.complete ? "text-on-surface" : "text-on-surface/55"}`}
          >
            {step.label}
          </span>
        </div>
      {/each}
    </div>
  {/if}
  {#if variantConfig.showCurrentStep && currentStep}
  <p class="m-0 ml-auto truncate text-label-md font-normal text-on-surface">
	{currentStep.label}
  </p>
{/if}
</div>
