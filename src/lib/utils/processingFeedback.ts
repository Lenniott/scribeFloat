/**
 * One vocabulary for capture processing feedback, shared by Record, Dictate,
 * and Upload. The backend emits the same `ProcessingStage` strings for all
 * three capture profiles (types.rs); profiles differ only in which steps they
 * display — Dictate has no transcript file or kept audio, so it stops at
 * TRANSCRIBING_AUDIO. Views render the result with ProgressBar and must not
 * re-derive stage order, completion, percent, or indeterminacy themselves.
 */

export type ProcessingStage =
	| 'PREPARING_AUDIO'
	| 'LOADING_MODEL'
	| 'TRANSCRIBING_AUDIO'
	| 'WRITING_TRANSCRIPT'
	| 'CLEANING_UP_AUDIO';

export type SequenceStep = { label: string; complete: boolean };

export type ProcessingFeedback = {
	/** 0–100 rounded, with the backend's model-load headroom stripped. */
	percent: number;
	/** Same scale as `percent` but fractional — for smooth progress visuals. */
	percentExact: number;
	/** True whenever the current stage has no meaningful percent yet. */
	indeterminate: boolean;
	sequence: SequenceStep[];
};

const STAGE_ORDER: ProcessingStage[] = [
	'PREPARING_AUDIO',
	'LOADING_MODEL',
	'TRANSCRIBING_AUDIO',
	'WRITING_TRANSCRIPT',
	'CLEANING_UP_AUDIO',
];

const STAGE_LABELS: Record<ProcessingStage, string> = {
	PREPARING_AUDIO: 'Preparing audio',
	LOADING_MODEL: 'Loading model',
	TRANSCRIBING_AUDIO: 'Transcribing',
	WRITING_TRANSCRIPT: 'Writing transcript',
	CLEANING_UP_AUDIO: 'Cleaning up',
};

export function stageLabel(stage: ProcessingStage): string {
	return STAGE_LABELS[stage];
}

/**
 * The backend reserves the first 5% of raw progress for model loading
 * (model.rs `INFERENCE_MODEL_LOAD_PROGRESS`); strip it once here so the
 * transcribing band scrubs 0→1 the moment inference begins.
 */
const MODEL_LOAD_HEADROOM = 0.05;

/**
 * Display-progress bands. Each stage owns a disjoint slice of 0–100, so the
 * bar is monotonic across the whole capture by construction: entering a stage
 * lands at its band start, measurable progress scrubs within the band, and a
 * stage with no measurable progress parks at its band start instead of
 * snapping to 100. The last step of a profile absorbs the remainder to 100.
 */
const STAGE_WEIGHTS: Record<ProcessingStage, number> = {
	PREPARING_AUDIO: 0,
	LOADING_MODEL: 10,
	TRANSCRIBING_AUDIO: 80,
	WRITING_TRANSCRIPT: 10,
	CLEANING_UP_AUDIO: 0,
};

export const RECORD_STEPS: ProcessingStage[] = [
	'LOADING_MODEL',
	'TRANSCRIBING_AUDIO',
	'WRITING_TRANSCRIPT',
];
export const DICTATE_STEPS: ProcessingStage[] = ['LOADING_MODEL', 'TRANSCRIBING_AUDIO'];
/** Upload decodes each source file before transcribing it — that phase is
 * visible per item, so it gets its own named step. */
export const UPLOAD_STEPS: ProcessingStage[] = [
	'PREPARING_AUDIO',
	'LOADING_MODEL',
	'TRANSCRIBING_AUDIO',
	'WRITING_TRANSCRIPT',
];

/**
 * [start, end] of `stage`'s slice of 0–100 for a given step profile.
 * Exported for the captureProgress store, which creeps toward the band end
 * while the backend is silent; views should not consume bands directly.
 */
export function stageBand(stage: ProcessingStage, steps: ProcessingStage[]): [number, number] {
	let start = 0;
	for (let i = 0; i < steps.length; i++) {
		const width = i === steps.length - 1 ? 100 - start : STAGE_WEIGHTS[steps[i]];
		if (steps[i] === stage) return [start, start + width];
		start += width;
	}
	// Stage past the profile's last step (e.g. cleanup after writing) — done.
	return STAGE_ORDER.indexOf(stage) > STAGE_ORDER.indexOf(steps[steps.length - 1])
		? [100, 100]
		: [0, 0];
}

/** 0–1 progress within `stage`; only transcribing reports a real fraction. */
function stageFraction(stage: ProcessingStage, rawProgress: number): number {
	const clamped = Math.max(0, Math.min(1, rawProgress));
	if (stage === 'TRANSCRIBING_AUDIO') {
		return Math.max(0, clamped - MODEL_LOAD_HEADROOM) / (1 - MODEL_LOAD_HEADROOM);
	}
	if (stage === 'LOADING_MODEL') {
		return clamped;
	}
	// WRITING_TRANSCRIPT / CLEANING_UP_AUDIO emit a constant 1.0 on entry —
	// there is no measurable intra-stage progress, so park at the band start.
	return 0;
}

function sequenceFor(stage: ProcessingStage, steps: ProcessingStage[]): SequenceStep[] {
	const currentIndex = STAGE_ORDER.indexOf(stage);
	return steps.map((step) => ({
		label: STAGE_LABELS[step],
		complete: STAGE_ORDER.indexOf(step) < currentIndex,
	}));
}

export function processingFeedback(
	stage: ProcessingStage,
	rawProgress: number,
	steps: ProcessingStage[],
): ProcessingFeedback {
	const [bandStart, bandEnd] = stageBand(stage, steps);
	const percentExact = bandStart + stageFraction(stage, rawProgress) * (bandEnd - bandStart);
	const percent = Math.round(percentExact);
	const indeterminate = stage === 'LOADING_MODEL';
	return { percent, percentExact, indeterminate, sequence: sequenceFor(stage, steps) };
}

/**
 * Upload-queue variant: `overallProgress` is the backend's average of per-item
 * 0–1 progress, which is already monotonic across the whole batch — mapping it
 * through per-capture stage bands would jump backwards at item boundaries.
 * The stage only drives the step sequence and the initial indeterminate wave.
 */
export function batchProcessingFeedback(
	stage: ProcessingStage,
	overallProgress: number,
	steps: ProcessingStage[],
): ProcessingFeedback {
	const percentExact = Math.max(0, Math.min(1, overallProgress)) * 100;
	const percent = Math.round(percentExact);
	return {
		percent,
		percentExact,
		indeterminate: percentExact === 0,
		sequence: sequenceFor(stage, steps),
	};
}
