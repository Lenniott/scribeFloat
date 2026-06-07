<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import Button from "@lib/components/Button.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
	import StepShell from "./StepShell.svelte";
	import { isWindows } from "$lib/platform";
	import type { OnboardingAnswers } from "$lib/types";

	let {
		currentStep,
		answers,
		onBack,
		onNext,
	}: {
		currentStep: number;
		answers: OnboardingAnswers;
		onBack: () => void;
		onNext: () => void;
	} = $props();

	let speakerCapture = $state(false);
	$effect(() => {
		speakerCapture = answers.speakerCapture;
	});
	let blackholeDetected = $state(false);
	let requiresDeviceName = $state(false);
	let saveMd = $state(false);
	let saveFolder = $state("~/Documents/transcripts_scribefloat");

	async function toggleSpeaker(enabled: boolean) {
		speakerCapture = enabled;
		await invoke("settings_set_scribe_capture_speaker", { enabled }).catch(() => {});
	}

	async function toggleMd(enabled: boolean) {
		saveMd = enabled;
		await invoke("settings_set_save_transcripts_as_markdown", { enabled }).catch(() => {});
	}

	onMount(async () => {
		[requiresDeviceName, blackholeDetected, saveFolder, saveMd] = await Promise.all([
			invoke<boolean>("settings_speaker_capture_requires_device_name").catch(() => false),
			invoke<boolean>("settings_blackhole_detected").catch(() => false),
			invoke<string>("settings_get_output_path").catch(() => saveFolder),
			invoke<boolean>("settings_get_save_transcripts_as_markdown").catch(() => false),
		]);
	});
</script>

<StepShell {currentStep} title="Set up Scribe" subtitle="Record your mic (and optionally computer audio) and get a transcript.">
	{#snippet children()}
		<div class="space-y-3">
			<!-- How to use -->
			<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-1.5">
				<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">How it works</p>
				<ol class="space-y-1 text-body-md text-fg-dim list-decimal list-inside">
					<li>Open Scribe from the menu bar icon</li>
					<li>Click <strong class="text-fg">Start Recording</strong> and speak</li>
					<li>Click <strong class="text-fg">Stop & Save</strong> — transcript appears in History</li>
				</ol>
			</div>

			<!-- Speaker capture -->
			<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2">
				<div class="flex items-center justify-between gap-3">
					<div class="flex-1 min-w-0">
						<p class="text-body-md text-fg font-medium">Capture computer audio</p>
						<p class="text-label-sm text-fg-dim">Record mic + system audio for calls and meetings.</p>
					</div>
					<ToggleSwitch checked={speakerCapture} onchange={toggleSpeaker} aria-label="Capture computer audio" />
				</div>

				{#if speakerCapture}
					{#if !isWindows && requiresDeviceName}
						{#if blackholeDetected}
							<p class="text-label-sm text-success">BlackHole detected — speaker capture ready.</p>
						{:else}
							<div class="rounded-sm bg-fill border border-warning/30 px-2.5 py-2 space-y-1">
								<p class="text-label-sm text-warning">BlackHole not detected</p>
								<p class="text-label-sm text-fg-dim">
									macOS requires the free <strong>BlackHole 2ch</strong> virtual audio driver.
									Install it, then set it up in Settings → General.
								</p>
							</div>
						{/if}
					{:else if isWindows}
						<p class="text-label-sm text-fg-dim">Windows captures system audio automatically — no setup needed.</p>
					{/if}
				{/if}
			</div>

			<!-- Save folder -->
			<div class="rounded-md bg-card border border-fill px-3 py-3">
				<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70 mb-1">Transcripts saved to</p>
				<p class="text-body-md text-fg font-mono truncate">{saveFolder}</p>
				<p class="text-label-sm text-fg-dim mt-0.5">Change this in Settings → General.</p>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={onNext}>Continue</Button>
	{/snippet}
</StepShell>
