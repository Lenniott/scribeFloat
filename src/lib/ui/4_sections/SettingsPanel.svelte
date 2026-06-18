<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import SettingGeneral from '@views/setting_general.svelte';
	import SettingPermissions from '@views/setting_permissions.svelte';
	import SettingModels from '@views/setting_models.svelte';
	import SettingHelp from '@views/setting_help.svelte';
	import SettingReplace from '@views/setting_replace.svelte';
	import ScrollablePanel from '@primitives/layout/ScrollBody.svelte';
	import { isWindows } from '@utils/platform';
	import { appErrorMessage, type PermissionStatus, type ModelListItem } from '@utils/types';
	import type { SettingsTab } from './settingsTypes';

	let {
		activeTab = $bindable<SettingsTab>('general'),
	}: {
		activeTab?: SettingsTab;
	} = $props();

	let permissionsKnown = $state(false);
	let modelKnown = $state(false);
	let permissionsReady = $state(false);
	let modelReady = $state(false);
	let speakerCaptureKnown = $state(false);
	let speakerCaptureRequiresDeviceName = $state(false);
	let blackholeDetected = $state(false);
	let savedSpeakerDeviceName = $state('');
	let setupError = $state('');

	const showSpeakerNameWarning = $derived(
		speakerCaptureRequiresDeviceName &&
			blackholeDetected &&
			savedSpeakerDeviceName.trim().length === 0,
	);

	const showSettingsBanner = $derived(
		(permissionsKnown && !permissionsReady) ||
			(modelKnown && !modelReady) ||
			(speakerCaptureKnown && showSpeakerNameWarning),
	);

	async function loadSpeakerCaptureBannerState() {
		speakerCaptureRequiresDeviceName = await invoke<boolean>(
			'settings_speaker_capture_requires_device_name',
		).catch(() => false);
		blackholeDetected = await invoke<boolean>('settings_blackhole_detected').catch(
			() => false,
		);
		const [, preferredSpeaker] = await invoke<[string | null, string | null]>(
			'settings_get_preferred_audio_devices',
		).catch((): [null, null] => [null, null]);
		savedSpeakerDeviceName = preferredSpeaker ?? '';
		speakerCaptureKnown = true;
	}

	function onSpeakerConfigSaved(name: string) {
		savedSpeakerDeviceName = name;
	}

	function goToTab(tab: SettingsTab) {
		activeTab = tab;
	}

	onMount(async () => {
		const [statuses, list] = await Promise.all([
			invoke<PermissionStatus[]>('settings_permissions_status').catch(() => []),
			invoke<ModelListItem[]>('model_list').catch(() => []),
			loadSpeakerCaptureBannerState(),
		]);
		permissionsReady =
			statuses.find((s) => s.kind === 'microphone')?.granted ?? false;
		permissionsKnown = true;

		const downloaded = list.filter((m) => m.downloaded);
		const hasSelected = list.some((m) => m.downloaded && m.selected);
		if (downloaded.length > 0 && !hasSelected) {
			try {
				await invoke('model_select', { modelId: downloaded[0].id });
				modelReady = true;
			} catch (e) {
				setupError = appErrorMessage(e);
				modelReady = false;
			}
		} else {
			modelReady = hasSelected;
		}
		modelKnown = true;
	});
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-panel">
	<header class="shrink-0 border-b border-card px-4 py-3">
		<h2 class="sf-headline-sm text-fg">Settings</h2>
	</header>

	{#if showSettingsBanner}
		<div class="flex shrink-0 flex-col gap-1 border-b border-warning bg-warning/15 px-4 py-2">
			{#if setupError}
				<p class="sf-label-sm text-fg">
					Could not select an installed model — {setupError}
				</p>
			{/if}
			{#if permissionsKnown && !permissionsReady}
				<p class="sf-label-sm text-fg">
					Microphone access needed —
					<button class="cursor-pointer underline" onclick={() => goToTab('permissions')}
						>go to Permissions</button
					>.
				</p>
			{/if}
			{#if modelKnown && !modelReady}
				<p class="sf-label-sm text-fg">
					No transcription model installed —
					<button class="cursor-pointer underline" onclick={() => goToTab('models')}
						>go to Models</button
					>.
				</p>
			{/if}
			{#if speakerCaptureKnown && showSpeakerNameWarning}
				<p class="sf-label-sm text-fg">
					BlackHole is installed, but no speaker capture device name is set — enter your
					<strong>Multi-Output Device</strong> name from Audio MIDI Setup in
					<button class="cursor-pointer underline" onclick={() => goToTab('general')}
						>General</button
					>.
				</p>
			{/if}
		</div>
	{/if}

	{#if activeTab === 'models'}
		<div class="flex min-h-0 flex-1 flex-col overflow-hidden bg-card">
			<SettingModels bind:ready={modelReady} />
		</div>
	{:else}
		<ScrollablePanel class="bg-card p-4">
			{#if activeTab === 'general'}
				<SettingGeneral
					{savedSpeakerDeviceName}
					{blackholeDetected}
					{speakerCaptureRequiresDeviceName}
					onSpeakerConfigSaved={onSpeakerConfigSaved}
				/>
			{:else if activeTab === 'permissions'}
				<SettingPermissions bind:ready={permissionsReady} micOnly={isWindows} />
			{:else if activeTab === 'replacements'}
				<SettingReplace />
			{:else}
				<SettingHelp />
			{/if}
		</ScrollablePanel>
	{/if}
</div>
