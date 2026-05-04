<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { watchThemeMode, applyThemeMode, type ThemeMode } from "$lib/theme";
	import "../app.css";

	let { children } = $props();

	onMount(() => {
		document.getElementById('sf-loading')?.remove();
		// Transparent dictate HUD: WebKit can spam prefers-color-scheme "change" → theme flicker.
		const trackSystemScheme =
			new URLSearchParams(window.location.search).get("view") !== "dictate";

		// Use localStorage to match the boot script's initial theme, avoiding a flash before
		// the backend invoke resolves.
		const localMode = (localStorage.getItem('sf_theme_mode') as ThemeMode | null) ?? "system";
		let cleanup = watchThemeMode(localMode, { trackSystemScheme });
		let mounted = true;
		invoke<ThemeMode>("settings_get_theme_mode")
			.catch(() => "system" as const)
			.then((themeMode) => {
				if (!mounted) return;
				cleanup();
				cleanup = watchThemeMode(themeMode, { trackSystemScheme });
			});

		// Sync theme changes made in other windows (e.g. settings window → scribe window).
		// The storage event fires in all windows except the one that wrote the value.
		function onStorage(e: StorageEvent) {
			if (e.key === "sf_theme_mode" && e.newValue) {
				cleanup();
				cleanup = watchThemeMode(e.newValue as ThemeMode, { trackSystemScheme });
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