<script lang="ts">
	import NoteCard, { type Note } from "./NoteCard.svelte";

	let {
		notes,
		selectedId = $bindable<string | null>(null),
	}: {
		notes: Note[];
		selectedId?: string | null;
	} = $props();
</script>

<div class="flex flex-col gap-2" role="list">
	{#each notes as note (note.id)}
		<div role="listitem">
			<NoteCard {note} selected={selectedId === note.id} onselect={(id) => (selectedId = id)} />
		</div>
	{:else}
		<p class="sf-label-md text-fg-muted py-6 text-left">No notes yet.</p>
	{/each}
</div>
