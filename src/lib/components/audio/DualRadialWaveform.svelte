<script lang="ts">
	import { onMount } from "svelte";

	/** Polar waveform: inner = responsive (mic), outer = smoothed (speaker). Colors from context/DESIGN.md (primary / tertiary). */

	let {
		micLevel = 0,
		speakerLevel = 0,
		speakerEnabled = true,
		class: className = "",
		/**
		 * Geometry vs `s = min(canvas width, height)` (CSS px):
		 * - `innerBaseScale`: mean radius of the inner (mic) ring — **raise this** to make both rings larger in the frame.
		 * - `outerScale`: outer mean radius = inner × this — spacing between rings.
		 * - `ampInner` / `ampOuter`: radial wobble depth (fraction of `s`) — adds “spikes”; also uses edge space.
		 * Keep `innerBaseScale + ampInner` (and outer similarly) below ~0.48 so peaks stay inside the canvas.
		 */
		innerBaseScale = 0.28,
		outerScale = 1.5,
		ampInner = 0.12,
		ampOuter = 0.14,
	}: {
		micLevel?: number;
		speakerLevel?: number;
		speakerEnabled?: boolean;
		class?: string;
		innerBaseScale?: number;
		outerScale?: number;
		ampInner?: number;
		ampOuter?: number;
	} = $props();

	let canvas: HTMLCanvasElement;
	let wrap: HTMLDivElement;

	const BINS = 128;
	const FAKE_FREQ_LEN = 1024;

	const CFG = {
		roundness: "round" as CanvasLineJoin,
	};

	/** Opacity only — hues come from @theme (`app.css`). */
	const WAVE_ALPHA = {
		innerStroke: 0.9,
		innerFill: 0.22,
		outerStroke: 0.5,
		outerFill: 0.16,
	} as const;

	/** Resolved from CSS each resize — single source of truth: `--color-*` in `app.css`. */
	let palette = $state({
		primary: "",
		tertiary: "",
		outline: "",
	});

	function syncPaletteFromTheme() {
		if (typeof document === "undefined") return;
		const cs = getComputedStyle(document.documentElement);
		palette.primary = cs.getPropertyValue("--color-primary").trim();
		palette.tertiary = cs.getPropertyValue("--color-tertiary").trim();
		palette.outline = cs.getPropertyValue("--color-outline-variant").trim();
	}

	let W = 0;
	let H = 0;
	let ctx: CanvasRenderingContext2D | null = null;
	let raf = 0;
	let idleT = 0;
	let t0 = 0;
	const smoothB = new Float64Array(BINS);
	let dpr = 1;

	function lerp(a: number, b: number, t: number): number {
		return a + (b - a) * t;
	}

	function idlePolar(offset = 0): Float64Array {
		const out = new Float64Array(BINS);
		for (let i = 0; i < BINS; i++) {
			out[i] = (Math.sin((i / BINS) * Math.PI * 4 + idleT + offset) * 0.5 + 0.5) * 0.08;
		}
		return out;
	}

	/** Same mapping as the reference `getPolarBins`, with a synthetic byte spectrum scaled by level. */
	function getSyntheticPolarBins(level: number, time: number, phase: number): Float64Array {
		const raw = new Uint8Array(FAKE_FREQ_LEN);
		const L = Math.max(0, Math.min(1, level)) * 255;
		for (let k = 0; k < FAKE_FREQ_LEN; k++) {
			const tn = k / FAKE_FREQ_LEN;
			const wobble =
				Math.sin(tn * Math.PI * 24 + time * 2.8 + phase) * 0.38 +
				Math.sin(tn * Math.PI * 48 + time * 1.9 + phase * 1.4) * 0.32 +
				Math.sin(tn * Math.PI * 12 + time * 1.1) * 0.3;
			raw[k] = Math.max(0, Math.min(255, (wobble * 0.5 + 0.5) * L));
		}
		const maxBin = Math.floor(raw.length * 0.75);
		const out = new Float64Array(BINS);
		for (let i = 0; i < BINS; i++) {
			const t = i / (BINS - 1);
			const idx = Math.min(Math.floor(Math.pow(maxBin, t)), raw.length - 1);
			const spread = Math.max(1, Math.floor(idx * 0.04));
			let sum = 0;
			for (let k = idx; k < Math.min(idx + spread, raw.length); k++) sum += raw[k];
			out[i] = sum / spread / 255;
		}
		return out;
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
		if (ctx) {
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		}
	}

	function radialPath(cx: number, cy: number, baseR: number, amp: number, data: Float64Array | Uint8Array) {
		if (!ctx) return;
		ctx.lineJoin = CFG.roundness;
		ctx.beginPath();
		for (let i = 0; i <= BINS; i++) {
			const idx = i % BINS;
			const ang = (idx / BINS) * Math.PI * 2 - Math.PI / 2;
			const r = baseR + data[idx] * amp;
			const x = cx + Math.cos(ang) * r;
			const y = cy + Math.sin(ang) * r;
			if (i === 0) ctx.moveTo(x, y);
			else ctx.lineTo(x, y);
		}
		ctx.closePath();
	}

	const SMOOTH_OUTER = 0.92;

	function drawFrame(now: number) {
		if (!ctx || W < 1 || H < 1) return;
		if (!palette.primary || !palette.tertiary) { syncPaletteFromTheme(); return; }
		const time = (now - t0) / 1000;
		const cx = W / 2;
		const cy = H / 2;
		const s = Math.min(W, H);

		const baseA = s * innerBaseScale;
		const ampA = s * ampInner;
		const baseB = baseA * outerScale;
		const ampB = s * ampOuter;

		const mic = Math.max(0, Math.min(1, micLevel));
		const spk = speakerEnabled ? Math.max(0, Math.min(1, speakerLevel)) : 0;

		const running = mic > 0.04 || (speakerEnabled && spk > 0.04);
		if (!running) idleT += 0.016;

		const dA = running ? getSyntheticPolarBins(mic, time, 0) : idlePolar(0);

		const targetB =
			running && speakerEnabled ? getSyntheticPolarBins(spk, time, 1.2) : idlePolar(1.2);
		for (let i = 0; i < BINS; i++) {
			smoothB[i] = lerp(smoothB[i], targetB[i], 1 - SMOOTH_OUTER);
		}

		ctx.clearRect(0, 0, W, H);

		const p = palette.primary;
		const t = palette.tertiary;
		const o = palette.outline;

		if (speakerEnabled) {
			radialPath(cx, cy, baseB, ampB, smoothB);
			ctx.globalAlpha = WAVE_ALPHA.outerFill;
			ctx.fillStyle = t;
			ctx.fill();
			ctx.globalAlpha = WAVE_ALPHA.outerStroke;
			ctx.strokeStyle = t;
			ctx.lineWidth = 1.5;
			ctx.stroke();
			ctx.globalAlpha = 1;

			const spokeN = 48;
			for (let i = 0; i < spokeN; i++) {
				const ang = (i / spokeN) * Math.PI * 2 - Math.PI / 2;
				const idxA = Math.floor((i / spokeN) * BINS);
				const idxB = Math.floor((i / spokeN) * BINS);
				const rA = baseA + dA[idxA] * ampA;
				const rB = baseB + smoothB[idxB] * ampB;
				ctx.strokeStyle = o || "rgb(255 255 255 / 0.15)";
				ctx.lineWidth = 0.5;
				ctx.beginPath();
				ctx.moveTo(cx + Math.cos(ang) * rA, cy + Math.sin(ang) * rA);
				ctx.lineTo(cx + Math.cos(ang) * rB, cy + Math.sin(ang) * rB);
				ctx.stroke();
			}
		}

		radialPath(cx, cy, baseA, ampA, dA);
		ctx.globalAlpha = WAVE_ALPHA.innerFill;
		ctx.fillStyle = p;
		ctx.fill();
		ctx.globalAlpha = WAVE_ALPHA.innerStroke;
		ctx.strokeStyle = p;
		ctx.lineWidth = 2;
		ctx.stroke();
		ctx.globalAlpha = 1;
	}

	function loop(now: number) {
		drawFrame(now);
		raf = requestAnimationFrame(loop);
	}

	onMount(() => {
		t0 = performance.now();
		for (let i = 0; i < BINS; i++) smoothB[i] = 0.03;
		resize();
		const ro = new ResizeObserver(() => resize());
		if (wrap) ro.observe(wrap);
		raf = requestAnimationFrame(loop);
		return () => {
			ro.disconnect();
			cancelAnimationFrame(raf);
		};
	});

</script>

<div bind:this={wrap} class="absolute inset-0 min-h-0 min-w-0 {className}">
	<canvas bind:this={canvas} class="block h-full w-full" aria-hidden="true"></canvas>
</div>
