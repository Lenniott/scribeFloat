<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onDestroy, onMount } from "svelte";
	import Button from "@lib/components/Button.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";
	import NoteComposer from "@lib/components/notes/NoteComposer.svelte";
	import NoteCard, { type Note } from "@lib/components/notes/NoteCard.svelte";
	import StepShell from "./StepShell.svelte";
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
	type DictateStateEvent = { state: DictateState; text?: string };

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
	}

	async function toggleEnter(enabled: boolean) {
		autoEnter = enabled;
		await invoke("settings_set_dictate_auto_enter", { enabled }).catch(() => {});
	}

	onMount(async () => {
		autoEnter = await invoke<boolean>("settings_get_dictate_auto_enter").catch(() => false);

		const ul = await listen<DictateStateEvent>("dictate://state-changed", (e) => {
			if (e.payload.state === "DONE" && e.payload.text) {
				noteDraft = e.payload.text;
				if (autoEnter) {
					addNote();
				}
			}
		});
		unlisteners = [ul];
	});

	onDestroy(() => unlisteners.forEach((u) => u()));
</script>

<StepShell
	title="Try Dictate"
	subtitle="Double-tap {dictateModifierLabel}, speak, then release. If text input is focused, text will appear at your cursor. Text is saved to history and copied to clipboard."
>
	{#snippet children()}
		<div class="flex gap-3 h-full min-h-0">
			<!-- Left: instructions + toggle -->
			<div class="flex flex-col gap-3 w-72 shrink-0 justify-between">
				<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2">
					<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">How to use</p>
					<ol class="space-y-1.5 text-body-md text-fg-dim list-decimal list-inside">
						<li>Click the text area</li>
						<li>Double-tap <strong class="text-fg">{dictateModifierLabel}</strong></li>
						<li>Hold and speak, or tap again to toggle</li>
						<li>Release (or tap) to stop</li>
					</ol>
				</div>

				<div class="rounded-md px-3 py-3 h-22 flex items-center justify-between">
					<div class="flex flex-col">
						<p class="text-body-md text-fg font-medium">Auto enter</p>
						<p class="text-label-sm text-fg-dim">Manage in Settings.</p>
					</div>
					<ToggleSwitch checked={autoEnter} onchange={toggleEnter} aria-label="Auto enter after dictate" />
				</div>
			</div>

			<!-- Right: note cards + composer -->
			<div class="flex flex-col flex-1 min-h-0 gap-2">
				<div class="flex-1 min-h-0">
					<div class="overflow-y-auto space-y-2">
						{#each notes as note (note.id)}
							<NoteCard {note} />
						{/each}
					</div>
					{#if notes.length === 0}
						<div class="flex items-center justify-center h-full">
							<p class="text-label-md text-fg/40 text-center px-3">
								Dictated notes will appear here
							</p>
						</div>
					{/if}
				</div>
				<NoteComposer
					bind:value={noteDraft}
					placeholder="Click here and test dictate"
					onSubmit={addNote}
				/>
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" onclick={onNext}>Continue</Button>
	{/snippet}
</StepShell>
