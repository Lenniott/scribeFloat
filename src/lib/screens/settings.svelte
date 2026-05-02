<script lang="ts">
	import NavButton from '@components/NavButton.svelte';
	import SettingGeneral from '@lib/screens/setting_general.svelte';
	import SettingPermissions from '@lib/screens/setting_permissions.svelte';
	import SettingModels from '@lib/screens/setting_models.svelte';
	import IconButton from '@components/IconButton.svelte';
	import { X } from 'lucide-svelte';

	type SettingsTab = 'general' | 'permissions' | 'models';

	let {
		onClose,
		standalone = false,
	}: {
		onClose?: () => void;
		standalone?: boolean;
	} = $props();

	let activeTab = $state<SettingsTab>('general');
	let permissionsReady = $state(false);
	let modelReady = $state(false);

	const tabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'general', label: 'General' },
		{ id: 'permissions', label: 'Permissions' },
		{ id: 'models', label: 'Models' },
	];
</script>

<div class={standalone ? 'min-h-screen bg-card' : 'fixed inset-0 z-50 bg-black/50 p-4'}>
	<div class="mx-auto flex h-screen max-w-5xl flex-col bg-panel shadow-lg">
		<header class="flex items-center justify-between border-b border-card px-4 py-3">
			<h2 class="sf-headline-sm">Settings</h2>
			<IconButton aria-label="close settings" variant="normal" icon={X} onclick={() => onClose?.()} />
		</header>

		{#if !permissionsReady || !modelReady}
			<div class="flex flex-col gap-1 border-b border-card bg-fill px-4 py-2">
				{#if !permissionsReady}
					<p class="text-label-sm text-fg/70">
						Microphone access needed —
						<button class="underline" onclick={() => (activeTab = 'permissions')}>go to Permissions</button>.
					</p>
				{/if}
				{#if !modelReady}
					<p class="text-label-sm text-fg/70">
						No transcription model installed —
						<button class="underline" onclick={() => (activeTab = 'models')}>go to Models</button>.
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

			<section class="min-h-0 flex-1 overflow-y-auto bg-card p-4">
				{#if activeTab === 'general'}
					<SettingGeneral />
				{:else if activeTab === 'permissions'}
					<SettingPermissions bind:ready={permissionsReady} />
				{:else}
					<SettingModels bind:ready={modelReady} />
				{/if}
			</section>
		</div>
	</div>
</div>
