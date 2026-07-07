<script lang="ts">
	/** How fast the visual catches up when the target jumps ahead (% per second). */
	const CATCH_UP_SPEED = 90;
	/**
	 * Capture pipelines emit sparse, bursty progress (sometimes just 5% → 100%),
	 * and the bar may unmount shortly after the terminal value arrives — cap the
	 * catch-up time so any jump completes before the UI tears down.
	 */
	const MAX_CATCH_UP_SECONDS = 0.35;

	function clampNumber(value: number, min: number, max: number): number {
		return Math.min(Math.max(value, min), max);
	}

	function cubeScrub(order: number, current: number): { opacity: number; yPercent: number } {
		const fill = Math.min(Math.max(current - order, 0), 1);
		return {
			opacity: fill > 0 ? 1 : 0,
			yPercent: (1 - fill) * -100,
		};
	}

	function prefersReducedMotion(): boolean {
		return (
			typeof window !== 'undefined' &&
			window.matchMedia('(prefers-reduced-motion: reduce)').matches
		);
	}

	let {
		progress = 0,
		indeterminate = false,
		rows = 3,
		columns = 32,
		color = "var(--color-focus)",
		cube = 3,
		gap = 1,
		scale = 1,
		speed = 70,
		fluid = false,
		statusLabel,
		class: className = "",
	}: {
		progress?: number;
		indeterminate?: boolean;
		rows?: number;
		columns?: number;
		color?: string;
		cube?: number;
		gap?: number;
		scale?: number;
		speed?: number;
		/** Fill the parent's width — column count derives from measured space. */
		fluid?: boolean;
		statusLabel?: string;
		class?: string;
	} = $props();

	let hostWidth = $state(0);

	const safeRows = $derived(clampNumber(rows, 1, 24));
	const safeCube = $derived(clampNumber(cube, 2, 40));
	const safeGap = $derived(clampNumber(gap, 0, 24));
	const safeScale = $derived(clampNumber(scale, 1, 20));
	const safeColumns = $derived(
		fluid && hostWidth > 0
			? clampNumber(
					Math.floor((hostWidth + safeGap * safeScale) / ((safeCube + safeGap) * safeScale)),
					1,
					160,
				)
			: clampNumber(columns, 1, 48),
	);
	const targetProgress = $derived(Math.max(0, Math.min(100, progress)));
	const safeProgress = $derived(Math.round(targetProgress));

	const totalCubes = $derived(safeRows * safeColumns);
	const fallDistance = $derived(
		Math.max(24, safeRows * (safeCube + safeGap) * 2),
	);
	const layoutWidth = $derived(
		safeColumns * safeCube + Math.max(0, safeColumns - 1) * safeGap,
	);
	const layoutHeight = $derived(
		safeRows * safeCube + Math.max(0, safeRows - 1) * safeGap,
	);

	/** Eased visual progress — chases `targetProgress` via rAF. */
	let smoothProgress = $state(0);
	/** Leading edge of the indeterminate wave (cube-order units). */
	let indeterminateHead = $state(0);

	const INDETERMINATE_WAVE = 10;

	$effect(() => {
		if (!indeterminate) {
			indeterminateHead = 0;
			return;
		}

		if (prefersReducedMotion()) {
			indeterminateHead = totalCubes * 0.35;
			return;
		}

		let frame = 0;
		let active = true;
		let lastTime = performance.now();
		const cubesPerSecond = 1000 / Math.max(speed, 16);

		function tick(now: number) {
			if (!active) return;

			const dt = Math.min((now - lastTime) / 1000, 0.05);
			lastTime = now;
			indeterminateHead += cubesPerSecond * dt;
			if (indeterminateHead > totalCubes + INDETERMINATE_WAVE) {
				indeterminateHead = 0;
			}
			frame = requestAnimationFrame(tick);
		}

		frame = requestAnimationFrame(tick);
		return () => {
			active = false;
			cancelAnimationFrame(frame);
		};
	});

	$effect(() => {
		if (indeterminate) return;

		const target = targetProgress;

		if (target === 0) {
			smoothProgress = 0;
			return;
		}

		if (prefersReducedMotion()) {
			smoothProgress = target;
			return;
		}

		let frame = 0;
		let active = true;
		let lastTime = performance.now();

		function tick(now: number) {
			if (!active) return;

			const dt = Math.min((now - lastTime) / 1000, 0.05);
			lastTime = now;

			if (smoothProgress < target) {
				const speed = Math.max(
					CATCH_UP_SPEED,
					(target - smoothProgress) / MAX_CATCH_UP_SECONDS,
				);
				smoothProgress = Math.min(target, smoothProgress + speed * dt);
				frame = requestAnimationFrame(tick);
			}
		}

		frame = requestAnimationFrame(tick);
		return () => {
			active = false;
			cancelAnimationFrame(frame);
		};
	});

	const fillPosition = $derived(
		indeterminate ? indeterminateHead : (smoothProgress / 100) * totalCubes,
	);

	const cubes = $derived(
		Array.from({ length: totalCubes }, (_, index) => {
			const row = Math.floor(index / safeColumns);
			const col = index % safeColumns;
			const bottomToTopRow = safeRows - row - 1;
			const order = col * safeRows + bottomToTopRow;
			return { index, order, ...cubeScrub(order, fillPosition) };
		}),
	);

	const ariaValueText = $derived(
		indeterminate
			? (statusLabel ?? "Loading")
			: statusLabel
				? `${statusLabel}, ${safeProgress} percent complete`
				: `Processing, ${safeProgress} percent complete`,
	);
</script>

<div
	class={`cube-loader-host ${fluid ? "is-fluid" : ""} ${className}`.trim()}
	style:width={fluid ? "100%" : `${layoutWidth * safeScale}px`}
	style:height="{layoutHeight * safeScale}px"
	bind:clientWidth={hostWidth}
>
	<div
		class="cube-loader is-scrubbing"
		role="progressbar"
		aria-valuemin={indeterminate ? undefined : 0}
		aria-valuemax={indeterminate ? undefined : 100}
		aria-valuenow={indeterminate ? undefined : safeProgress}
		aria-valuetext={ariaValueText}
		aria-busy={indeterminate ? true : undefined}
		aria-label={ariaValueText}
		style:--cols={safeColumns}
		style:--cube-color={color}
		style:--cube-size="{safeCube}px"
		style:--cube-gap="{safeGap}px"
		style:--loader-scale={safeScale}
		style:--fall-distance="{fallDistance}px"
		style:--stagger="{speed}ms"
	>
		{#each cubes as { index, order, opacity, yPercent } (index)}
			<span
				class="cube-loader__cube"
				style:--cube-opacity={opacity}
				style:--cube-y="calc(var(--fall-distance) * {yPercent / 100})"
				data-order={order}
				aria-hidden="true"
			></span>
		{/each}
	</div>
</div>

<style>
	.cube-loader-host {
		position: relative;
		display: inline-block;
		vertical-align: middle;
		overflow: visible;
	}

	.cube-loader-host.is-fluid {
		display: block;
	}

	.cube-loader {
		position: absolute;
		top: 0;
		left: 0;
		display: grid;
		grid-template-columns: repeat(var(--cols), var(--cube-size));
		grid-auto-rows: var(--cube-size);
		gap: var(--cube-gap);
		transform: scale(var(--loader-scale));
		transform-origin: top left;
	}

	.cube-loader__cube {
		width: var(--cube-size);
		height: var(--cube-size);
		background: var(--cube-color);
	}

	.cube-loader.is-scrubbing .cube-loader__cube {
		animation: none;
		opacity: var(--cube-opacity, 0);
		transform: translateY(var(--cube-y, calc(var(--fall-distance) * -1)));
		transition:
			transform 100ms linear,
			opacity 60ms linear;
	}

	@media (prefers-reduced-motion: reduce) {
		.cube-loader.is-scrubbing .cube-loader__cube {
			transition: none;
		}
	}
</style>
