<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import NavButton from '@components/NavButton.svelte';
	import Button from '@components/Button.svelte';
	import SettingGeneral from '@lib/screens/setting_general.svelte';
	import SettingPermissions from '@lib/screens/setting_permissions.svelte';
	import SettingModels from '@lib/screens/setting_models.svelte';
	import IconButton from '@components/IconButton.svelte';
	import { X } from 'lucide-svelte';
	import { extractErrorMessage } from '$lib/types';

	type SettingsTab = 'general' | 'permissions' | 'models';

	let {
		onClose,
		standalone = false,
		setupHighlight = false,
	}: {
		onClose?: () => void;
		standalone?: boolean;
		setupHighlight?: boolean;
	} = $props();

	let activeTab = $state<SettingsTab | undefined>();
	let modelReady = $state(false);
	let permissionsReady = $state(false);
	let setupMessage = $state('');
	let finishing = $state(false);
	let setupDismissedThisSession = $state(false);
	const setupMode = $derived(setupHighlight && !setupDismissedThisSession);

	const currentTab = $derived(activeTab ?? (setupMode ? 'models' : 'general'));
	const prerequisitesMet = $derived(modelReady && permissionsReady);
	const setupStep = $derived(currentTab === 'models' ? 1 : currentTab === 'permissions' ? 2 : 0);

	const tabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'general', label: 'General' },
		{ id: 'permissions', label: 'Permissions' },
		{ id: 'models', label: 'Models' },
	];
	const highlightTabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'models', label: 'Models' },
		{ id: 'permissions', label: 'Permissions' },
	];
	const visibleTabs = $derived(setupMode ? highlightTabs : tabs);

	function selectTab(tab: SettingsTab) {
		if (setupMode && tab === 'general') return;
		activeTab = tab;
	}

	async function markSetupDismissedPersisted(): Promise<boolean> {
		setupMessage = '';
		const ok = await invoke('settings_complete_onboarding')
			.then(() => true)
			.catch((error: unknown) => {
				setupMessage = extractErrorMessage(error, 'Could not save setup preference.');
				return false;
			});
		return ok;
	}

	async function configureLater() {
		if (finishing) return;
		finishing = true;
		const ok = await markSetupDismissedPersisted();
		finishing = false;
		if (!ok) return;
		setupDismissedThisSession = true;
		setupMessage = '';
		onClose?.();
	}

	async function finishHighlightSetup() {
		if (!prerequisitesMet || finishing) return;
		finishing = true;
		const ok = await markSetupDismissedPersisted();
		finishing = false;
		if (!ok) return;
		setupDismissedThisSession = true;
		onClose?.();
	}
</script>

<div class={standalone ? 'min-h-screen bg-surface-low' : 'fixed inset-0 z-50 bg-black/50 p-4'}>
	<div class="mx-auto flex h-screen max-w-5xl flex-col bg-surface-lowest shadow-lg">
		<header class="flex items-center justify-between border-b border-surface-low px-4 py-3">
			<div>
				<h2 class="sf-headline-sm">
					{setupMode ? 'Set up Liscribe' : 'Settings'}
				</h2>
				{#if setupMode}
					<p class="text-label-sm text-on-surface/60">
						Step {setupStep} of 2 — {currentTab === 'models'
							? 'Install and select a model'
							: 'Grant microphone access for recording'}.
					</p>
				{/if}
			</div>
			<IconButton aria-label="close settings" variant="normal" icon={X} onclick={() => onClose?.()} />
		</header>

		<div class="flex min-h-0 h-full">
			<nav class="w-52 border-r border-surface-low p-2">
				<div class="flex flex-col gap-1">
					{#each visibleTabs as tab (tab.id)}
						<NavButton active={currentTab === tab.id} onclick={() => selectTab(tab.id)}>
							{tab.label}
						</NavButton>
					{/each}
				</div>
				{#if setupMode}
					<div class="mt-4 space-y-2 rounded-md border border-surface-low px-3 py-2">
						<p class="text-label-sm text-on-surface/70">Before recording you will need:</p>
						<p class={modelReady ? 'text-label-sm text-on-surface' : 'text-label-sm text-on-surface/50'}>
							{modelReady ? 'Model installed and selected' : 'Model not ready'}
						</p>
						<p class={permissionsReady ? 'text-label-sm text-on-surface' : 'text-label-sm text-on-surface/50'}>
							{permissionsReady ? 'Microphone granted' : 'Microphone not granted'}
						</p>
					</div>
				{/if}
			</nav>

			<section class="min-h-0 flex-1 overflow-y-auto bg-surface-low p-4">
				{#if currentTab === 'general'}
					<SettingGeneral />
				{:else if currentTab === 'permissions'}
					<SettingPermissions bind:ready={permissionsReady} />
				{:else}
					<SettingModels bind:ready={modelReady} />
				{/if}
			</section>
		</div>

		{#if setupMode && standalone}
			<footer class="flex flex-wrap items-center justify-between gap-3 border-t border-surface-low px-4 py-3">
				<div class="min-w-0 flex-1">
					{#if setupMessage}
						<p class="text-label-sm text-error">{setupMessage}</p>
					{:else}
						<p class="text-label-sm text-on-surface/60">
							{prerequisitesMet
								? 'You can finish setup or continue configuring later — Liscribe will warn you if something is missing when recording.'
								: 'Optional for now — you will see a clear message when you try to record without a model or microphone access.'}
						</p>
					{/if}
				</div>
				<div class="flex shrink-0 flex-wrap items-center gap-2">
					<Button variant="ghost" onclick={configureLater} disabled={finishing}>
						Configure later
					</Button>
					{#if currentTab === 'permissions'}
						<Button variant="secondary" onclick={() => selectTab('models')}>Back</Button>
					{/if}
					{#if currentTab === 'models'}
						<Button variant="primary" disabled={!modelReady || finishing} onclick={() => selectTab('permissions')}>
							Continue
						</Button>
					{:else}
						<Button
							variant="primary"
							disabled={!prerequisitesMet || finishing}
							onclick={finishHighlightSetup}
						>
							{finishing ? 'Saving…' : 'Finish setup'}
						</Button>
					{/if}
				</div>
			</footer>
		{/if}
	</div>
</div>
