<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import ScrollablePanel from '@primitives/layout/ScrollBody.svelte';

	type SessionNote = {
		id: string;
		text: string;
		recorded_at_ms: number;
	};

	let { noteId }: { noteId: string } = $props();

	let html = $state('');
	let sessionNotes = $state<SessionNote[]>([]);
	let loadError = $state('');

	onMount(async () => {
		try {
			const [rendered, detail] = await Promise.all([
				invoke<string>('note_render_transcript_html', { id: noteId }),
				invoke<{ notes?: SessionNote[] }>('history_get_detail', { id: noteId }),
			]);
			html = rendered;
			sessionNotes = detail.notes ?? [];
		} catch (e) {
			loadError = String(e);
		}
	});

	async function copyTranscript() {
		const plain = html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
		if (!plain) return;
		await writeText(plain).catch(() => {});
	}

	function formatNoteTime(ms: number): string {
		const totalSec = Math.floor(ms / 1000);
		const m = Math.floor(totalSec / 60);
		const s = totalSec % 60;
		return `${m}:${s.toString().padStart(2, '0')}`;
	}
</script>

<div class="flex h-full min-h-0 flex-col">
	<ScrollablePanel>
		<div class="flex justify-end px-4 pt-3 pb-1">
			<button
				type="button"
				class="sf-label-sm text-fg-dim hover:text-fg disabled:opacity-40"
				disabled={!html}
				onclick={copyTranscript}
			>
				Copy
			</button>
		</div>
		{#if loadError}
			<p class="px-4 pb-3 sf-body-sm text-destructive">{loadError}</p>
		{:else if html}
			<div class="prose-note px-4 pb-3 sf-body-sm text-fg">{@html html}</div>
		{:else}
			<p class="px-4 pb-3 sf-body-sm text-fg-muted">No transcript content.</p>
		{/if}
		{#if sessionNotes.length > 0}
			<div class="mt-4 flex flex-col gap-1.5 border-t border-rim/30 px-4 pt-3 pb-4">
				<span class="sf-label-sm text-fg-dim">Notes</span>
				{#each sessionNotes as note (note.id)}
					<div class="rounded border border-fill bg-fill/50 px-3 py-2">
						<span class="sf-meta-sm text-fg-dim">{formatNoteTime(note.recorded_at_ms)}</span>
						<p class="mt-0.5 sf-body-md text-fg">{note.text}</p>
					</div>
				{/each}
			</div>
		{/if}
	</ScrollablePanel>
</div>

<style>
	:global(.prose-note p) {
		margin: 0 0 0.75rem;
	}
	:global(.prose-note p:last-child) {
		margin-bottom: 0;
	}
</style>
