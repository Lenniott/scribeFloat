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

	const tabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'general', label: 'General' },
		{ id: 'permissions', label: 'Permissions' },
		{ id: 'models', label: 'Models' },
	];
</script>

<div class={standalone ? 'min-h-screen bg-surface-low' : 'fixed inset-0 z-50 bg-black/50 p-4'}>
	<div class="mx-auto flex h-screen max-w-5xl flex-col bg-surface-lowest shadow-lg">
		<header class="flex items-center justify-between border-b border-surface-low px-4 py-3">
			<h2 class="sf-headline-sm">Settings</h2>
			<IconButton aria-label="close settings" variant="normal" icon={X} onclick={() => onClose?.()} />
		</header>

		<div class="flex min-h-0 h-full">
			<nav class="w-52 border-r border-surface-low p-2">
				<div class="flex flex-col gap-1">
					{#each tabs as tab (tab.id)}
						<NavButton active={activeTab === tab.id} onclick={() => (activeTab = tab.id)}>
							{tab.label}
						</NavButton>
					{/each}
				</div>
			</nav>

			<section class="min-h-0 flex-1 overflow-y-auto bg-surface-low p-4">
				{#if activeTab === 'general'}
					<SettingGeneral />
				{:else if activeTab === 'permissions'}
					<SettingPermissions />
				{:else}
					<SettingModels />
				{/if}
			</section>
		</div>
	</div>
</div>
