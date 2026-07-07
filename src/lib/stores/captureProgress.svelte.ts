/**
 * Display progress for one capture run (Record, Dictate, or an Upload batch).
 *
 * The backend's progress signal is sparse and bursty: long silent phases
 * (model load, WAV finalize), then a handful of ticks. Views want the
 * opposite — one smooth fill paced to the real press-to-paste duration.
 *
 * Single-run mode glides on a clock: `begin(hintSeconds)` sets the expected
 * duration (the view estimates it from the recording length), and the value
 * approaches `GLIDE_CEILING` so most of the bar fills over that window. The
 * hint is corrected by a persisted per-flow factor learned from how long runs
 * actually took on this machine (`estimateKey`), so pacing converges on
 * reality. Real backend ticks anchor the value via the stage-band contract —
 * they can only pull it forward — and a terminal tick (raw ≥ 1) or
 * `complete()` sweeps to 100. The glide alone never reaches the ceiling:
 * "done" always comes from a real event.
 *
 * Batch mode (upload queue) has a trustworthy continuous signal — the queue
 * average — so it creeps gently instead: never more than `CREEP_HEADROOM`
 * ahead of the last real number, capped at `MAX_CREEP_PER_SECOND`.
 *
 * Views render `percent` / `percentExact` / `sequence` and call
 * `begin` / `update` / `complete` / `reset`; they must not re-derive progress.
 */
import {
	batchProcessingFeedback,
	processingFeedback,
	stageLabel,
	type ProcessingStage,
	type SequenceStep,
} from '@utils/processingFeedback';

const TICK_MS = 120;
/** The glide's asymptote — only a real terminal event may pass it. */
const GLIDE_CEILING = 97;
/** expected duration / tau — 2.5 puts the glide at ~92% of ceiling at t=expected. */
const GLIDE_TAU_RATIO = 2.5;
const DEFAULT_EXPECTED_SECONDS = 8;
const MIN_EXPECTED_SECONDS = 1;
const MAX_EXPECTED_SECONDS = 300;
/** EMA weight of the newest observed duration/hint ratio. */
const LEARN_ALPHA = 0.35;
const STORAGE_PREFIX = 'sf-capture-eta:';

/** Batch mode: creep never runs further than this ahead of the queue average. */
const CREEP_HEADROOM = 10;
const MAX_CREEP_PER_SECOND = 1.5;
const CREEP_TAU_SECONDS = 4;
/** Stop moving this close to a ceiling so we never visually reach it. */
const CEILING_DEAD_ZONE = 0.05;

function readSpeedFactor(key: string | undefined): number | null {
	if (!key || typeof localStorage === 'undefined') return null;
	const stored = Number(localStorage.getItem(STORAGE_PREFIX + key));
	return Number.isFinite(stored) && stored > 0 ? stored : null;
}

function writeSpeedFactor(key: string | undefined, factor: number): void {
	if (!key || typeof localStorage === 'undefined') return;
	localStorage.setItem(STORAGE_PREFIX + key, String(factor));
}

class CaptureProgress {
	#steps: ProcessingStage[];
	#batch: boolean;
	#estimateKey: string | undefined;
	#stage = $state<ProcessingStage>('LOADING_MODEL');
	#percentExact = $state(0);
	/** Last percent derived from a real backend event — the display's floor. */
	#anchor = 0;
	#timer: ReturnType<typeof setInterval> | undefined;
	#expectedSeconds = DEFAULT_EXPECTED_SECONDS;
	#hintSeconds: number | undefined;
	#elapsedTicks = 0;

	constructor(steps: ProcessingStage[], batch: boolean, estimateKey?: string) {
		this.#steps = steps;
		this.#batch = batch;
		this.#estimateKey = estimateKey;
	}

	get percent(): number {
		return Math.round(this.#percentExact);
	}

	get percentExact(): number {
		return this.#percentExact;
	}

	get sequence(): SequenceStep[] {
		return processingFeedback(this.#stage, 0, this.#steps).sequence;
	}

	/**
	 * True while pre-transcription work blocks the run (audio finalize/decode,
	 * model load) — no bar movement yet; views show "`stageLabel`…" dots
	 * instead of the bar. A mid-queue stage regression does not count: once
	 * there is real progress, it stays a bar.
	 */
	get loading(): boolean {
		return (
			(this.#stage === 'PREPARING_AUDIO' || this.#stage === 'LOADING_MODEL') &&
			this.#percentExact === 0
		);
	}

	/** Human label for the current stage — the text next to the loading dots. */
	get stageLabel(): string {
		return stageLabel(this.#stage);
	}

	/**
	 * Arm pacing for a run whose transcription is expected to take about
	 * `hintSeconds`. The glide clock starts when the run leaves the loading
	 * phase, so model-load time is excluded from pacing and learning.
	 */
	begin(hintSeconds: number): void {
		this.#hintSeconds = hintSeconds;
		this.#elapsedTicks = 0;
		this.#expectedSeconds = Math.max(
			MIN_EXPECTED_SECONDS,
			Math.min(MAX_EXPECTED_SECONDS, hintSeconds * (readSpeedFactor(this.#estimateKey) ?? 1)),
		);
	}

	update(stage: ProcessingStage, rawProgress: number): void {
		this.#stage = stage;
		const feedback = this.#batch ? batchProcessingFeedback : processingFeedback;
		const floor = feedback(stage, rawProgress, this.#steps).percentExact;
		this.#anchor = Math.max(this.#anchor, floor);
		this.#percentExact = Math.max(this.#percentExact, this.#anchor);
		if (this.#anchor >= 100) {
			this.complete();
			return;
		}
		if (!this.loading) {
			this.#startTicker();
		}
	}

	complete(): void {
		this.#learnFromRun();
		this.#stopTicker();
		this.#anchor = 100;
		this.#percentExact = 100;
	}

	reset(): void {
		this.#stopTicker();
		this.#stage = 'LOADING_MODEL';
		this.#anchor = 0;
		this.#percentExact = 0;
		this.#hintSeconds = undefined;
		this.#elapsedTicks = 0;
		this.#expectedSeconds = DEFAULT_EXPECTED_SECONDS;
	}

	/** Fold this run's actual duration into the persisted hint-correction factor. */
	#learnFromRun(): void {
		if (this.#hintSeconds === undefined || !this.#estimateKey) return;
		const actualSeconds = (this.#elapsedTicks * TICK_MS) / 1000;
		if (actualSeconds <= 0) return;
		const runFactor = Math.max(0.2, Math.min(5, actualSeconds / this.#hintSeconds));
		const prior = readSpeedFactor(this.#estimateKey);
		// First observation counts fully — the default of 1 carries no information.
		const blended = prior === null ? runFactor : prior * (1 - LEARN_ALPHA) + runFactor * LEARN_ALPHA;
		writeSpeedFactor(this.#estimateKey, blended);
	}

	#tick(): void {
		this.#elapsedTicks += 1;
		const dt = TICK_MS / 1000;
		if (this.#batch) {
			const ceiling = Math.min(100, this.#anchor + CREEP_HEADROOM);
			const remaining = ceiling - this.#percentExact;
			if (remaining <= CEILING_DEAD_ZONE) return;
			const step = Math.min(remaining * (dt / CREEP_TAU_SECONDS), MAX_CREEP_PER_SECOND * dt);
			this.#percentExact += step;
			return;
		}
		const remaining = GLIDE_CEILING - this.#percentExact;
		if (remaining <= CEILING_DEAD_ZONE) return;
		const tau = this.#expectedSeconds / GLIDE_TAU_RATIO;
		this.#percentExact += remaining * (dt / tau);
	}

	#startTicker(): void {
		if (this.#timer !== undefined) return;
		this.#timer = setInterval(() => this.#tick(), TICK_MS);
	}

	#stopTicker(): void {
		if (this.#timer !== undefined) {
			clearInterval(this.#timer);
			this.#timer = undefined;
		}
	}
}

export function createCaptureProgress(
	steps: ProcessingStage[],
	opts: { batch?: boolean; estimateKey?: string } = {},
): CaptureProgress {
	return new CaptureProgress(steps, opts.batch ?? false, opts.estimateKey);
}

export type { CaptureProgress };
