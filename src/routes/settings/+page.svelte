<script lang="ts">
	import { appState } from '@lib/stores/appState.svelte';
	import SettingsPanel from '@lib/components/sections/SettingsPanel.svelte';
</script>

<SettingsPanel bind:activeTab={appState.settingsTab} />
