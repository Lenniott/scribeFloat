<script lang="ts">
	import { Circle, Mic, Upload } from 'lucide-svelte';

	let { kind }: { kind: string } = $props();
</script>

<div
	class="flex size-8 shrink-0 items-center justify-center rounded-md bg-fill"
	aria-hidden="true"
>
	{#if kind === 'dictate'}
		<Mic class="size-3.5 text-fg-dim" />
	{:else if kind === 'transcribe'}
		<Upload class="size-3.5 text-fg-dim" />
	{:else}
		<Circle class="size-3.5 fill-fg-dim text-fg-dim" />
	{/if}
</div>
