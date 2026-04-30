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
		success: 'border-surface-high bg-surface-low text-green',
		error: 'border-error/40 bg-error text-on-error',
		normal: 'border-surface-low/20 bg-surface-low text-on-surface',
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
			class={`rounded-md border p-2 text-body-md shadow-ambient min-w-2xs ${variantClass[state]}`}
		>
			{message}
		</p>
	</div>
{/if}
