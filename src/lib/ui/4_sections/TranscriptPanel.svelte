<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import Button from '@components/controls/Button.svelte';
	import TextField from '@primitives/form/TextField.svelte';
	import ScrollablePanel from '@primitives/layout/ScrollBody.svelte';

	type SessionNote = {
		id: string;
		text: string;
		recorded_at_ms: number;
	};
	type SpeakerBlock = {
		label: string;
		start_ms: number | null;
		end_ms: number | null;
		text: string;
	};
	type ProfileSummary = {
		slug: string;
		name: string;
	};

	let { noteId }: { noteId: string } = $props();

	let html = $state('');
	let sessionNotes = $state<SessionNote[]>([]);
	let speakerBlocks = $state<SpeakerBlock[]>([]);
	let profiles = $state<ProfileSummary[]>([]);
	let userDisplayName = $state('You');
	let hideOthers = $state(false);
	let renamingLabel = $state('');
	let renameValue = $state('');
	let loadError = $state('');

	onMount(async () => {
		try {
			const [rendered, detail] = await Promise.all([
				invoke<string>('note_render_transcript_html', { id: noteId }),
				invoke<{ notes?: SessionNote[]; speaker_blocks?: SpeakerBlock[] }>('history_get_detail', { id: noteId }),
			]);
			html = rendered;
			sessionNotes = detail.notes ?? [];
			speakerBlocks = detail.speaker_blocks ?? [];
			profiles = await invoke<ProfileSummary[]>('voiceprint_list_profiles').catch(() => []);
			userDisplayName = await invoke<string>('settings_get_user_display_name').catch(() => 'You');
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

	function formatBlockTime(ms: number | null): string {
		if (ms == null) return '';
		const totalSec = Math.floor(ms / 1000);
		const m = Math.floor(totalSec / 60);
		const s = totalSec % 60;
		return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
	}

	function displayLabel(label: string): string {
		return label === 'You' ? userDisplayName : label;
	}

	function visibleBlock(block: SpeakerBlock): boolean {
		return !hideOthers || block.label === 'You' || block.label === userDisplayName;
	}

	function startRename(label: string) {
		renamingLabel = label;
		renameValue = displayLabel(label);
	}

	async function saveRename() {
		const profile = profiles.find((item) => item.name === renamingLabel || item.name === renameValue);
		if (!profile || !renameValue.trim()) {
			renamingLabel = '';
			return;
		}
		const previous = speakerBlocks;
		speakerBlocks = speakerBlocks.map((block) =>
			block.label === renamingLabel ? { ...block, label: renameValue.trim() } : block,
		);
		try {
			await invoke('voiceprint_rename_profile', { slug: profile.slug, name: renameValue.trim() });
			renamingLabel = '';
		} catch {
			speakerBlocks = previous;
		}
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
		{:else if speakerBlocks.length > 0}
			<div class="flex justify-end px-4 pb-2">
				<Button variant={hideOthers ? 'active' : 'normal'} size="small" onclick={() => (hideOthers = !hideOthers)}>
					Hide Other
				</Button>
			</div>
			<div class="flex flex-col px-4 pb-3">
				{#each speakerBlocks as block, index (`${block.label}-${block.start_ms}-${index}`)}
					{#if visibleBlock(block)}
						{#if index > 0}
							<div class="my-3 border-t border-rim/30"></div>
						{/if}
						<div class="flex flex-col gap-1">
							<div class="flex flex-wrap items-center gap-2">
								<button
									type="button"
									class="rounded-sm border border-fill bg-fill px-1.5 py-0.5 sf-label-sm text-fg"
									onclick={() => startRename(block.label)}
								>
									[{displayLabel(block.label)}]
								</button>
								{#if block.start_ms != null && block.end_ms != null}
									<span class="sf-meta-sm text-fg-dim">{formatBlockTime(block.start_ms)} → {formatBlockTime(block.end_ms)}</span>
								{/if}
							</div>
							{#if renamingLabel === block.label}
								<div class="flex items-end gap-2">
									<TextField label="Rename speaker" bind:value={renameValue} />
									<Button variant="ghost" size="small" onclick={() => (renamingLabel = '')}>Cancel</Button>
									<Button variant="primary" size="small" onclick={() => void saveRename()}>Save</Button>
								</div>
							{/if}
							<p class="sf-body-sm text-fg">{block.text}</p>
						</div>
					{/if}
				{/each}
			</div>
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
