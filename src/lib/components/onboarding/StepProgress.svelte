<script lang="ts">
	// currentStep is 2–5; step 1 (Welcome) shows no progress dots
	let { currentStep }: { currentStep: number } = $props();
	const totalSteps = 4;
	const stepIndex = $derived(currentStep - 2); // 0-based
</script>

<div
	class="flex items-center gap-1.5"
	role="progressbar"
	aria-valuenow={stepIndex + 1}
	aria-valuemax={totalSteps}
	aria-label={`Step ${stepIndex + 1} of ${totalSteps}`}
>
	{#each Array(totalSteps) as _, i}
		<div
			class={[
				"rounded-full transition-all duration-300 ease-out",
				i < stepIndex
					? "w-2 h-2 bg-brand opacity-60"
					: i === stepIndex
						? "w-3 h-3 bg-brand"
						: "w-2 h-2 bg-rim",
			].join(" ")}
		></div>
	{/each}
</div>
