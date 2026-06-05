<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { onMount } from "svelte";
	import Button from "@lib/components/Button.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
	import StepShell from "./StepShell.svelte";
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
		onNext: (updates: Partial<OnboardingAnswers>) => void;
	} = $props();

	let saveMd = $state(answers.saveMd);

	async function toggleMd(enabled: boolean) {
		saveMd = enabled;
		await invoke("settings_set_save_transcripts_as_markdown", { enabled }).catch(() => {});
	}

	onMount(async () => {
		saveMd = await invoke<boolean>("settings_get_save_transcripts_as_markdown").catch(() => false);
	});
</script>

<StepShell {currentStep} title="Your transcripts" subtitle="Every recording is saved automatically to History.">
	{#snippet children()}
		<div class="space-y-3">
			<!-- Preview card -->
			<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2 pointer-events-none select-none">
				<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Example history entry</p>
				<div class="flex items-start justify-between gap-2">
					<p class="text-body-md text-fg font-medium">Team standup — Monday</p>
					<div class="flex gap-1.5 shrink-0">
						<span class="rounded-sm bg-fill px-1.5 py-0.5 text-label-sm text-fg-dim">Today</span>
						<span class="rounded-sm bg-fill px-1.5 py-0.5 text-label-sm text-fg-dim">142 words</span>
					</div>
				</div>
				<p class="text-label-sm text-fg-dim line-clamp-2">
					Good morning, let's get started. Yesterday I finished the API integration and opened the pull request for review…
				</p>
			</div>

			<!-- What's stored -->
			<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-1">
				<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">What's stored</p>
				<ul class="space-y-1 text-body-md text-fg-dim">
					<li>• Full transcript text with timestamps</li>
					<li>• Metadata: duration, word count, model used</li>
					<li>• All Scribe and Dictate sessions</li>
				</ul>
				<p class="text-label-sm text-fg-dim mt-1">
					Open History from the menu bar icon any time to search, view, export, or delete.
				</p>
			</div>

			<!-- Markdown export toggle -->
			<div class="rounded-md bg-card border border-fill px-3 py-3">
				<div class="flex items-center justify-between gap-3">
					<div class="flex-1 min-w-0">
						<p class="text-body-md text-fg font-medium">Save as Markdown files</p>
						<p class="text-label-sm text-fg-dim">
							Also write a <code class="font-mono bg-fill px-1 rounded-sm">.md</code> file to your save folder for each transcript.
							Opens in any text editor or notes app.
						</p>
					</div>
					<ToggleSwitch checked={saveMd} onchange={toggleMd} aria-label="Save as Markdown" />
				</div>
				{#if saveMd}
					<p class="text-label-sm text-fg-dim mt-2">
						Dictate sessions are always stored in History but are never written as Markdown files.
					</p>
				{/if}
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={() => onNext({ saveMd })}>Continue</Button>
	{/snippet}
</StepShell>
