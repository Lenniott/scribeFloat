<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
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
	let loadError = $state('');
	let actionError = $state('');
	const toast = createToast();

	async function refresh() {
		loadError = '';
		try {
			const [nextMarkdown, nextKeepWav, nextOpenWithApp] = await Promise.all([
				invoke<boolean>('settings_get_save_transcripts_as_markdown'),
				invoke<boolean>('settings_get_keep_wav'),
				invoke<string | null>('settings_get_open_with_app_path'),
			]);
			saveTranscriptsAsMarkdown = nextMarkdown;
			keepWav = nextKeepWav;
			openWithApp = nextOpenWithApp ?? '';
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

	onMount(refresh);
	onDestroy(() => {
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
</section>

<Toast message={toast.message} state={toast.state} position="bottom-center" />
