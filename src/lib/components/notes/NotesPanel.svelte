<script lang="ts">
	import NotesList from "./NotesList.svelte";
	import NoteComposer from "./NoteComposer.svelte";
	import type { Note } from "./NoteCard.svelte";
	import type { Snippet } from "svelte";

	let {
		notes,
		selectedId = $bindable<string | null>(null),
		draft = $bindable(""),
		onAddNote,
		header,
	}: {
		notes: Note[];
		selectedId?: string | null;
		draft?: string;
		onAddNote?: (text: string) => void;
		header?: Snippet;
	} = $props();

	function add() {
		const t = draft.trim();
		if (!t) return;
		onAddNote?.(t);
		draft = "";
	}
</script>

<div class="flex min-h-0 flex-1 flex-col gap-0">
	<div class="shrink-0 px-4 pt-4 pb-2">
		{#if header}
			{@render header()}
		{:else}
			<h2 class="font-mono text-label-sm tracking-stamped text-fg/60 uppercase">Notes</h2>
		{/if}
	</div>
	<div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-4">
		<NotesList bind:selectedId {notes} />
	</div>
	<div class="shrink-0 border-t border-card/0 px-4 pt-3 pb-4" style="box-shadow: inset 0 1px 0 0 var(--color-surface-low);">
		<NoteComposer bind:value={draft} onSubmit={add} />
	</div>
</div>
