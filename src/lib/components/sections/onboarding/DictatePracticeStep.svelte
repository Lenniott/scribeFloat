<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onDestroy, onMount } from "svelte";
	import Button from "../../ui/controls/Button.svelte";
	import ToggleSwitch from "../../ui/controls/Toggle.svelte";
	import NoteComposer from "../../patterns/NoteComposer.svelte";
	import NoteCard, { type Note } from "../../ui/cards/NoteSnippet.svelte";
	import StepShell from "../../primitives/layout/StepFrame.svelte";
	import { dictateModifierLabel } from "$lib/platform";

	let {
		onBack,
		onNext,
	}: {
		onBack: () => void;
		onNext: () => void;
	} = $props();

	let autoEnter = $state(false);
	let noteDraft = $state("");
	let notes = $state<Note[]>([]);
	let unlisteners: (() => void)[] = [];

	type DictateState = "IDLE" | "RECORDING" | "TRANSCRIBING" | "PASTING" | "DONE" | "ERROR";
	type DictateStateEvent = { state: DictateState; text?: string; error?: string };

	let dictateState = $state<DictateState>("IDLE");
	let dictateError = $state("");

	const MAX_NOTES = 2;

	function addNote() {
		const text = noteDraft.trim();
		if (!text) return;
		const next: Note[] = [
			{ id: crypto.randomUUID(), text, recordedAtMs: Date.now() },
			...notes,
		].slice(0, MAX_NOTES);
		notes = next;
		noteDraft = "";
		dictateError = "";
	}

	async function toggleEnter(enabled: boolean) {
		autoEnter = enabled;
		await invoke("settings_set_dictate_auto_enter", { enabled }).catch(() => {});
	}

	onMount(async () => {
		autoEnter = await invoke<boolean>("settings_get_dictate_auto_enter").catch(() => false);

		const ul = await listen<DictateStateEvent>("dictate://state-changed", (e) => {
			const prev = dictateState;
			dictateState = e.payload.state;

			if (e.payload.state === "DONE") {
				dictateError = "";
				if (!e.payload.text) {
					dictateError = "Nothing was heard. Try speaking clearly for at least a second.";
				}
			} else if (e.payload.state === "ERROR") {
				dictateError = e.payload.error ?? "Dictation failed. Check that a model is installed.";
			} else if (e.payload.state === "IDLE" && prev === "TRANSCRIBING") {
				if (!noteDraft) {
					dictateError = "Nothing was heard. Try speaking clearly for at least a second.";
				}
			}
		});
		unlisteners = [ul];
	});

	onDestroy(() => unlisteners.forEach((u) => u()));
</script>

<StepShell
	title="Try Dictate"
	subtitle="Double-tap {dictateModifierLabel}, speak, then release. Text is saved to history and copied to clipboard."
>
	{#snippet children()}
		<div class="flex gap-3 h-full min-h-0">
			<div class="flex flex-col gap-3 w-72 shrink-0 justify-between">
				<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2">
					<p class="sf-section-label text-fg-dim">How to use</p>
					<ol class="space-y-1.5 sf-body-md text-fg-dim list-decimal list-inside">
						<li>Click the text area on the right</li>
						<li>Double-tap <strong class="text-fg">{dictateModifierLabel}</strong></li>
						<li>Speak clearly for 2+ seconds</li>
						<li>Release (or tap) to stop</li>
					</ol>
				</div>

				<div class="rounded-md px-3 py-3 h-22 flex items-center justify-between">
					<div class="flex flex-col">
						<p class="sf-body-md-strong text-fg">Auto enter</p>
						<p class="sf-label-sm text-fg-dim">Manage in Settings.</p>
					</div>
					<ToggleSwitch checked={autoEnter} onchange={toggleEnter} aria-label="Auto enter after dictate" />
				</div>
			</div>

			<div class="flex flex-col flex-1 min-h-0 gap-2">
				<div class="relative min-h-0 flex-1">
					<div class="space-y-2">
						{#each notes as note (note.id)}
							<NoteCard {note} />
						{/each}
					</div>
					{#if notes.length === 0}
						<div class="flex items-center justify-center h-full">
							<p class="sf-label-md text-fg-muted text-center px-3">
								Dictated notes will appear here
							</p>
						</div>
					{/if}
				</div>

				{#if dictateError}
					<p class="sf-label-sm text-destructive px-1">{dictateError}</p>
				{/if}

				<div>
					<NoteComposer
						bind:value={noteDraft}
						placeholder="Click here and test dictate"
						focusOnMount={true}
						onSubmit={addNote}
					/>
				</div>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={onNext}>Continue</Button>
	{/snippet}
</StepShell>
