<script lang="ts">
	import { appState } from '@stores/appState.svelte';
	import SettingsPanel from '@sections/SettingsPanel.svelte';
</script>

<SettingsPanel bind:activeTab={appState.settingsTab} />
