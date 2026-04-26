<script lang="ts">
	import NavButton from '@lib/components/NavButton.svelte';
	import SettingGeneral from '@lib/screens/setting_general.svelte';
	import SettingPermissions from '@lib/screens/setting_permissions.svelte';
	import SettingModels from '@lib/screens/setting_models.svelte';
	import IconButton from '@lib/components/IconButton.svelte';
	import { X } from 'lucide-svelte';

	type SettingsTab = 'general' | 'permissions' | 'models';

	let { onClose }: { onClose?: () => void } = $props();
	let activeTab = $state<SettingsTab>('general');

	const tabs: Array<{ id: SettingsTab; label: string }> = [
		{ id: 'general', label: 'General' },
		{ id: 'permissions', label: 'Permissions' },
		{ id: 'models', label: 'Models' },
	];
</script>

<div class="fixed inset-0 z-50 bg-black/50 p-4">
	<div class="mx-auto flex h-full max-w-5xl flex-col rounded-lg bg-surface-container-lowest shadow-lg">
		<header class="flex items-center justify-between border-b border-surface-container-low px-4 py-3">
			<h2 class="text-title-md font-normal tracking-tight">Settings</h2>
			<IconButton aria-label="close settings" variant="normal" icon={X} onclick={() => onClose?.()}/>
		</header>

		<div class="flex min-h-0 flex-1">
			<nav class="w-52 border-r border-surface-container-low p-2">
				<div class="flex flex-col gap-1">
					{#each tabs as tab (tab.id)}
						<NavButton
							active={activeTab === tab.id}
							onclick={() => (activeTab = tab.id)}
						>
							{tab.label}
						</NavButton>
					{/each}
				</div>
			</nav>

			<section class="min-h-0 flex-1 overflow-y-auto p-4 bg-surface-container-low">
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
