<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";
	import StepShell from "./StepShell.svelte";
	import { CircleCheckBig } from "lucide-svelte";
	import type { ModelListItem, OnboardingAnswers } from "$lib/types";

	let {
		currentStep,
		answers,
		onFinish,
		onOpenSettings,
	}: {
		currentStep: number;
		answers: OnboardingAnswers;
		onFinish: () => void;
		onOpenSettings: () => void;
	} = $props();

	let modelLabel = $state<string | null>(null);

	onMount(async () => {
		if (answers.selectedModelId) {
			const models = await invoke<ModelListItem[]>("model_list").catch(() => []);
			const m = models.find((m) => m.id === answers.selectedModelId);
			modelLabel = m?.label ?? answers.selectedModelId;
		}
	});

	async function tryScribeNow() {
		await invoke("settings_open_scribe_window").catch(() => {});
		onFinish();
	}
</script>

<StepShell {currentStep} title="You're ready!" subtitle="ScribeFloat is set up and ready to use.">
	{#snippet children()}
		<div class="space-y-4">
			<!-- Summary chips -->
			<div class="space-y-2">
				<div class="flex items-center gap-2.5">
					<CircleCheckBig class="size-4 text-success shrink-0" />
					<span class="text-body-md text-fg">Microphone permission granted</span>
				</div>
				{#if answers.selectedModelId}
					<div class="flex items-center gap-2.5">
						<CircleCheckBig class="size-4 text-success shrink-0" />
						<span class="text-body-md text-fg">
							{modelLabel ?? answers.selectedModelId} model installed
						</span>
					</div>
				{:else}
					<div class="flex items-center gap-2.5">
						<span class="size-4 rounded-full border-2 border-warning shrink-0"></span>
						<span class="text-body-md text-fg-dim">No model installed — download one in Settings → Models</span>
					</div>
				{/if}
				<div class="flex items-center gap-2.5">
					<CircleCheckBig class="size-4 text-success shrink-0" />
					<span class="text-body-md text-fg">
						Speaker capture {answers.speakerCapture ? "on" : "off"}
					</span>
				</div>
				<div class="flex items-center gap-2.5">
					<CircleCheckBig class="size-4 text-success shrink-0" />
					<span class="text-body-md text-fg">
						Markdown export {answers.saveMd ? "on" : "off"}
					</span>
				</div>
			</div>

			<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-1">
				<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Quick start</p>
				<ul class="space-y-1 text-body-md text-fg-dim">
					<li>• Click the menu bar icon to open Scribe, History, or Settings</li>
					<li>• Use your Dictate hotkey to voice-input text anywhere</li>
					<li>• All transcripts are in History — searchable and exportable</li>
				</ul>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onOpenSettings}>Open Settings</Button>
		<Button variant="normal" onclick={tryScribeNow}>Try Scribe now</Button>
		<Button variant="primary" onclick={onFinish}>Start using ScribeFloat</Button>
	{/snippet}
</StepShell>
