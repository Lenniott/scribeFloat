<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
	import OptionGroup from "@lib/components/form/OptionGroup.svelte";
	import PathSelectorField from "@lib/components/form/PathSelectorField.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
	import { applyThemeMode, type ThemeMode } from "$lib/theme";

	let outputPath = $state("");
	let openHotkey = $state("");
	let dictateHotkey = $state("");
	let inputLabel = $state("");
	let outputLabel = $state("");
	let preferredInputDevice = $state("");
	let preferredSpeakerDevice = $state("");
	let outputDevices = $state<string[]>([]);
	let scribeCaptureSpeaker = $state(false);
	let dictateAutoEnter = $state(false);
	let keepWav = $state(false);
	let themeMode = $state<ThemeMode>("system");
	let openWithApp = $state("");
	let message = $state("");
	let messageClearId: ReturnType<typeof setTimeout> | undefined;

	// Apply theme immediately as the user toggles it (live preview)
	$effect(() => {
		applyThemeMode(themeMode);
	});

	const themeOptions = [
		{ value: "system", label: "System" },
		{ value: "dark", label: "Dark" },
		{ value: "light", label: "Light" },
	];

	async function refresh() {
		outputPath = await invoke<string>("settings_get_output_path").catch(() => "");
		const [open, dictate] = await invoke<[string, string]>("settings_get_hotkeys").catch(() => ["", ""]);
		openHotkey = open;
		dictateHotkey = dictate;
		const [inLabel, outLabel] = await invoke<[string, string]>("settings_get_input_labels").catch(() => [
			"Mic",
			"Speaker",
		]);
		inputLabel = inLabel;
		outputLabel = outLabel;
		themeMode = await invoke<ThemeMode>("settings_get_theme_mode").catch(() => "system");
		openWithApp = (await invoke<string | null>("settings_get_open_with_app_path").catch(() => null)) ?? "";
		const [preferredInput, preferredSpeaker] = await invoke<[string | null, string | null]>(
			"settings_get_preferred_audio_devices",
		).catch(() => [null, null]);
		preferredInputDevice = preferredInput ?? "";
		preferredSpeakerDevice = preferredSpeaker ?? "";
		outputDevices = await invoke<string[]>("settings_list_output_devices").catch(() => []);
		scribeCaptureSpeaker = await invoke<boolean>("settings_get_scribe_capture_speaker").catch(
			() => false,
		);
		dictateAutoEnter = await invoke<boolean>("settings_get_dictate_auto_enter").catch(() => false);
		keepWav = await invoke<boolean>("settings_get_keep_wav").catch(() => false);
	}

	async function saveAll() {
		if (messageClearId !== undefined) {
			clearTimeout(messageClearId);
			messageClearId = undefined;
		}
		message = "";
		try {
			await invoke("settings_set_output_path", { path: outputPath });
			await invoke("settings_set_hotkeys", { openScribe: openHotkey, dictate: dictateHotkey });
			await invoke("settings_set_input_labels", { inputLabel, outputLabel });
			await invoke("settings_set_preferred_audio_devices", {
				preferredInputDevice: preferredInputDevice.trim() || null,
				preferredSpeakerDevice: preferredSpeakerDevice.trim() || null,
			});
			await invoke("settings_set_scribe_capture_speaker", { enabled: scribeCaptureSpeaker });
		await invoke("settings_set_dictate_auto_enter", { enabled: dictateAutoEnter });
			await invoke("settings_set_keep_wav", { enabled: keepWav });
			await invoke("settings_set_theme_mode", { themeMode });
			await invoke("settings_set_open_with_app_path", { path: openWithApp.trim() || null });
			message = "Saved";
			messageClearId = setTimeout(() => {
				message = "";
				messageClearId = undefined;
			}, 3000);
		} catch (e) {
			message = "Failed to save: " + String(e);
		}
	}

	onMount(refresh);
	onDestroy(() => {
		if (messageClearId !== undefined) clearTimeout(messageClearId);
	});
</script>

<section class="space-y-4">
	<h2 class="sf-headline-sm">General settings</h2>
	<OptionGroup name="theme-mode" label="Theme" options={themeOptions} bind:selected={themeMode} />
	<PathSelectorField label="Default save folder" bind:path={outputPath} />
	<PathSelectorField
		label="Open transcripts with"
		bind:path={openWithApp}
		directory={false}
	/>
	<div class="flex flex-col gap-1">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Open Scribe hotkey</span>
		<p class="text-label-sm text-fg/50">
			<code class="font-mono bg-fill px-1 rounded">{openHotkey}</code> — opens the Scribe panel from anywhere. The hotkey is fixed and cannot be changed here.
		</p>
	</div>
	<div class="flex flex-col gap-1">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Dictate hotkey</span>
		<p class="text-label-sm text-fg/50">
			Tap left <strong>Ctrl</strong>, release, then tap <strong>Ctrl</strong> again — hold ~½s for push-to-talk (release stops), or a quick tap–release toggles mic on/off; press <strong>Ctrl</strong> again to stop. Dictate listens to left Ctrl only here; the dictate hotkey is fixed and cannot be changed.
		</p>
	</div>
	<LabeledTextField label="Input label" bind:value={inputLabel} />
	<LabeledTextField label="Output label" bind:value={outputLabel} />
	<div class="flex flex-col items-start justify-center gap-1 h-10">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
			Capture speaker by default
		</span>
		<ToggleSwitch checked={scribeCaptureSpeaker} aria-label="Toggle default speaker capture" onchange={(next) => (scribeCaptureSpeaker = next)} />
	</div>
	<div class="flex flex-col items-start justify-center gap-1 h-10">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
			Press Enter after dictate
		</span>
		<ToggleSwitch checked={dictateAutoEnter} aria-label="Press Enter after dictation paste" onchange={(next) => (dictateAutoEnter = next)} />
	</div>
	<div class="flex flex-col items-start justify-center gap-1 h-10">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
			Keep audio after transcription
		</span>
		<ToggleSwitch checked={keepWav} aria-label="Keep WAV file after transcription" onchange={(next) => (keepWav = next)} />
	</div>
	<LabeledTextField
		label="Speaker capture device name"
		bind:value={preferredSpeakerDevice}
		placeholder="Type the exact Audio MIDI device name"
	/>
	<div class="flex items-center gap-3">
		<Button variant="primary" onclick={saveAll}>Save</Button>
		{#if message}
			<p class="text-label-sm text-fg/70">{message}</p>
		{/if}
	</div>
</section>