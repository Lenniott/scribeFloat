<script lang="ts">
	import NotesList from '../patterns/NoteList.svelte';
	import NoteComposer from '../patterns/NoteComposer.svelte';
	import ScrollablePanel from '../primitives/layout/ScrollBody.svelte';
	import PanelFooter from '../primitives/layout/PanelFooter.svelte';
	import type { Note } from '../ui/cards/NoteSnippet.svelte';
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

<div class="flex min-h-0 flex-1 flex-col gap-0 overflow-hidden">
	<div class="shrink-0 px-4 pt-4 pb-2">
		{#if header}
			{@render header()}
		{:else}
			<h2 class="sf-section-label text-fg-dim">Notes</h2>
		{/if}
	</div>
	<ScrollablePanel class="px-4">
		<NotesList bind:selectedId {notes} />
	</ScrollablePanel>
	<PanelFooter class="w-full justify-start border-t-0 px-4 pt-3 pb-4">
		<NoteComposer bind:value={draft} onSubmit={add} />
	</PanelFooter>
</div>
