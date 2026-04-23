<script lang="ts">
	import TimestampLabel from "./TimestampLabel.svelte";

	export type Note = {
		id: string;
		text: string;
		recordedAtMs: number;
	};

	let {
		note,
		selected = false,
		onselect,
	}: {
		note: Note;
		selected?: boolean;
		onselect?: (id: string) => void;
	} = $props();
</script>

<article
	class="rounded-md px-3 py-2 text-left transition-colors {selected
		? 'bg-surface-container-high'
		: 'bg-surface-container-low hover:bg-surface-container-high/80'}"
>
	<button
		type="button"
		class="w-full text-left"
		onclick={() => onselect?.(note.id)}
	>
		<TimestampLabel at={note.recordedAtMs} class="mb-1 block" />
		<p class="text-body-md whitespace-pre-wrap text-on-surface">{note.text}</p>
	</button>
</article>
