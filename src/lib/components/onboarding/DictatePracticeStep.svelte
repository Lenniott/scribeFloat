<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import Button from "@lib/components/Button.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
	import HotkeyCaptureField from "@lib/components/form/HotkeyCaptureField.svelte";
	import StepShell from "./StepShell.svelte";
	import { dictateModifierLabel } from "$lib/platform";

	let {
		currentStep,
		onBack,
		onNext,
	}: {
		currentStep: number;
		onBack: () => void;
		onNext: () => void;
	} = $props();

	let dictateHotkey = $state("Ctrl");
	let autoPaste = $state(true);
	let autoEnter = $state(false);

	async function saveHotkey(value: string) {
		if (!value) return;
		const [openHotkey] = await invoke<[string, string]>("settings_get_hotkeys").catch(() => [""]);
		await invoke("settings_set_hotkeys", { openScribe: openHotkey, dictate: value }).catch(() => {});
	}

	async function togglePaste(enabled: boolean) {
		autoPaste = enabled;
		await invoke("settings_set_dictate_auto_paste", { enabled }).catch(() => {});
	}

	async function toggleEnter(enabled: boolean) {
		autoEnter = enabled;
		await invoke("settings_set_dictate_auto_enter", { enabled }).catch(() => {});
	}

	onMount(async () => {
		const [[, hotkey], paste, enter] = await Promise.all([
			invoke<[string, string]>("settings_get_hotkeys").catch(() => ["", "Ctrl"]),
			invoke<boolean>("settings_get_dictate_auto_paste").catch(() => true),
			invoke<boolean>("settings_get_dictate_auto_enter").catch(() => false),
		]);
		dictateHotkey = hotkey;
		autoPaste = paste;
		autoEnter = enter;
	});
</script>

<StepShell {currentStep} title="Set up Dictate" subtitle="Hold a hotkey, speak, release — text appears wherever your cursor is.">
	{#snippet children()}
		<div class="space-y-3">
			<!-- How to use -->
			<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-1.5">
				<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">How it works</p>
				<ol class="space-y-1 text-body-md text-fg-dim list-decimal list-inside">
					<li>Tap <strong class="text-fg">{dictateModifierLabel}</strong>, release, tap again</li>
					<li>Hold for push-to-talk (or release quickly for toggle mode)</li>
					<li>Speak — then release the key (or tap again) to stop</li>
				</ol>
			</div>

			<!-- Hotkey -->
			<div class="rounded-md bg-card border border-fill px-3 py-3">
				<HotkeyCaptureField
					label="Dictate hotkey"
					bind:value={dictateHotkey}
					allowModifierOnly={true}
				/>
				<p class="text-label-sm text-fg-dim mt-1.5">
					Tap twice: first tap arms it, second tap (hold or toggle) starts recording.
				</p>
				{#if dictateHotkey !== "Ctrl"}
					<button type="button" class="mt-1.5 text-label-sm text-fg-dim hover:text-fg transition-colors" onclick={() => saveHotkey(dictateHotkey)}>
						Save hotkey
					</button>
				{/if}
			</div>

			<!-- Toggles -->
			<div class="rounded-md bg-card border border-fill divide-y divide-fill">
				<div class="flex items-center justify-between gap-3 px-3 py-3">
					<div class="flex-1 min-w-0">
						<p class="text-body-md text-fg font-medium">Auto-paste</p>
						<p class="text-label-sm text-fg-dim">Paste text at cursor after dictating.</p>
					</div>
					<ToggleSwitch checked={autoPaste} onchange={togglePaste} aria-label="Auto-paste" />
				</div>
				<div class="flex items-center justify-between gap-3 px-3 py-3">
					<div class="flex-1 min-w-0">
						<p class="text-body-md text-fg font-medium">Press Enter after</p>
						<p class="text-label-sm text-fg-dim">Useful for messaging apps and search bars.</p>
					</div>
					<ToggleSwitch checked={autoEnter} onchange={toggleEnter} aria-label="Press Enter after dictate" />
				</div>
			</div>

			{#if !autoPaste}
				<p class="text-label-sm text-fg-dim px-1">
					Auto-paste is off — dictated text will be copied to your clipboard instead.
				</p>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={onNext}>Continue</Button>
	{/snippet}
</StepShell>
