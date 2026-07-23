/**
 * Coalesces bursts of calls into one trailing invocation after `waitMs` of
 * silence. Exposes `cancel()` so callers (e.g. component unmount) can drop
 * a pending call before it fires.
 */
export function debounce<Args extends unknown[]>(
	fn: (...args: Args) => void,
	waitMs: number,
): { (...args: Args): void; cancel: () => void } {
	let timer: ReturnType<typeof setTimeout> | undefined;

	function debounced(...args: Args) {
		if (timer !== undefined) clearTimeout(timer);
		timer = setTimeout(() => {
			timer = undefined;
			fn(...args);
		}, waitMs);
	}

	debounced.cancel = () => {
		if (timer !== undefined) {
			clearTimeout(timer);
			timer = undefined;
		}
	};

	return debounced;
}
