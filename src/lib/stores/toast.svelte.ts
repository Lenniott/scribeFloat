import type { ToastState } from '@components/indicators/Toast.svelte';

/**
 * Transient toast state for a view: `show()` replaces the current message and
 * restarts the auto-dismiss timer. Render with `<Toast message={toast.message}
 * state={toast.state} />`.
 */
export function createToast(durationMs = 2000) {
	let message = $state('');
	let toastState = $state<ToastState>('normal');
	let timer: ReturnType<typeof setTimeout> | undefined;

	return {
		get message() {
			return message;
		},
		get state() {
			return toastState;
		},
		show(nextMessage: string, nextState: ToastState = 'normal') {
			if (timer) clearTimeout(timer);
			message = nextMessage;
			toastState = nextState;
			timer = setTimeout(() => {
				message = '';
				timer = undefined;
			}, durationMs);
		},
		dismiss() {
			if (timer) clearTimeout(timer);
			timer = undefined;
			message = '';
		},
	};
}
