<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Button from '@components/controls/Button.svelte';
	import Toast from '@components/indicators/Toast.svelte';
	import PathPicker from '@components/controls/PathPicker.svelte';
	import ToggleSwitch from '@components/controls/Toggle.svelte';
	import SettingsList from '@sections/SettingList.svelte';
	import SettingsRow from '@components/cards/SettingRow.svelte';
	import SettingsSection from '@primitives/form/SettingsSection.svelte';
	import { appErrorMessage } from '@utils/types';
	import { createToast } from '@stores/toast.svelte';

	let saveTranscriptsAsMarkdown = $state(false);
	let keepWav = $state(false);
	let openWithApp = $state('');
	let threshold = $state(0.75);
	let embeddingsRetention = $state<'keep' | 'delete_after_transcript'>('keep');
	let confirmingRemoveTranscriptEmbeddings = $state(false);
	let loadError = $state('');
	let actionError = $state('');
	let actionMessage = $state('');
	let thresholdSaveTimer: ReturnType<typeof setTimeout> | undefined;
	const toast = createToast();

	async function refresh() {
		loadError = '';
		try {
			const [nextMarkdown, nextKeepWav, nextOpenWithApp, nextThreshold, retention] =
				await Promise.all([
					invoke<boolean>('settings_get_save_transcripts_as_markdown'),
					invoke<boolean>('settings_get_keep_wav'),
					invoke<string | null>('settings_get_open_with_app_path'),
					invoke<number>('settings_get_voice_similarity_threshold'),
					invoke<'keep' | 'delete_after_transcript'>('settings_get_voice_embeddings_retention'),
				]);
			saveTranscriptsAsMarkdown = nextMarkdown;
			keepWav = nextKeepWav;
			openWithApp = nextOpenWithApp ?? '';
			threshold = nextThreshold;
			embeddingsRetention = retention;
		} catch (e) {
			loadError = `Could not load settings: ${appErrorMessage(e)}`;
		}
	}

	async function setSaveTranscriptsAsMarkdown(enabled: boolean) {
		const previous = saveTranscriptsAsMarkdown;
		saveTranscriptsAsMarkdown = enabled;
		actionError = '';
		try {
			await invoke('settings_set_save_transcripts_as_markdown', { enabled });
			toast.show('Saved', 'success');
		} catch (e) {
			saveTranscriptsAsMarkdown = previous;
			actionError = `Could not save markdown setting: ${appErrorMessage(e)}`;
		}
	}

	async function setKeepWav(enabled: boolean) {
		const previous = keepWav;
		keepWav = enabled;
		actionError = '';
		try {
			await invoke('settings_set_keep_wav', { enabled });
			toast.show('Saved', 'success');
		} catch (e) {
			keepWav = previous;
			actionError = `Could not save audio setting: ${appErrorMessage(e)}`;
		}
	}

	async function saveOpenWithApp(path: string) {
		actionError = '';
		try {
			await invoke('settings_set_open_with_app_path', { path: path.trim() || null });
			toast.show('Saved', 'success');
		} catch (e) {
			actionError = `Could not save app path: ${appErrorMessage(e)}`;
			await refresh();
		}
	}

	function onThresholdInput(event: Event) {
		threshold = Number((event.currentTarget as HTMLInputElement).value);
		if (thresholdSaveTimer) clearTimeout(thresholdSaveTimer);
		thresholdSaveTimer = setTimeout(() => {
			void saveThreshold();
		}, 300);
	}

	async function saveThreshold() {
		actionError = '';
		try {
			await invoke('settings_set_voice_similarity_threshold', { threshold });
			toast.show('Saved', 'success');
		} catch (e) {
			actionError = `Could not save matching sensitivity: ${appErrorMessage(e)}`;
		}
	}

	async function setEmbeddingsRetention(retention: 'keep' | 'delete_after_transcript') {
		const previous = embeddingsRetention;
		embeddingsRetention = retention;
		actionError = '';
		try {
			await invoke('settings_set_voice_embeddings_retention', { retention });
			toast.show('Saved', 'success');
		} catch (e) {
			embeddingsRetention = previous;
			actionError = `Could not save voice data setting: ${appErrorMessage(e)}`;
		}
	}

	async function removeTranscriptEmbeddings() {
		confirmingRemoveTranscriptEmbeddings = false;
		actionError = '';
		actionMessage = '';
		try {
			const changed = await invoke<number>('history_remove_all_voice_embeddings');
			actionMessage = `Removed voice vectors from ${changed} ${changed === 1 ? 'transcript' : 'transcripts'}.`;
		} catch (e) {
			actionError = `Could not remove transcript voice data: ${appErrorMessage(e)}`;
		}
	}

	onMount(refresh);
	onDestroy(() => {
		if (thresholdSaveTimer) clearTimeout(thresholdSaveTimer);
		toast.dismiss();
	});
</script>

<section class="space-y-5">
	<h2 class="sf-headline-sm text-fg">Advanced</h2>

	{#if loadError}
		<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
			{loadError}
		</p>
	{/if}
	{#if actionError}
		<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
			{actionError}
		</p>
	{/if}
	{#if actionMessage}
		<p class="rounded-md border border-fill bg-panel px-3 py-2 sf-label-sm text-fg-dim">
			{actionMessage}
		</p>
	{/if}

	<SettingsSection title="Transcripts">
		<SettingsList>
			<SettingsRow title="Save transcripts as Markdown">
				{#snippet control()}
					<ToggleSwitch
						checked={saveTranscriptsAsMarkdown}
						onchange={(next) => void setSaveTranscriptsAsMarkdown(next)}
						aria-label="Save transcripts as Markdown"
					/>
				{/snippet}
			</SettingsRow>

			{#if saveTranscriptsAsMarkdown}
				<SettingsRow title="Open transcripts with">
					<PathPicker
						label="Open transcripts with"
						labelHidden={true}
						bind:path={openWithApp}
						directory={false}
						onChange={(next) => void saveOpenWithApp(next)}
					/>
				</SettingsRow>
			{/if}

			<SettingsRow title="Keep audio after transcription">
				{#snippet control()}
					<ToggleSwitch
						checked={keepWav}
						onchange={(next) => void setKeepWav(next)}
						aria-label="Keep audio after transcription"
					/>
				{/snippet}
			</SettingsRow>
		</SettingsList>
	</SettingsSection>

	<SettingsSection title="Voice matching">
		<SettingsList>
			<SettingsRow
				title="Speaker matching sensitivity"
				description="Lower is more inclusive. Higher only labels very confident matches."
			>
				{#snippet control()}
					<div class="flex w-full min-w-56 flex-col gap-1">
						<div class="flex items-center justify-between sf-meta-sm text-fg-dim">
							<span>Inclusive</span>
							<span>{threshold.toFixed(2)}</span>
							<span>Strict</span>
						</div>
						<input
							type="range"
							min="0"
							max="1"
							step="0.05"
							value={threshold}
							class="w-full accent-brand"
							oninput={onThresholdInput}
							aria-label="Speaker matching sensitivity"
						/>
					</div>
				{/snippet}
			</SettingsRow>

			<SettingsRow
				title="Keep voice data for future speaker matching"
				description="When off, transcripts can keep text and labels while embedding vectors are removed after processing."
			>
				{#snippet control()}
					<ToggleSwitch
						checked={embeddingsRetention === 'keep'}
						onchange={(next) =>
							void setEmbeddingsRetention(next ? 'keep' : 'delete_after_transcript')}
						aria-label="Keep voice data for future speaker matching"
					/>
				{/snippet}
			</SettingsRow>
		</SettingsList>

		<div class="mt-3 flex flex-col gap-2 rounded-md border border-fill bg-panel px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
			<div>
				<p class="sf-label-md text-fg">Remove voice vectors from transcripts</p>
				<p class="sf-label-sm text-fg-dim">
					Transcript text, speaker names, times, and quality scores stay readable.
				</p>
			</div>
			{#if confirmingRemoveTranscriptEmbeddings}
				<div class="flex shrink-0 gap-2">
					<Button variant="ghost" size="small" onclick={() => (confirmingRemoveTranscriptEmbeddings = false)}>Cancel</Button>
					<Button variant="destructive" size="small" onclick={() => void removeTranscriptEmbeddings()}>Remove vectors</Button>
				</div>
			{:else}
				<Button
					variant="destructive"
					size="small"
					onclick={() => (confirmingRemoveTranscriptEmbeddings = true)}
				>
					Remove vectors
				</Button>
			{/if}
		</div>
	</SettingsSection>
</section>

<Toast message={toast.message} state={toast.state} position="bottom-center" />
