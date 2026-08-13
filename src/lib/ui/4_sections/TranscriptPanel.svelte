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
		chunk_id?: string | null;
	};
	type LabelCorrection = {
		from_label: string;
		to_label: string;
		corrected_at_ms: number;
		auto: boolean;
	};
	type SpeakerChunkInfo = {
		id: string;
		label: string;
		corrections?: LabelCorrection[];
	};
	type NoteDetail = {
		notes?: SessionNote[];
		speaker_blocks?: SpeakerBlock[];
		/** Legacy chunk-tier notes only; read for correction badges. */
		speaker_chunks?: SpeakerChunkInfo[];
	};

	type SpeakerNameEntry = {
		slug: string;
		name: string;
	};

	const CHANNEL_LABEL_IN = 'In';
	const CHANNEL_LABEL_OUT = 'Out';

	let { noteId }: { noteId: string } = $props();

	let html = $state('');
	let sessionNotes = $state<SessionNote[]>([]);
	let speakerBlocks = $state<SpeakerBlock[]>([]);
	let speakerChunks = $state<SpeakerChunkInfo[]>([]);
	let savedNames = $state<SpeakerNameEntry[]>([]);
	let userDisplayName = $state('You');
	let inputLabel = $state('Mic');
	let outputLabel = $state('Speaker');
	let hideOthers = $state(false);
	let correctingIndex = $state<number | null>(null);
	let newSpeakerName = $state('');
	/** Target name chosen from the picker or typed field, awaiting a this-turn/all-turns scope choice. */
	let pendingLabel = $state<string | null>(null);
	let correctionError = $state('');
	let loadError = $state('');

	const isChannelTier = $derived(
		speakerBlocks.some((b) => b.label === CHANNEL_LABEL_IN || b.label === CHANNEL_LABEL_OUT),
	);
	const isIdentityTier = $derived(speakerBlocks.length > 0 && !isChannelTier);
	const hasCopyableContent = $derived(
		speakerBlocks.length > 0 || html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim().length > 0,
	);

	function adoptDetail(detail: NoteDetail) {
		sessionNotes = detail.notes ?? [];
		speakerBlocks = detail.speaker_blocks ?? [];
		speakerChunks = detail.speaker_chunks ?? [];
	}

	onMount(async () => {
		try {
			const [rendered, detail, labels] = await Promise.all([
				invoke<string>('note_render_transcript_html', { id: noteId }),
				invoke<NoteDetail>('history_get_detail', { id: noteId }),
				invoke<[string, string]>('settings_get_input_labels').catch(() => ['Mic', 'Speaker'] as [string, string]),
			]);
			html = rendered;
			adoptDetail(detail);
			[inputLabel, outputLabel] = labels;
			savedNames = await invoke<SpeakerNameEntry[]>('speaker_names_list').catch(() => []);
			userDisplayName = await invoke<string>('settings_get_user_display_name').catch(() => 'You');
		} catch (e) {
			loadError = String(e);
		}
	});

	function displayLabel(label: string): string {
		if (label === 'You') return userDisplayName;
		if (label === CHANNEL_LABEL_IN) return inputLabel;
		if (label === CHANNEL_LABEL_OUT) return outputLabel;
		return label;
	}

	function blockPlainText(block: SpeakerBlock): string {
		const time =
			block.start_ms != null && block.end_ms != null
				? `${formatBlockTime(block.start_ms)} → ${formatBlockTime(block.end_ms)} `
				: '';
		return `[${displayLabel(block.label)}] ${time}${block.text}`.trim();
	}

	async function copyTranscript() {
		const plain =
			speakerBlocks.length > 0
				? speakerBlocks.map(blockPlainText).join('\n\n')
				: html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
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

	function visibleBlock(block: SpeakerBlock): boolean {
		if (!hideOthers) return true;
		return block.label === 'You' || block.label === userDisplayName;
	}

	function canRelabelBlock(block: SpeakerBlock): boolean {
		return block.label !== CHANNEL_LABEL_IN && block.label !== CHANNEL_LABEL_OUT;
	}

	function lastCorrection(chunkId: string | null | undefined): LabelCorrection | null {
		if (!chunkId) return null;
		const corrections = speakerChunks.find((chunk) => chunk.id === chunkId)?.corrections;
		return corrections?.length ? corrections[corrections.length - 1] : null;
	}

	function relabelOptions(block: SpeakerBlock): string[] {
		const labels = [
			...savedNames.map((entry) => entry.name),
			...speakerBlocks.map((other) => other.label),
			'Other',
		];
		return [...new Set(labels)].filter((label) => label !== block.label);
	}

	function startCorrection(index: number) {
		correctingIndex = correctingIndex === index ? null : index;
		newSpeakerName = '';
		pendingLabel = null;
		correctionError = '';
	}

	async function applyRelabel(block: SpeakerBlock, label: string, scope: 'all' | 'one', index: number) {
		const trimmed = label.trim();
		if (!trimmed) return;
		correctionError = '';
		try {
			const payload: Record<string, unknown> = { id: noteId, toLabel: trimmed, scope };
			if (scope === 'all') {
				payload.fromLabel = block.label;
			} else {
				payload.blockIndex = index;
			}
			const updated = await invoke<NoteDetail>('note_relabel_speaker', payload);
			adoptDetail(updated);
			correctingIndex = null;
			pendingLabel = null;
			newSpeakerName = '';
			savedNames = await invoke<SpeakerNameEntry[]>('speaker_names_list').catch(() => savedNames);
		} catch (e) {
			correctionError = String(e);
		}
	}
</script>

<div class="flex h-full min-h-0 flex-col">
	<ScrollablePanel>
		<div class="flex justify-end px-4 pt-3 pb-1">
			<button
				type="button"
				class="sf-label-sm text-fg-dim hover:text-fg disabled:opacity-40"
				disabled={!hasCopyableContent}
				onclick={copyTranscript}
			>
				Copy
			</button>
		</div>
		{#if loadError}
			<p class="px-4 pb-3 sf-body-sm text-destructive">{loadError}</p>
		{:else if speakerBlocks.length > 0}
			{#if isIdentityTier}
				<div class="flex justify-end px-4 pb-2">
					<Button variant={hideOthers ? 'active' : 'normal'} size="small" onclick={() => (hideOthers = !hideOthers)}>
						Hide Other
					</Button>
				</div>
			{/if}
			<div class="flex flex-col px-4 pb-3">
				{#each speakerBlocks as block, index (`${block.label}-${block.start_ms}-${index}`)}
					{#if visibleBlock(block)}
						{#if index > 0}
							<div class="my-3 border-t border-rim/30"></div>
						{/if}
						<div class="flex flex-col gap-1">
							<div class="flex flex-wrap items-center gap-2">
								{#if canRelabelBlock(block)}
									<button
										type="button"
										class="rounded-sm border border-fill bg-fill px-1.5 py-0.5 sf-label-sm text-fg"
										onclick={() => startCorrection(index)}
									>
										[{displayLabel(block.label)}]
									</button>
								{:else}
									<span class="rounded-sm border border-fill bg-fill px-1.5 py-0.5 sf-label-sm text-fg">
										[{displayLabel(block.label)}]
									</span>
								{/if}
								{#if lastCorrection(block.chunk_id)?.auto}
									<span class="sf-meta-sm text-fg-dim">auto-corrected</span>
								{:else if lastCorrection(block.chunk_id)}
									<span class="sf-meta-sm text-fg-dim">corrected</span>
								{/if}
								{#if block.start_ms != null && block.end_ms != null}
									<span class="sf-meta-sm text-fg-dim">{formatBlockTime(block.start_ms)} → {formatBlockTime(block.end_ms)}</span>
								{/if}
							</div>
							{#if correctingIndex === index}
								<div class="flex flex-col gap-2 rounded border border-fill bg-fill/50 p-2">
									{#if pendingLabel === null}
										<span class="sf-label-sm text-fg-dim">Who is speaking?</span>
										<div class="flex flex-wrap gap-1.5">
											{#each relabelOptions(block) as option (option)}
												<Button variant="normal" size="small" onclick={() => (pendingLabel = option)}>
													{displayLabel(option)}
												</Button>
											{/each}
										</div>
										<div class="flex items-end gap-2">
											<TextField label="New speaker" bind:value={newSpeakerName} />
											<Button variant="ghost" size="small" onclick={() => (correctingIndex = null)}>Cancel</Button>
											<Button
												variant="primary"
												size="small"
												disabled={!newSpeakerName.trim()}
												onclick={() => (pendingLabel = newSpeakerName.trim())}
											>
												Continue
											</Button>
										</div>
									{:else}
										<span class="sf-label-sm text-fg-dim">Rename to "{displayLabel(pendingLabel)}"</span>
										<div class="flex flex-wrap gap-1.5">
											<Button
												variant="primary"
												size="small"
												onclick={() => void applyRelabel(block, pendingLabel ?? '', 'one', index)}
											>
												This turn
											</Button>
											<Button
												variant="normal"
												size="small"
												onclick={() => void applyRelabel(block, pendingLabel ?? '', 'all', index)}
											>
												All turns named {displayLabel(block.label)}
											</Button>
											<Button variant="ghost" size="small" onclick={() => (pendingLabel = null)}>Back</Button>
										</div>
									{/if}
									{#if correctionError}
										<p class="sf-body-sm text-destructive">{correctionError}</p>
									{/if}
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
