<script lang="ts">
	import { CircleHelp, MicVocal, Shield, SlidersHorizontal, Wrench } from 'lucide-svelte';
	import NavItem from '@components/nav/NavItem.svelte';
	import { SETTINGS_TABS, type SettingsTab } from '@sections/settingsTypes';

	const SETTINGS_TAB_ICONS = {
		general: SlidersHorizontal,
		advanced: Wrench,
		voice: MicVocal,
		permissions: Shield,
		help: CircleHelp,
	} as const satisfies Record<SettingsTab, typeof SlidersHorizontal>;

	let {
		activeTab,
		ontabchange,
	}: {
		activeTab: SettingsTab;
		ontabchange: (tab: SettingsTab) => void;
	} = $props();
</script>

<aside class="flex h-full min-h-0 w-44 shrink-0 flex-col border-r border-card bg-panel p-2.5">
	<p class="sf-section-label mb-2 px-2 pt-1 text-fg-dim">Settings</p>
	<div class="flex flex-col gap-0.5">
		{#each SETTINGS_TABS as tab (tab.id)}
			<NavItem
				label={tab.label}
				icon={SETTINGS_TAB_ICONS[tab.id]}
				active={activeTab === tab.id}
				onclick={() => ontabchange(tab.id)}
			/>
		{/each}
	</div>
</aside>
