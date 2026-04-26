<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { watchThemeMode, type ThemeMode } from "$lib/theme";
	import "../app.css";

	let { children } = $props();

	onMount(() => {
		let cleanup = watchThemeMode("system");
		let mounted = true;
		invoke<ThemeMode>("settings_get_theme_mode")
			.catch(() => "system" as const)
			.then((themeMode) => {
				if (!mounted) return;
				cleanup();
				cleanup = watchThemeMode(themeMode);
			});
		return () => {
			mounted = false;
			cleanup();
		};
	});
</script>

{@render children()}