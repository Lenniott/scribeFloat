<script lang="ts">
	import { Mic, Upload } from 'lucide-svelte';

	let { origin }: { origin: 'mic' | 'upload' } = $props();
</script>

<div
	class={`flex size-8 shrink-0 items-center justify-center rounded-md ${origin === 'upload' ? 'bg-warning/10 text-warning' : 'bg-destructive/10 text-destructive'}`}
	aria-hidden="true"
>
	{#if origin === 'upload'}
		<Upload class="size-3.5" />
	{:else}
		<Mic class="size-3.5" />
	{/if}
</div>
