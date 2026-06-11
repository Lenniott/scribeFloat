<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { emit } from "@tauri-apps/api/event";
	import Button from "@lib/components/Button.svelte";
	import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
	import OptionGroup from "@lib/components/form/OptionGroup.svelte";
	import PathSelectorField from "@lib/components/form/PathSelectorField.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
	import { applyThemeMode, type ThemeMode } from "$lib/theme";
	import { dictateModifierLabel } from "$lib/platform";
	import { appErrorMessage } from "$lib/types";

	let {
		savedSpeakerDeviceName = "",
		blackholeDetected = false,
		speakerCaptureRequiresDeviceName = false,
		onSpeakerConfigSaved,
	}: {
		savedSpeakerDeviceName?: string;
		blackholeDetected?: boolean;
		speakerCaptureRequiresDeviceName?: boolean;
		onSpeakerConfigSaved?: (name: string) => void;
	} = $props();

	let outputPath = $state("");
	let openHotkey = $state("");
	let dictateHotkey = $state("");
	let inputLabel = $state("");
	let outputLabel = $state("");
	let preferredInputDevice = $state("");
	let preferredSpeakerDevice = $state("");
	let scribeCaptureSpeaker = $state(false);
	let dictateAutoEnter = $state(false);
	let keepWav = $state(false);
	let saveTranscriptsAsMarkdown = $state(false);
	let themeMode = $state<ThemeMode>("system");
	let openWithApp = $state("");
	let message = $state("");
	let loadError = $state("");
	let messageClearId: ReturnType<typeof setTimeout> | undefined;

	const speakerCaptureAvailable = $derived(
		!speakerCaptureRequiresDeviceName ||
			(blackholeDetected && savedSpeakerDeviceName.trim().length > 0),
	);

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
		loadError = "";
		try {
			const [
				nextOutputPath,
				[open, dictate],
				[inLabel, outLabel],
				nextThemeMode,
				nextOpenWithApp,
				[preferredInput, preferredSpeaker],
				nextScribeCaptureSpeaker,
				nextDictateAutoEnter,
				nextKeepWav,
				nextSaveTranscriptsAsMarkdown,
			] = await Promise.all([
				invoke<string>("settings_get_output_path"),
				invoke<[string, string]>("settings_get_hotkeys"),
				invoke<[string, string]>("settings_get_input_labels"),
				invoke<ThemeMode>("settings_get_theme_mode"),
				invoke<string | null>("settings_get_open_with_app_path"),
				invoke<[string | null, string | null]>("settings_get_preferred_audio_devices"),
				invoke<boolean>("settings_get_scribe_capture_speaker"),
				invoke<boolean>("settings_get_dictate_auto_enter"),
				invoke<boolean>("settings_get_keep_wav"),
				invoke<boolean>("settings_get_save_transcripts_as_markdown"),
			]);

			outputPath = nextOutputPath;
			openHotkey = open;
			dictateHotkey = dictate;
			inputLabel = inLabel;
			outputLabel = outLabel;
			themeMode = nextThemeMode;
			openWithApp = nextOpenWithApp ?? "";
			preferredInputDevice = preferredInput ?? "";
			preferredSpeakerDevice = preferredSpeaker ?? "";
			scribeCaptureSpeaker = nextScribeCaptureSpeaker;
			dictateAutoEnter = nextDictateAutoEnter;
			keepWav = nextKeepWav;
			saveTranscriptsAsMarkdown = nextSaveTranscriptsAsMarkdown;
		} catch (e) {
			loadError = `Could not load settings: ${appErrorMessage(e)}`;
		}
	}

	async function saveAll() {
		if (messageClearId !== undefined) {
			clearTimeout(messageClearId);
			messageClearId = undefined;
		}
		message = "";
		try {
			const trimmedSpeaker = preferredSpeakerDevice.trim();
			const captureAvailableAfterSave =
				!speakerCaptureRequiresDeviceName || (blackholeDetected && trimmedSpeaker.length > 0);
			const captureDefault = captureAvailableAfterSave && scribeCaptureSpeaker;
			await invoke("settings_save_general", {
				payload: {
					outputPath,
					openHotkey,
					dictateHotkey,
					inputLabel,
					outputLabel,
					preferredInputDevice: preferredInputDevice.trim() || null,
					preferredSpeakerDevice: trimmedSpeaker || null,
					scribeCaptureSpeaker,
					speakerCaptureAvailable: captureAvailableAfterSave,
					dictateAutoEnter,
					keepWav,
					saveTranscriptsAsMarkdown,
					themeMode,
					openWithAppPath: openWithApp.trim() || null,
				},
			});
			scribeCaptureSpeaker = captureDefault;
			preferredSpeakerDevice = trimmedSpeaker;
			onSpeakerConfigSaved?.(trimmedSpeaker);
			await emit("settings://speaker-capture-saved");
			message = "Saved";
			messageClearId = setTimeout(() => {
				message = "";
				messageClearId = undefined;
			}, 3000);
		} catch (e) {
			message = `Failed to save: ${appErrorMessage(e)}`;
		}
	}

	onMount(refresh);
	onDestroy(() => {
		if (messageClearId !== undefined) clearTimeout(messageClearId);
	});
</script>

<section class="space-y-4">
	<h2 class="sf-headline-sm">General settings</h2>

	{#if loadError}
		<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 text-label-sm text-destructive">
			{loadError}
		</p>
	{/if}

	<!-- Keyboard shortcuts — moved to top for quick reference -->
	<div class="flex flex-col gap-2">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Keyboard shortcuts</span>
		<div class="rounded-md bg-fill px-3 py-2 flex flex-col gap-0.5">
			<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/60 uppercase">Open Scribe</span>
			<p class="text-label-sm text-fg/80">
				<code class="font-mono bg-card px-1 rounded">{openHotkey}</code> — opens the Scribe panel from anywhere. Fixed; cannot be changed here.
			</p>
		</div>
		<div class="rounded-md bg-fill px-3 py-2 flex flex-col gap-0.5">
			<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/60 uppercase">Dictate</span>
			<p class="text-label-sm text-fg/80">
				Tap left <strong>{dictateModifierLabel}</strong>, release, tap again — hold ≥½s for push-to-talk (release stops), or quick tap–release toggles mic on/off; press again to stop. Fixed; cannot be changed here.
			</p>
		</div>
	</div>

	<OptionGroup name="theme-mode" label="Theme" options={themeOptions} bind:selected={themeMode} />
	<PathSelectorField label="Default save folder" bind:path={outputPath} />

	<!-- Save transcripts as Markdown, with dependent option directly below -->
	<div class="flex flex-col items-start justify-center gap-1">
		<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
			Save transcripts as Markdown
		</span>
		<ToggleSwitch checked={saveTranscriptsAsMarkdown} aria-label="Save transcripts as Markdown" onchange={(next) => (saveTranscriptsAsMarkdown = next)} />
		<p class="text-label-sm text-fg/50">Off by default — history is always saved; Markdown is an extra export.</p>
	</div>
	{#if saveTranscriptsAsMarkdown}
		<PathSelectorField
			label="Open transcripts with"
			bind:path={openWithApp}
			directory={false}
		/>
	{/if}

	<!-- Speaker capture settings — macOS: device name always visible so it can be configured;
	     labels and toggle only shown once capture is available (BlackHole named on macOS, always on Windows). -->
	{#if speakerCaptureRequiresDeviceName}
		<LabeledTextField
			label="Speaker capture device name"
			bind:value={preferredSpeakerDevice}
			placeholder="Type the exact Audio MIDI device name"
		/>
	{/if}
	{#if speakerCaptureAvailable}
		<LabeledTextField label="Input label" bind:value={inputLabel} />
		<LabeledTextField label="Output label" bind:value={outputLabel} />
		<div class="flex flex-col items-start justify-center gap-1 h-10">
			<span class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">
				Capture speaker by default
			</span>
			<ToggleSwitch checked={scribeCaptureSpeaker} aria-label="Toggle default speaker capture" onchange={(next) => (scribeCaptureSpeaker = next)} />
		</div>
	{/if}

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

	<div class="flex items-center gap-3">
		<Button variant="primary" onclick={saveAll}>Save</Button>
		{#if message}
			<p class="text-label-sm text-fg/70">{message}</p>
		{/if}
	</div>
</section>
