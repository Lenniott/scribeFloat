<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import NavButton from '@components/NavButton.svelte';
	import SettingsPanel from '@lib/components/settings/SettingsPanel.svelte';
	import { SETTINGS_TABS, type SettingsTab } from '@lib/components/settings/settingsTypes';

	let { standalone = false }: { standalone?: boolean } = $props();

	let activeTab = $state<SettingsTab>('general');
</script>

<div class={standalone ? 'min-h-screen bg-card' : 'sf-scrim fixed inset-0 z-50 p-4'}>
	<div
		class={standalone
			? 'mx-auto flex min-h-screen max-w-5xl flex-col bg-panel'
			: 'mx-auto flex h-full max-w-5xl flex-col bg-panel'}
	>
		<div class="flex h-full min-h-0 overflow-hidden">
			<nav class="flex h-full min-h-0 w-52 shrink-0 flex-col border-r border-card p-2">
				<div class="flex flex-col gap-1">
					{#each SETTINGS_TABS as tab (tab.id)}
						<NavButton active={activeTab === tab.id} onclick={() => (activeTab = tab.id)}>
							{tab.label}
						</NavButton>
					{/each}
				</div>
			</nav>

			<div class="flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
				<SettingsPanel bind:activeTab />
			</div>
		</div>
	</div>
</div>
