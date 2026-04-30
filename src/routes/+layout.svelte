<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { watchThemeMode, applyThemeMode, type ThemeMode } from "$lib/theme";
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

		// Sync theme changes made in other windows (e.g. settings window → scribe window).
		// The storage event fires in all windows except the one that wrote the value.
		function onStorage(e: StorageEvent) {
			if (e.key === "sf_theme_mode" && e.newValue) {
				cleanup();
				cleanup = watchThemeMode(e.newValue as ThemeMode);
			}
		}
		window.addEventListener("storage", onStorage);

		return () => {
			mounted = false;
			cleanup();
			window.removeEventListener("storage", onStorage);
		};
	});
</script>

{@render children()}