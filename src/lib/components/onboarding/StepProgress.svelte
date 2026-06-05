<script lang="ts">
	// currentStep is 2–8; step 1 (Welcome) shows no progress dots
	let { currentStep }: { currentStep: number } = $props();
	const totalSteps = 7;
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
				"rounded-full transition-all duration-200",
				i < stepIndex
					? "w-2 h-2 bg-brand"
					: i === stepIndex
						? "w-2.5 h-2.5 bg-brand"
						: "w-2 h-2 bg-rim",
			].join(" ")}
		></div>
	{/each}
</div>
