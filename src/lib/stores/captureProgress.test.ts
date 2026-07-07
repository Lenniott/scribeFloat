import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createCaptureProgress } from './captureProgress.svelte';
import { DICTATE_STEPS, RECORD_STEPS, UPLOAD_STEPS } from '@utils/processingFeedback';

describe('createCaptureProgress', () => {
	beforeEach(() => {
		vi.useFakeTimers();
		localStorage.clear();
	});
	afterEach(() => {
		vi.useRealTimers();
	});

	it('starts at zero', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		expect(capture.percent).toBe(0);
	});

	it('anchors to the stage-band floor on real backend progress', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.update('TRANSCRIBING_AUDIO', 0.525);
		expect(capture.percent).toBe(50);
	});

	it('never moves backwards, even when raw progress regresses', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.update('TRANSCRIBING_AUDIO', 0.525);
		capture.update('TRANSCRIBING_AUDIO', 0.2);
		expect(capture.percent).toBe(50);
	});

	it('is a distinct loading phase while the model loads — no bar movement, loading flag on', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.begin(2);
		capture.update('LOADING_MODEL', 0);
		vi.advanceTimersByTime(10_000);
		expect(capture.loading).toBe(true);
		expect(capture.percentExact).toBe(0);
	});

	it('audio finalization is part of the waiting phase, labelled truthfully', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.begin(2);
		capture.update('PREPARING_AUDIO', 0);
		vi.advanceTimersByTime(10_000);
		expect(capture.loading).toBe(true);
		expect(capture.percentExact).toBe(0);
		expect(capture.stageLabel).toBe('Preparing audio');
		capture.update('LOADING_MODEL', 0);
		expect(capture.loading).toBe(true);
		expect(capture.stageLabel).toBe('Loading model');
	});

	it('leaves the loading phase and starts gliding at the first transcribe tick', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.begin(2);
		capture.update('LOADING_MODEL', 0);
		vi.advanceTimersByTime(10_000);
		capture.update('TRANSCRIBING_AUDIO', 0.05);
		expect(capture.loading).toBe(false);
		const start = capture.percentExact;
		vi.advanceTimersByTime(1_000);
		expect(capture.percentExact).toBeGreaterThan(start);
	});

	it('never claims done on its own — glide stalls below 97 without a terminal event', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.begin(2);
		capture.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(120_000);
		expect(capture.percentExact).toBeLessThan(97);
	});

	it('paces the glide to the expected duration — most of the bar fills over that window', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.begin(4);
		capture.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(4_000);
		expect(capture.percentExact).toBeGreaterThan(70);
		expect(capture.percentExact).toBeLessThan(97);
	});

	it('a shorter estimate fills faster', () => {
		const quick = createCaptureProgress(RECORD_STEPS);
		const slow = createCaptureProgress(RECORD_STEPS);
		quick.begin(2);
		slow.begin(20);
		quick.update('TRANSCRIBING_AUDIO', 0.05);
		slow.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(1_000);
		expect(quick.percentExact).toBeGreaterThan(slow.percentExact);
	});

	it('learns machine speed across runs — a machine faster than the hint fills faster next time', () => {
		const first = createCaptureProgress(RECORD_STEPS, { estimateKey: 'test-learn' });
		first.begin(10);
		first.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(2_000);
		first.complete(); // finished in 2s against a 10s hint

		const second = createCaptureProgress(RECORD_STEPS, { estimateKey: 'test-learn' });
		const fresh = createCaptureProgress(RECORD_STEPS, { estimateKey: 'test-fresh' });
		second.begin(10);
		fresh.begin(10);
		second.update('TRANSCRIBING_AUDIO', 0.05);
		fresh.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(1_000);
		expect(second.percentExact).toBeGreaterThan(fresh.percentExact + 15);
	});

	it('learns from transcription time only — waiting on the model load does not skew the estimate', () => {
		const first = createCaptureProgress(RECORD_STEPS, { estimateKey: 'test-load-skew' });
		first.begin(2);
		first.update('LOADING_MODEL', 0);
		vi.advanceTimersByTime(60_000); // long cold model load
		first.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(2_000);
		first.complete();
		// Transcription matched the 2s hint, so the stored factor stays ~1.
		const stored = Number(localStorage.getItem('sf-capture-eta:test-load-skew'));
		expect(stored).toBeGreaterThan(0.8);
		expect(stored).toBeLessThan(1.3);
	});

	it('real backend progress only ever pulls the glide forward, never back', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.begin(2);
		capture.update('TRANSCRIBING_AUDIO', 0.05);
		vi.advanceTimersByTime(3_000);
		const glided = capture.percentExact;
		capture.update('TRANSCRIBING_AUDIO', 0.2); // band floor below the glide
		expect(capture.percentExact).toBeGreaterThanOrEqual(glided);
	});

	it('a real terminal tick reaches 100 for the last profiled step', () => {
		const capture = createCaptureProgress(DICTATE_STEPS);
		capture.update('TRANSCRIBING_AUDIO', 1);
		expect(capture.percent).toBe(100);
	});

	it('complete() pins 100 and stops the creep', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.update('WRITING_TRANSCRIPT', 1);
		capture.complete();
		vi.advanceTimersByTime(10_000);
		expect(capture.percent).toBe(100);
	});

	it('reset() returns to zero and stays there', () => {
		const capture = createCaptureProgress(RECORD_STEPS);
		capture.update('TRANSCRIBING_AUDIO', 0.5);
		capture.reset();
		vi.advanceTimersByTime(10_000);
		expect(capture.percent).toBe(0);
	});

	it('exposes the step sequence for the current stage', () => {
		const capture = createCaptureProgress(UPLOAD_STEPS);
		capture.update('WRITING_TRANSCRIPT', 1);
		expect(capture.sequence.map((s) => s.complete)).toEqual([true, true, true, false]);
	});

	describe('batch mode (upload queue)', () => {
		it('anchors to the queue average directly', () => {
			const capture = createCaptureProgress(UPLOAD_STEPS, { batch: true });
			capture.update('TRANSCRIBING_AUDIO', 0.5);
			expect(capture.percent).toBe(50);
		});

		it('creep stays within headroom of the last real number — no fake near-done', () => {
			const capture = createCaptureProgress(UPLOAD_STEPS, { batch: true });
			capture.update('TRANSCRIBING_AUDIO', 0.5);
			vi.advanceTimersByTime(300_000);
			expect(capture.percentExact).toBeLessThan(60);
		});

		it('stays monotonic when a later item re-enters an earlier stage', () => {
			const capture = createCaptureProgress(UPLOAD_STEPS, { batch: true });
			capture.update('WRITING_TRANSCRIPT', 0.5);
			capture.update('LOADING_MODEL', 0.5);
			expect(capture.percent).toBeGreaterThanOrEqual(50);
		});

		it('only the initial model load counts as loading — not a mid-queue reload', () => {
			const capture = createCaptureProgress(UPLOAD_STEPS, { batch: true });
			capture.update('LOADING_MODEL', 0);
			expect(capture.loading).toBe(true);
			capture.update('TRANSCRIBING_AUDIO', 0.5);
			capture.update('LOADING_MODEL', 0.5);
			expect(capture.loading).toBe(false);
		});
	});
});
