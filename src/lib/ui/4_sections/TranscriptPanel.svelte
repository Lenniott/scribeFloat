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
	type SessionSpeakerInfo = {
		session_speaker_id: string;
		label: string;
	};
	type NoteDetail = {
		notes?: SessionNote[];
		speaker_blocks?: SpeakerBlock[];
		speaker_chunks?: SpeakerChunkInfo[];
		session_speakers?: SessionSpeakerInfo[];
	};

	type ProfileSummary = {
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
	let sessionSpeakers = $state<SessionSpeakerInfo[]>([]);
	let profiles = $state<ProfileSummary[]>([]);
	let userDisplayName = $state('You');
	let inputLabel = $state('Mic');
	let outputLabel = $state('Speaker');
	let hideOthers = $state(false);
	let correctingIndex = $state<number | null>(null);
	let newSpeakerName = $state('');
	let correctionError = $state('');
	let learnOffer = $state<{ profileName: string; sessionSpeakerId: string } | null>(null);
	let learnBusy = $state(false);
	let learnNotice = $state('');
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
		sessionSpeakers = detail.session_speakers ?? [];
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
			profiles = await invoke<ProfileSummary[]>('voiceprint_list_profiles').catch(() => []);
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

	function canCorrectBlock(block: SpeakerBlock): boolean {
		return isIdentityTier && block.chunk_id != null;
	}

	function lastCorrection(chunkId: string | null | undefined): LabelCorrection | null {
		if (!chunkId) return null;
		const corrections = speakerChunks.find((chunk) => chunk.id === chunkId)?.corrections;
		return corrections?.length ? corrections[corrections.length - 1] : null;
	}

	function correctionOptions(block: SpeakerBlock): string[] {
		const labels = [
			...profiles.map((profile) => profile.name),
			...sessionSpeakers.map((speaker) => speaker.label),
			'Other',
		];
		return [...new Set(labels)].filter((label) => label !== block.label);
	}

	function startCorrection(index: number) {
		correctingIndex = correctingIndex === index ? null : index;
		newSpeakerName = '';
		correctionError = '';
		learnNotice = '';
	}

	async function applyCorrection(block: SpeakerBlock, label: string) {
		const trimmed = label.trim();
		if (!trimmed || !block.chunk_id) return;
		correctionError = '';
		try {
			const updated = await invoke<NoteDetail>('note_correct_chunk_label', {
				id: noteId,
				chunkId: block.chunk_id,
				label: trimmed,
			});
			adoptDetail(updated);
			correctingIndex = null;
			newSpeakerName = '';
			await maybeOfferLearning(trimmed);
		} catch (e) {
			correctionError = String(e);
		}
	}

	async function maybeOfferLearning(label: string) {
		learnOffer = null;
		if (!profiles.some((profile) => profile.name === label)) return;
		const speaker = sessionSpeakers.find((item) => item.label === label);
		if (!speaker) return;
		try {
			const report = await invoke<{ eligible: boolean }>('voiceprint_evaluate_session_evidence', {
				noteId,
				sessionSpeakerId: speaker.session_speaker_id,
			});
			if (report.eligible) {
				learnOffer = { profileName: label, sessionSpeakerId: speaker.session_speaker_id };
			}
		} catch {
			// Learning disabled or unavailable — no offer, nothing to report.
		}
	}

	async function acceptLearnOffer() {
		if (!learnOffer || learnBusy) return;
		learnBusy = true;
		try {
			await invoke('voiceprint_apply_session_evidence', {
				noteId,
				sessionSpeakerId: learnOffer.sessionSpeakerId,
				profileName: learnOffer.profileName,
			});
			learnNotice = `Updated ${displayLabel(learnOffer.profileName)}'s voiceprint from this recording.`;
			learnOffer = null;
		} catch (e) {
			learnNotice = String(e);
		} finally {
			learnBusy = false;
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
			{#if learnOffer}
				<div class="mx-4 mb-2 flex flex-wrap items-center gap-2 rounded border border-fill bg-fill/50 px-3 py-2">
					<span class="sf-body-sm text-fg">
						Improve {displayLabel(learnOffer.profileName)}'s voiceprint from this recording?
					</span>
					<Button variant="primary" size="small" disabled={learnBusy} onclick={() => void acceptLearnOffer()}>
						Add
					</Button>
					<Button variant="ghost" size="small" onclick={() => (learnOffer = null)}>Not now</Button>
				</div>
			{:else if learnNotice}
				<p class="mx-4 mb-2 sf-body-sm text-fg-dim">{learnNotice}</p>
			{/if}
			<div class="flex flex-col px-4 pb-3">
				{#each speakerBlocks as block, index (`${block.label}-${block.start_ms}-${index}`)}
					{#if visibleBlock(block)}
						{#if index > 0}
							<div class="my-3 border-t border-rim/30"></div>
						{/if}
						<div class="flex flex-col gap-1">
							<div class="flex flex-wrap items-center gap-2">
								{#if canCorrectBlock(block)}
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
									<span class="sf-label-sm text-fg-dim">Who is speaking?</span>
									<div class="flex flex-wrap gap-1.5">
										{#each correctionOptions(block) as option (option)}
											<Button variant="normal" size="small" onclick={() => void applyCorrection(block, option)}>
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
											onclick={() => void applyCorrection(block, newSpeakerName)}
										>
											Save
										</Button>
									</div>
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
