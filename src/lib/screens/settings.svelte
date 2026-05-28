<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import NavButton from '@components/NavButton.svelte';
	import SettingGeneral from '@lib/screens/setting_general.svelte';
	import SettingPermissions from '@lib/screens/setting_permissions.svelte';
	import SettingModels from '@lib/screens/setting_models.svelte';
	import SettingHelp from '@lib/screens/setting_help.svelte';
	import SettingReplace from '@lib/screens/setting_replace.svelte';
	import IconButton from '@components/IconButton.svelte';
	import { X } from 'lucide-svelte';
	import type { PermissionStatus, ModelListItem } from '$lib/types';

	type SettingsTab = 'general' | 'permissions' | 'models' | 'replacements' | 'help';

	let {
		onClose,
		standalone = false,
	}: {
		onClose?: () => void;
		standalone?: boolean;
	} = $props();

	let activeTab = $state<SettingsTab>('general');
	let permissionsKnown = $state(false);
	let modelKnown = $state(false);
	let permissionsReady = $state(false);
	let modelReady = $state(false);
	let speakerCaptureKnown = $state(false);
	let speakerCaptureRequiresDeviceName = $state(false);
	let blackholeDetected = $state(false);
	let savedSpeakerDeviceName = $state('');

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
			await invoke('model_select', { modelId: downloaded[0].id }).catch(() => {});
			modelReady = true;
		} else {
			modelReady = hasSelected;
		}
		modelKnown = true;
	});

	const tabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'general', label: 'General' },
		{ id: 'permissions', label: 'Permissions' },
		{ id: 'models', label: 'Models' },
		{ id: 'replacements', label: 'Replacements' },
		{ id: 'help', label: 'Help' },
	];
</script>

<div class={standalone ? 'min-h-screen bg-card' : 'fixed inset-0 z-50 bg-black/50 p-4'}>
	<div class="mx-auto flex h-screen max-w-5xl flex-col bg-panel">
		<header class="flex items-center justify-between border-b border-card px-4 py-3">
			<h2 class="sf-headline-sm">Settings</h2>
			<IconButton aria-label="close settings" variant="normal" icon={X} onclick={() => onClose?.()} />
		</header>

		{#if showSettingsBanner}
			<div class="flex flex-col gap-1 border-b border-warning bg-warning/15 px-4 py-2">
				{#if permissionsKnown && !permissionsReady}
					<p class="text-label-sm text-fg">
						Microphone access needed —
						<button class="underline cursor-pointer" onclick={() => (activeTab = 'permissions')}>go to Permissions</button>.
					</p>
				{/if}
				{#if modelKnown && !modelReady}
					<p class="text-label-sm text-fg">
						No transcription model installed —
						<button class="underline cursor-pointer" onclick={() => (activeTab = 'models')}>go to Models</button>.
					</p>
				{/if}
				{#if speakerCaptureKnown && showSpeakerNameWarning}
					<p class="text-label-sm text-fg">
						BlackHole is installed, but no speaker capture device name is set — enter your
						<strong>Multi-Output Device</strong> name from Audio MIDI Setup in
						<button class="underline cursor-pointer" onclick={() => (activeTab = 'general')}>General</button>.
					</p>
				{/if}
			</div>
		{/if}

		<div class="flex min-h-0 h-full">
			<nav class="w-52 border-r border-card p-2">
				<div class="flex flex-col gap-1">
					{#each tabs as tab (tab.id)}
						<NavButton active={activeTab === tab.id} onclick={() => (activeTab = tab.id)}>
							{tab.label}
						</NavButton>
					{/each}
				</div>
			</nav>

			<section
				class={`min-h-0 flex-1 bg-card ${activeTab === 'models' ? 'flex flex-col overflow-hidden p-0' : 'overflow-y-auto p-4'}`}
			>
				{#if activeTab === 'general'}
					<SettingGeneral
						{savedSpeakerDeviceName}
						{blackholeDetected}
						{speakerCaptureRequiresDeviceName}
						onSpeakerConfigSaved={onSpeakerConfigSaved}
					/>
				{:else if activeTab === 'permissions'}
					<SettingPermissions bind:ready={permissionsReady} />
				{:else if activeTab === 'replacements'}
					<SettingReplace />
				{:else if activeTab === 'help'}
					<SettingHelp />
				{:else}
					<div class="flex h-full min-h-0 flex-1 flex-col">
						<SettingModels bind:ready={modelReady} />
					</div>
				{/if}
			</section>
		</div>
	</div>
</div>
