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
		success: 'border-fill bg-card text-success',
		error: 'border-destructive/40 bg-destructive text-on-destructive',
		normal: 'border-card/20 bg-card text-fg',
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
			class={`sf-body-md rounded-md border p-2 shadow-ambient min-w-2xs ${variantClass[state]}`}
		>
			{message}
		</p>
	</div>
{/if}
