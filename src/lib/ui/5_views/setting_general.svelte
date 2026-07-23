<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { emit } from '@tauri-apps/api/event';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import Toast from '@components/indicators/Toast.svelte';
	import Button from '@components/controls/Button.svelte';
	import TextField from '@primitives/form/TextField.svelte';
	import PathPicker from '@components/controls/PathPicker.svelte';
	import ToggleSwitch from '@components/controls/Toggle.svelte';
	import SettingsList from '@sections/SettingList.svelte';
	import SettingsRow from '@components/cards/SettingRow.svelte';
	import SettingsSection from '@primitives/form/SettingsSection.svelte';
	import { appErrorMessage, type UpdateCheckResult } from '@utils/types';
	import { createToast } from '@stores/toast.svelte';

	type UpdateState =
		| 'idle'
		| 'checking'
		| 'up_to_date'
		| 'update_available'
		| 'error';

	let {
		savedSpeakerDeviceName = '',
		blackholeDetected = false,
		speakerCaptureRequiresDeviceName = false,
		onSpeakerConfigSaved,
	}: {
		savedSpeakerDeviceName?: string;
		blackholeDetected?: boolean;
		speakerCaptureRequiresDeviceName?: boolean;
		onSpeakerConfigSaved?: (name: string) => void;
	} = $props();

	let outputPath = $state('');
	let preferredInputDevice = $state('');
	let preferredSpeakerDevice = $state('');
	let scribeCaptureSpeaker = $state(false);
	let dictateAutoEnter = $state(false);
	let actionError = $state('');
	let loadError = $state('');
	let updateState = $state<UpdateState>('idle');
	let updateResult = $state<UpdateCheckResult | null>(null);
	let restartingOnboarding = $state(false);
	const toast = createToast(3000);

	const speakerCaptureAvailable = $derived(
		!speakerCaptureRequiresDeviceName ||
			(blackholeDetected && savedSpeakerDeviceName.trim().length > 0),
	);

	async function refresh() {
		loadError = '';
		try {
			const [
				nextOutputPath,
				[preferredInput, preferredSpeaker],
				nextScribeCaptureSpeaker,
				nextDictateAutoEnter,
			] = await Promise.all([
				invoke<string>('settings_get_output_path'),
				invoke<[string | null, string | null]>('settings_get_preferred_audio_devices'),
				invoke<boolean>('settings_get_scribe_capture_speaker'),
				invoke<boolean>('settings_get_dictate_auto_enter'),
			]);

			outputPath = nextOutputPath;
			preferredInputDevice = preferredInput ?? '';
			preferredSpeakerDevice = preferredSpeaker ?? '';
			scribeCaptureSpeaker = nextScribeCaptureSpeaker;
			dictateAutoEnter = nextDictateAutoEnter;
		} catch (e) {
			loadError = `Could not load settings: ${appErrorMessage(e)}`;
		}
	}

	async function saveOutputPath(path: string) {
		actionError = '';
		try {
			await invoke('settings_set_output_path', { path });
			toast.show('Saved', 'success');
		} catch (e) {
			actionError = `Could not save save folder: ${appErrorMessage(e)}`;
			await refresh();
		}
	}

	async function saveSpeakerDeviceName() {
		const trimmed = preferredSpeakerDevice.trim();
		actionError = '';
		try {
			await invoke('settings_set_preferred_audio_devices', {
				preferredInputDevice: preferredInputDevice.trim() || null,
				preferredSpeakerDevice: trimmed || null,
			});
			preferredSpeakerDevice = trimmed;
			onSpeakerConfigSaved?.(trimmed);
			await emit('settings://speaker-capture-saved');
			toast.show('Saved', 'success');
		} catch (e) {
			actionError = `Could not save speaker device: ${appErrorMessage(e)}`;
		}
	}

	async function setScribeCaptureSpeaker(enabled: boolean) {
		const previous = scribeCaptureSpeaker;
		scribeCaptureSpeaker = enabled;
		actionError = '';
		try {
			await invoke('settings_set_scribe_capture_speaker', { enabled });
			toast.show('Saved', 'success');
		} catch (e) {
			scribeCaptureSpeaker = previous;
			actionError = `Could not save speaker capture setting: ${appErrorMessage(e)}`;
		}
	}

	async function setDictateAutoEnter(enabled: boolean) {
		const previous = dictateAutoEnter;
		dictateAutoEnter = enabled;
		actionError = '';
		try {
			await invoke('settings_set_dictate_auto_enter', { enabled });
			toast.show('Saved', 'success');
		} catch (e) {
			dictateAutoEnter = previous;
			actionError = `Could not save dictate setting: ${appErrorMessage(e)}`;
		}
	}

	async function restartOnboarding() {
		restartingOnboarding = true;
		await invoke('settings_reset_onboarding').catch(() => {});
		await invoke('settings_show_onboarding_window').catch(() => {});
		restartingOnboarding = false;
	}

	async function checkForUpdates() {
		updateState = 'checking';
		updateResult = null;
		try {
			const result = await invoke<UpdateCheckResult>('update_check');
			updateResult = result;
			if (result.update_available) {
				updateState = 'update_available';
				toast.show(`Version ${result.latest_version} is available`);
			} else {
				updateState = 'up_to_date';
				toast.show("You're on the latest version.", 'success');
			}
		} catch (e) {
			const message = typeof e === 'string' ? e : 'Could not reach update server.';
			updateState = 'error';
			toast.show(`Could not check for updates: ${message}`, 'error');
		}
	}

	async function openDownloadPage() {
		if (updateResult) await openUrl(updateResult.release_url);
	}

	onMount(refresh);
	onDestroy(() => toast.dismiss());
</script>

<section class="space-y-5">
	{#if loadError}
		<p
			class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive"
		>
			{loadError}
		</p>
	{/if}
	{#if actionError}
		<p
			class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive"
		>
			{actionError}
		</p>
	{/if}

	<SettingsSection title="Record">
		<SettingsList>
			<SettingsRow title="Default save folder">
				<PathPicker
					label="Default save folder"
					labelHidden={true}
					bind:path={outputPath}
					onChange={(next) => void saveOutputPath(next)}
				/>
			</SettingsRow>

			{#if speakerCaptureRequiresDeviceName}
				<SettingsRow
					title="Speaker capture device name"
					description="Type the exact Audio MIDI device name."
				>
					<TextField
						label="Speaker capture device name"
						labelHidden={true}
						bind:value={preferredSpeakerDevice}
						placeholder="Type the exact Audio MIDI device name"
						onblur={() => void saveSpeakerDeviceName()}
					/>
				</SettingsRow>
			{/if}

			{#if speakerCaptureAvailable}
				<SettingsRow title="Capture speaker by default">
					{#snippet control()}
						<ToggleSwitch
							checked={scribeCaptureSpeaker}
							onchange={(next) => void setScribeCaptureSpeaker(next)}
							aria-label="Capture speaker by default"
						/>
					{/snippet}
				</SettingsRow>
			{/if}
		</SettingsList>
	</SettingsSection>

	<SettingsSection title="Dictate">
		<SettingsList>
			<SettingsRow title="Press Enter after dictate">
				{#snippet control()}
					<ToggleSwitch
						checked={dictateAutoEnter}
						onchange={(next) => void setDictateAutoEnter(next)}
						aria-label="Press Enter after dictate"
					/>
				{/snippet}
			</SettingsRow>
		</SettingsList>
	</SettingsSection>

	<SettingsSection title="App">
		<div class="flex flex-col gap-2">
			<Button
				variant="normal"
				onclick={restartOnboarding}
				disabled={restartingOnboarding}
			>
				{restartingOnboarding ? 'Opening…' : 'Restart Setup Wizard'}
			</Button>
			<Button
				variant="normal"
				onclick={checkForUpdates}
				disabled={updateState === 'checking'}
			>
				{updateState === 'checking' ? 'Checking…' : 'Check for updates'}
			</Button>

			{#if updateState === 'update_available' && updateResult}
				<div class="mt-2 space-y-2 rounded-md border border-fill bg-fill p-3">
					<p class="sf-body-md-strong text-fg">
						Version {updateResult.latest_version} is available
					</p>
					{#if updateResult.release_notes}
						<p class="sf-body-md text-fg-dim">{updateResult.release_notes}</p>
					{/if}
					<Button variant="primary" onclick={openDownloadPage}>
						Open download page
					</Button>
				</div>
			{/if}
		</div>
	</SettingsSection>
</section>

<Toast message={toast.message} state={toast.state} position="bottom-center" />
