<script lang="ts">
	import type { Snippet } from "svelte";
	import AudioLayerLegend from "./AudioLayerLegend.svelte";
	import DualRadialWaveform from "./DualRadialWaveform.svelte";

	let {
		micLevel = 0,
		speakerLevel = 0,
		speakerEnabled = true,
		showLegend = true,
		size = 220,
		innerBaseScale,
		outerScale,
		ampInner,
		ampOuter,
		children,
	}: {
		micLevel?: number;
		speakerLevel?: number;
		speakerEnabled?: boolean;
		showLegend?: boolean;
		size?: number;
		/** Passed to `DualRadialWaveform` — tune ring size vs canvas (see that component’s prop docs). */
		innerBaseScale?: number;
		outerScale?: number;
		ampInner?: number;
		ampOuter?: number;
		children?: Snippet;
	} = $props();
</script>

<div class="flex w-full flex-col items-center gap-3">
	<div class="relative shrink-0" style="width: {size}px; height: {size}px;">
		<DualRadialWaveform
			class="absolute inset-0 h-full w-full"
			{micLevel}
			{speakerLevel}
			{speakerEnabled}
			{innerBaseScale}
			{outerScale}
			{ampInner}
			{ampOuter}
		/>

		{#if children}
			<div class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center">
				<div class="pointer-events-auto">
					{@render children?.()}
				</div>
			</div>
		{/if}
	</div>

	{#if showLegend}
	<div class="flex w-full shrink-0 justify-center pt-0.5">
		<AudioLayerLegend {speakerEnabled} />
	</div>
	{/if}
</div>
