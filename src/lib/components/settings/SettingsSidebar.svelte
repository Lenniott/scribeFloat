<script lang="ts">
	import { ArrowLeft } from 'lucide-svelte';
	import NavButton from '@components/NavButton.svelte';
	import { SETTINGS_TABS, type SettingsTab } from './settingsTypes';

	let {
		activeTab,
		ontabchange,
		onback,
		backLabel = 'Back',
	}: {
		activeTab: SettingsTab;
		ontabchange: (tab: SettingsTab) => void;
		onback: () => void;
		backLabel?: string;
	} = $props();
</script>

<aside class="flex h-full min-h-0 w-44 shrink-0 flex-col border-r border-card bg-panel p-2.5">
	<button
		type="button"
		class="flex cursor-pointer items-center gap-2 rounded-md px-2 py-2 sf-label-md text-fg-dim transition-colors hover:bg-fill hover:text-fg"
		onclick={onback}
	>
		<ArrowLeft class="h-4 w-4 shrink-0" aria-hidden="true" />
		{backLabel}
	</button>

	<div class="my-3 border-t border-card"></div>

	<p class="sf-section-label mb-2 px-2 text-fg-dim">Settings</p>
	<div class="flex flex-col gap-0.5">
		{#each SETTINGS_TABS as tab (tab.id)}
			<NavButton active={activeTab === tab.id} onclick={() => ontabchange(tab.id)}>
				{tab.label}
			</NavButton>
		{/each}
	</div>
</aside>
