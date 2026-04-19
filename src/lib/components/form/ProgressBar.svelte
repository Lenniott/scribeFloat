<script lang="ts">
	type ProgressStep = {
		label: string;
		complete?: boolean;
	};
	type SequenceMode = "window" | "current";
	type UiSize = "sm" | "lg";

	const uiSizeMap: Record<
		UiSize,
		{
			ring: number;
			stroke: number;
			layoutGap: string;
			percentText: string;
			listText: string;
			itemGap: string;
			box: string;
			check: string;
		}
	> = {
		sm: {
			ring: 80,
			stroke: 8,
			layoutGap: "gap-4",
			percentText: "text-base",
			listText: "text-label-md",
			itemGap: "gap-2",
			box: "h-2 w-2",
			check: "text-xs",
		},
		lg: {
			ring: 220,
			stroke: 12,
			layoutGap: "gap-7",
			percentText: "text-3xl",
			listText: "text-body-md",
			itemGap: "gap-3.5",
			box: "h-5 w-5",
			check: "text-lg",
		},
	};

	let {
		progress = 0,
		sequence = [],
		size,
		strokeWidth,
		sequenceMode = "window",
		uiSize = "lg",
		class: className = "",
	}: {
		progress?: number;
		sequence?: ProgressStep[] | null;
		size?: number;
		strokeWidth?: number;
		sequenceMode?: SequenceMode;
		uiSize?: UiSize;
		class?: string;
	} = $props();

	const visualSize = $derived(uiSizeMap[uiSize]);
	const ringSize = $derived(size ?? visualSize.ring);
	const ringStroke = $derived(strokeWidth ?? visualSize.stroke);

	const radius = $derived((ringSize - ringStroke) / 2);
	const circumference = $derived(2 * Math.PI * radius);
	const safeProgress = $derived(Math.max(0, Math.min(100, progress)));
	const dashOffset = $derived(circumference * (1 - safeProgress / 100));
	const progressColor = $derived(`var(--color-primary)`);
	const sequenceList = $derived(Array.isArray(sequence) ? sequence : []);
	const hasSequence = $derived(sequenceList.length > 0);
	const currentStageIndex = $derived(
		!hasSequence
			? -1
			: sequenceList.findIndex((step) => !step.complete) === -1
				? sequenceList.length - 1
				: sequenceList.findIndex((step) => !step.complete),
	);
	const currentStage = $derived(currentStageIndex >= 0 ? sequenceList[currentStageIndex] : null);
	const windowSize = 4;
	const windowStart = $derived(
		!hasSequence ? 0 : Math.min(Math.max(currentStageIndex - 1, 0), Math.max(sequenceList.length - windowSize, 0)),
	);
	const windowedSequence = $derived(!hasSequence ? [] : sequenceList.slice(windowStart, windowStart + windowSize));
	const hasTopOverflow = $derived(hasSequence && windowStart > 0);
	const hasBottomOverflow = $derived(hasSequence && windowStart + windowSize < sequenceList.length);
</script>

<div class={`flex items-center ${visualSize.layoutGap} ${className}`.trim()}>
	<div class="relative grid place-items-center" style={`width: ${ringSize}px; height: ${ringSize}px;`}>
		<svg class="h-full w-full -rotate-90" viewBox={`0 0 ${ringSize} ${ringSize}`} role="img" aria-label={`Progress ${safeProgress}%`}>
			<circle
				class="fill-none stroke-white/25"
				cx={ringSize / 2}
				cy={ringSize / 2}
				r={radius}
				stroke-width={ringStroke}
			/>
			<circle
				class="fill-none stroke-linecap-round transition-[stroke,stroke-dashoffset] duration-200 ease-out"
				cx={ringSize / 2}
				cy={ringSize / 2}
				r={radius}
				stroke-width={ringStroke}
				stroke-dasharray={circumference}
				stroke-dashoffset={dashOffset}
				style={`stroke: ${progressColor};`}
			/>
		</svg>
		<div class={`pointer-events-none absolute font-data font-medium text-on-surface ${visualSize.percentText}`}>
			{Math.round(safeProgress)}%
		</div>
	</div>

	{#if hasSequence}
		{#if sequenceMode === "current"}
			<p class={`m-0 font-medium tracking-[0.01em] w-11 text-on-surface flex justify-end ${visualSize.listText}`}>{currentStage?.label}</p>
		{:else}
			<ul class={`m-0 grid list-none gap-2 overflow-hidden p-0 text-on-surface ${visualSize.listText}`} aria-label="Progress sequence">
				{#each windowedSequence as step, index (`${step.label}-${index}`)}
					{@const faded = (index === 0 && hasTopOverflow) || (index === windowedSequence.length - 1 && hasBottomOverflow)}
					<li class={`flex items-center ${visualSize.itemGap} rounded-sm px-1 py-0.5 transition-opacity ${faded ? "opacity-45" : "opacity-100"}`}>
						<span
							class={`relative inline-flex shrink-0 items-center justify-center rounded-md border ${visualSize.box} ${
								step.complete
									? "border-primary bg-primary text-on-primary"
									: "border-white/85 bg-transparent text-transparent"
							}`}
							aria-hidden="true"
						>
							<span class={`leading-none font-bold ${visualSize.check}`}>✓</span>
						</span>
						<span class="tracking-[0.01em]">{step.label}</span>
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</div>
