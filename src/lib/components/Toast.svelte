<script module lang="ts">
	export type ToastPosition = 'bottom-right' | 'bottom-center';
	export type ToastState = 'success' | 'error' | 'normal';
</script>

<script lang="ts">
	import { fade, fly } from 'svelte/transition';

	let {
		message = '',
		position = 'bottom-right',
		state = 'normal',
	}: {
		message?: string;
		position?: ToastPosition;
		state?: ToastState;
	} = $props();

	const variantClass: Record<ToastState, string> = {
		success: 'border-green bg-green/10 text-on-surface',
		error: 'border-error/40 bg-error-container text-on-error-container',
		normal: 'border-outline-variant/20 bg-surface-container-low text-on-surface',
	};

	const positionClass = $derived(
		position === 'bottom-center'
			? 'bottom-6 left-1/2 -translate-x-1/2'
			: 'right-6 bottom-6',
	);
</script>

{#if message}
	<div class={`fixed z-50 ${positionClass}`} role="status" aria-live="polite">
		<p
			in:fly={{ y: 12, duration: 160 }}
			out:fade={{ duration: 140 }}
			class={`rounded-md border p-2 text-body-sm shadow-ambient min-w-2xs ${variantClass[state]}`}
		>
			{message}
		</p>
	</div>
{/if}
