<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import SettingGeneral from '@views/setting_general.svelte';
	import SettingAdvanced from '@views/setting_advanced.svelte';
	import SettingPermissions from '@views/setting_permissions.svelte';
	import SettingVoice from '@views/setting_voice.svelte';
	import SettingHelp from '@views/setting_help.svelte';
	import ScrollablePanel from '@primitives/layout/ScrollBody.svelte';
	import { isWindows } from '@utils/platform';
	import { appState } from '@stores/appState.svelte';
	import type { PermissionStatus } from '@utils/types';
	import { SETTINGS_TABS, type SettingsTab } from './settingsTypes';
	import type { Component } from 'svelte';

	type SettingsTabView = {
		component: Component;
		props: Record<string, unknown>;
	};

	const activeTab = $derived(
		SETTINGS_TABS.find((tab) => tab.id === appState.settingsTab) ?? SETTINGS_TABS[0],
	);

	const tabViews = $derived.by(
		(): Record<SettingsTab, SettingsTabView> => ({
			general: {
				component: SettingGeneral,
				props: {
					savedSpeakerDeviceName,
					blackholeDetected,
					speakerCaptureRequiresDeviceName,
					onSpeakerConfigSaved,
				},
			},
			advanced: { component: SettingAdvanced, props: {} },
			voice: { component: SettingVoice, props: {} },
			permissions: {
				component: SettingPermissions,
				props: {
					micOnly: isWindows,
					onReadyChange: (ready: boolean) => {
						permissionsReady = ready;
					},
				},
			},
			help: { component: SettingHelp, props: {} },
		}),
	);

	const activeView = $derived(tabViews[appState.settingsTab]);

	let permissionsKnown = $state(false);
	let permissionsReady = $state(false);
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
		appState.settingsTab = tab;
	}

	onMount(async () => {
		const [statuses] = await Promise.all([
			invoke<PermissionStatus[]>('settings_permissions_status').catch(() => []),
			loadSpeakerCaptureBannerState(),
		]);
		permissionsReady =
			statuses.find((s) => s.kind === 'microphone')?.granted ?? false;
		permissionsKnown = true;
	});
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden bg-panel">
	<header class="shrink-0 border-b border-card px-4 py-3">
		<h2 class="sf-headline-sm text-fg">{activeTab.label}</h2>
	</header>

	{#if showSettingsBanner}
		<div class="flex shrink-0 flex-col gap-1 border-b border-warning bg-warning/15 px-4 py-2">
			{#if permissionsKnown && !permissionsReady}
				<p class="sf-label-sm text-fg">
					Microphone access needed —
					<button class="cursor-pointer underline" onclick={() => goToTab('permissions')}
						>go to Permissions</button
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

	<ScrollablePanel class="bg-card">
		{#key appState.settingsTab}
			<activeView.component {...activeView.props} />
		{/key}
	</ScrollablePanel>
</div>
