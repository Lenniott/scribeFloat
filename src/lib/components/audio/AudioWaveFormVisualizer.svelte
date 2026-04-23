<script lang="ts">
	import { onMount } from "svelte";

	export type StackBlockSize = "normal" | "small";

	type Preset = {
		width: number;
		height: number;
		sensitivity: number;
		density: number;
		reach: number;
	};

	// Presets from approved tuning snapshots.
	const PRESETS: Record<StackBlockSize, Preset> = {
		small: {
			width: 40 * 2.8,
			height: 40 * 0.72,
			sensitivity: 0.8,
			density: 30,
			reach: 80,
		},
		normal: {
			width: 140 * 2.8,
			height: 140 * 0.72,
			sensitivity: 0.75,
			density: 63,
			reach: 90,
		},
	};

	let {
		micLevel = 0,
		speakerLevel = 0,
		speakerEnabled = true,
		size = "normal",
		class: className = "",
	}: {
		micLevel?: number;
		speakerLevel?: number;
		speakerEnabled?: boolean;
		size?: StackBlockSize;
		class?: string;
	} = $props();

	let canvas: HTMLCanvasElement;
	let wrap: HTMLDivElement;
	let ctx: CanvasRenderingContext2D | null = null;
	let W = 0;
	let H = 0;
	let dpr = 1;
	let raf = 0;
	let t0 = 0;
	let smoothMic = 0;
	let smoothSpk = 0;
	let palette = $state({ primary: "", tertiary: "" });

	function currentPreset(): Preset {
		return PRESETS[size] ?? PRESETS.normal;
	}

	function syncPaletteFromTheme() {
		if (typeof document === "undefined") return;
		const cs = getComputedStyle(document.documentElement);
		palette.primary = cs.getPropertyValue("--color-primary").trim();
		palette.tertiary = cs.getPropertyValue("--color-tertiary").trim();
	}

	function normalize(level: number): number {
		const cfg = currentPreset();
		const clamped = Math.max(0, Math.min(1, level));
		const gated = Math.max(0, clamped - 0.01);
		const boosted = Math.min(1, gated * 4.2 * Math.max(0.5, Math.min(2.5, cfg.sensitivity)));
		return Math.pow(boosted, 0.52);
	}

	function resize() {
		if (!canvas || !wrap) return;
		syncPaletteFromTheme();
		const rect = wrap.getBoundingClientRect();
		W = rect.width;
		H = rect.height;
		dpr = typeof window !== "undefined" ? Math.min(2, window.devicePixelRatio || 1) : 1;
		canvas.width = Math.max(1, Math.floor(W * dpr));
		canvas.height = Math.max(1, Math.floor(H * dpr));
		canvas.style.width = `${W}px`;
		canvas.style.height = `${H}px`;
		ctx = canvas.getContext("2d");
		if (ctx) ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
	}

	function drawSingleBottomStacks(level: number, color: string) {
		if (!ctx) return;
		const cfg = currentPreset();
		const bars = 46;
		const gap = 2;
		const barW = Math.max(1, (W - (bars - 1) * gap) / bars);
		const density = Math.max(0, Math.min(100, cfg.density));
		const reach = Math.max(0, Math.min(100, cfg.reach));
		const minLayers = 2 + Math.round((density / 100) * 8);
		const maxExtraLayers = 8 + Math.round((density / 100) * 32);
		const maxReachPx = H * (0.3 + (reach / 100) * 0.7);
		const unitH = Math.max(1, H * 0.022);
		const bottomY = H - 2;

		for (let i = 0; i < bars; i++) {
			const x = i * (barW + gap);
			const p = i / Math.max(1, bars - 1);
			const shape = Math.sin(p * Math.PI * 8 + t0 * 0.004) * 0.2 + 0.8;
			const peaks = Math.sin(p * Math.PI * 22 + t0 * 0.009) * 0.5 + 0.5;
			const local = Math.max(0, Math.min(1, level * shape * (0.7 + peaks * 0.6)));
			const layers = Math.max(minLayers, Math.round(minLayers + local * maxExtraLayers));
			ctx.fillStyle = color;
			ctx.globalAlpha = 1;
			for (let l = 0; l < layers; l++) {
				const y = bottomY - (l + 1) * unitH;
				if (bottomY - y > maxReachPx) break;
				ctx.fillRect(x, y, barW, Math.max(1, unitH - 1));
			}
		}
	}

	function drawSplitBottomStacks(mic: number, spk: number) {
		if (!ctx) return;
		const cfg = currentPreset();
		const bars = 46;
		const gap = 2;
		const slotW = Math.max(2, (W - (bars - 1) * gap) / bars);
		const innerGap = 1;
		const sideW = Math.max(1, (slotW - innerGap) / 2);
		const density = Math.max(0, Math.min(100, cfg.density));
		const reach = Math.max(0, Math.min(100, cfg.reach));
		const minLayers = 2 + Math.round((density / 100) * 8);
		const maxExtraLayers = 8 + Math.round((density / 100) * 32);
		const maxReachPx = H * (0.3 + (reach / 100) * 0.7);
		const unitH = Math.max(1, H * 0.022);
		const bottomY = H - 2;

		for (let i = 0; i < bars; i++) {
			const x = i * (slotW + gap);
			const p = i / Math.max(1, bars - 1);
			const shape = Math.sin(p * Math.PI * 8 + t0 * 0.004) * 0.2 + 0.8;
			const peaks = Math.sin(p * Math.PI * 22 + t0 * 0.009) * 0.5 + 0.5;
			const localMic = Math.max(0, Math.min(1, mic * shape * (0.7 + peaks * 0.6)));
			const localSpk = Math.max(0, Math.min(1, spk * shape * (0.7 + peaks * 0.6)));
			const micLayers = Math.max(minLayers, Math.round(minLayers + localMic * maxExtraLayers));
			const spkLayers = Math.max(minLayers, Math.round(minLayers + localSpk * maxExtraLayers));
			const maxLayers = Math.max(micLayers, spkLayers);

			for (let l = 0; l < maxLayers; l++) {
				const y = bottomY - (l + 1) * unitH;
				if (bottomY - y > maxReachPx) break;
				ctx.globalAlpha = 1;
				if (l < spkLayers) {
					ctx.fillStyle = palette.tertiary;
					ctx.fillRect(x, y, sideW, Math.max(1, unitH - 1));
				}
				if (l < micLayers) {
					ctx.fillStyle = palette.primary;
					ctx.fillRect(x + sideW + innerGap, y, sideW, Math.max(1, unitH - 1));
				}
			}
		}
	}

	function frame(now: number) {
		if (!ctx) return;
		if (!palette.primary || !palette.tertiary) syncPaletteFromTheme();
		t0 = now;
		const mic = normalize(micLevel);
		const spk = speakerEnabled ? normalize(speakerLevel) : 0;
		smoothMic += (mic - smoothMic) * (mic > smoothMic ? 0.35 : 0.14);
		smoothSpk += (spk - smoothSpk) * (spk > smoothSpk ? 0.35 : 0.14);
		ctx.clearRect(0, 0, W, H);

		const micOut = Math.min(1, smoothMic * 1.02);
		const spkOut = Math.min(1, smoothSpk * 1.1);
		if (speakerEnabled && spkOut > 0.02) {
			drawSplitBottomStacks(micOut, spkOut);
		} else {
			drawSingleBottomStacks(micOut, palette.primary);
		}

		raf = requestAnimationFrame(frame);
	}

	onMount(() => {
		resize();
		const ro = new ResizeObserver(() => resize());
		if (wrap) ro.observe(wrap);
		raf = requestAnimationFrame(frame);
		return () => {
			ro.disconnect();
			cancelAnimationFrame(raf);
		};
	});
</script>

<div
	bind:this={wrap}
	class="relative overflow-hidden rounded-md bg-surface-container-low {className}"
	style="width: {currentPreset().width}px; height: {currentPreset().height}px;"
>
	<canvas bind:this={canvas} class="block h-full w-full" aria-hidden="true"></canvas>
</div>
