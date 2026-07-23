import { describe, expect, it } from 'vitest';
import {
	batchProcessingFeedback,
	DICTATE_STEPS,
	processingFeedback,
	RECORD_STEPS,
	UPLOAD_STEPS,
} from './processingFeedback';

describe('processingFeedback', () => {
	it('is indeterminate while the model loads and the first step stays incomplete', () => {
		const fb = processingFeedback('LOADING_MODEL', 0, RECORD_STEPS);
		expect(fb.indeterminate).toBe(true);
		expect(fb.sequence[0]).toEqual({ label: 'Loading model', complete: false });
	});

	it('maps transcribing into its stage band — Record: 10 to 90', () => {
		// Backend reserves the first 5% of raw progress for model loading; the
		// display strips that once and scrubs the transcribing band.
		expect(processingFeedback('TRANSCRIBING_AUDIO', 0.05, RECORD_STEPS).percent).toBe(10);
		expect(processingFeedback('TRANSCRIBING_AUDIO', 0.525, RECORD_STEPS).percent).toBe(50);
		expect(processingFeedback('TRANSCRIBING_AUDIO', 1, RECORD_STEPS).percent).toBe(90);
	});

	it('gives the last step the remainder of the bar — Dictate transcribing ends at 100', () => {
		expect(processingFeedback('TRANSCRIBING_AUDIO', 0.05, DICTATE_STEPS).percent).toBe(10);
		expect(processingFeedback('TRANSCRIBING_AUDIO', 1, DICTATE_STEPS).percent).toBe(100);
	});

	it('exposes fractional percentExact for smooth progress visuals', () => {
		const fb = processingFeedback('TRANSCRIBING_AUDIO', 0.525, RECORD_STEPS);
		expect(fb.percentExact).toBeCloseTo(50, 5);
		expect(fb.percent).toBe(50);
	});

	it('is determinate from the first transcribing tick — the band start is visible fill', () => {
		expect(processingFeedback('TRANSCRIBING_AUDIO', 0.05, RECORD_STEPS).indeterminate).toBe(
			false,
		);
		expect(processingFeedback('TRANSCRIBING_AUDIO', 0.3, RECORD_STEPS).indeterminate).toBe(false);
	});

	it('parks stages without measurable progress at their band start', () => {
		// Scribe emits a constant raw 1.0 while writing; the bar must not snap
		// to 100 while work continues.
		expect(processingFeedback('WRITING_TRANSCRIPT', 1, RECORD_STEPS).percent).toBe(90);
	});

	it('shows 100 once the capture moves past the last profiled step', () => {
		expect(processingFeedback('CLEANING_UP_AUDIO', 1, RECORD_STEPS).percent).toBe(100);
	});

	it('never moves backwards across stage transitions', () => {
		const endOfTranscribe = processingFeedback('TRANSCRIBING_AUDIO', 1, RECORD_STEPS);
		const startOfWriting = processingFeedback('WRITING_TRANSCRIPT', 1, RECORD_STEPS);
		expect(startOfWriting.percentExact).toBeGreaterThanOrEqual(endOfTranscribe.percentExact);
	});

	it('marks only strictly earlier steps complete — the running stage is not done', () => {
		const fb = processingFeedback('TRANSCRIBING_AUDIO', 0.5, UPLOAD_STEPS);
		expect(fb.sequence.map((s) => s.complete)).toEqual([true, true, false, false]);
	});

	it('upload names audio decode honestly — preparing precedes loading in its steps', () => {
		const fb = processingFeedback('PREPARING_AUDIO', 0, UPLOAD_STEPS);
		expect(fb.sequence.map((s) => s.label)).toEqual([
			'Preparing audio',
			'Loading model',
			'Transcribing',
			'Writing transcript',
		]);
		expect(fb.percent).toBe(0);
	});

	it('a preparing stage outside the profile floors at zero (Record/Dictate)', () => {
		expect(processingFeedback('PREPARING_AUDIO', 0, RECORD_STEPS).percent).toBe(0);
		expect(processingFeedback('PREPARING_AUDIO', 0, DICTATE_STEPS).percent).toBe(0);
	});

	it('completes writing only once cleanup begins', () => {
		const writing = processingFeedback('WRITING_TRANSCRIPT', 0.98, RECORD_STEPS);
		expect(writing.sequence.map((s) => s.complete)).toEqual([true, true, false]);
		expect(writing.indeterminate).toBe(false);
		const cleanup = processingFeedback('CLEANING_UP_AUDIO', 1, RECORD_STEPS);
		expect(cleanup.sequence.map((s) => s.complete)).toEqual([true, true, true]);
	});

	it('gives Dictate its two-step subset of the same vocabulary', () => {
		const fb = processingFeedback('TRANSCRIBING_AUDIO', 0.5, DICTATE_STEPS);
		expect(fb.sequence.map((s) => s.label)).toEqual(['Loading model', 'Transcribing']);
	});

	it('clamps out-of-range raw progress to the stage band', () => {
		expect(processingFeedback('TRANSCRIBING_AUDIO', 1.4, RECORD_STEPS).percent).toBe(90);
		expect(processingFeedback('TRANSCRIBING_AUDIO', -0.2, RECORD_STEPS).percent).toBe(10);
	});
});

describe('batchProcessingFeedback', () => {
	it('maps the queue average straight to percent — batch progress is already monotonic', () => {
		const fb = batchProcessingFeedback('TRANSCRIBING_AUDIO', 0.5, UPLOAD_STEPS);
		expect(fb.percent).toBe(50);
		expect(fb.percentExact).toBeCloseTo(50, 5);
	});

	it('is indeterminate only before any item has progressed', () => {
		expect(batchProcessingFeedback('LOADING_MODEL', 0, UPLOAD_STEPS).indeterminate).toBe(true);
		// A later item re-entering LOADING_MODEL must not restart the wave mid-queue.
		expect(batchProcessingFeedback('LOADING_MODEL', 0.5, UPLOAD_STEPS).indeterminate).toBe(
			false,
		);
	});

	it("derives the step sequence from the current item's stage", () => {
		const fb = batchProcessingFeedback('WRITING_TRANSCRIPT', 0.9, UPLOAD_STEPS);
		expect(fb.sequence.map((s) => s.complete)).toEqual([true, true, true, false]);
	});
});
