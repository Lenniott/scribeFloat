import { describe, expect, it, vi } from 'vitest';
import { debounce } from './debounce';

describe('debounce', () => {
	it('coalesces a burst of calls within the window into a single trailing call', () => {
		vi.useFakeTimers();
		const fn = vi.fn();
		const debounced = debounce(fn, 200);

		debounced();
		debounced();
		debounced();
		vi.advanceTimersByTime(199);
		expect(fn).not.toHaveBeenCalled();
		vi.advanceTimersByTime(1);
		expect(fn).toHaveBeenCalledTimes(1);

		vi.useRealTimers();
	});

	it('still fires promptly for a single isolated call', () => {
		vi.useFakeTimers();
		const fn = vi.fn();
		const debounced = debounce(fn, 200);

		debounced();
		vi.advanceTimersByTime(200);
		expect(fn).toHaveBeenCalledTimes(1);

		vi.useRealTimers();
	});

	it('cancel() drops a pending call so it never fires', () => {
		vi.useFakeTimers();
		const fn = vi.fn();
		const debounced = debounce(fn, 200);

		debounced();
		debounced.cancel();
		vi.advanceTimersByTime(500);
		expect(fn).not.toHaveBeenCalled();

		vi.useRealTimers();
	});

	it('passes through the latest arguments to the trailing call', () => {
		vi.useFakeTimers();
		const fn = vi.fn();
		const debounced = debounce(fn, 200);

		debounced('first');
		debounced('second');
		vi.advanceTimersByTime(200);
		expect(fn).toHaveBeenCalledWith('second');

		vi.useRealTimers();
	});
});
