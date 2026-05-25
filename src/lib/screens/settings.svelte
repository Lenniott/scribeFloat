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

	// Windows auto-prompts for mic on first use and exposes no other permissions
	// we manage, so the Permissions tab is dead weight there — and querying status
	// triggers a `reg query` subprocess which used to flash a cmd window.
	const isWindows =
		typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);

	let activeTab = $state<SettingsTab>('general');
	let permissionsKnown = $state(false);
	let modelKnown = $state(false);
	let permissionsReady = $state(false);
	let modelReady = $state(false);

	onMount(async () => {
		const [statuses, list] = await Promise.all([
			isWindows
				? Promise.resolve([] as PermissionStatus[])
				: invoke<PermissionStatus[]>('settings_permissions_status').catch(() => []),
			invoke<ModelListItem[]>('model_list').catch(() => []),
		]);
		permissionsReady = isWindows
			? true
			: (statuses.find((s) => s.kind === 'microphone')?.granted ?? false);
		permissionsKnown = true;

		modelReady = list.some((m) => m.downloaded && m.selected);
		modelKnown = true;
		if (!list.some((m) => m.downloaded)) {
			activeTab = 'models';
		}
	});

	const tabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'general', label: 'General' },
		...(isWindows ? [] : [{ id: 'permissions' as SettingsTab, label: 'Permissions' }]),
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

		{#if (permissionsKnown && !permissionsReady) || (modelKnown && !modelReady)}
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
					<SettingGeneral />
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
