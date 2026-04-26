<script lang="ts">
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import SettingsScreen from "@lib/screens/settings.svelte";

	async function closeSettings() {
		await getCurrentWindow().close();
	}
</script>

<SettingsScreen standalone onClose={closeSettings} />
