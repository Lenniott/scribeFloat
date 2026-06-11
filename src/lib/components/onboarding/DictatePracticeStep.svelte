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
	type DictateStateEvent = { state: DictateState; text?: string; error?: string };

	let dictateState = $state<DictateState>("IDLE");
	let dictateError = $state("");

	const MAX_NOTES = 2;

	const isActive = $derived(
		dictateState === "RECORDING" || dictateState === "TRANSCRIBING" || dictateState === "PASTING",
	);

	const statusLabel = $derived(
		dictateState === "RECORDING"
			? "Recording…"
			: dictateState === "TRANSCRIBING" || dictateState === "PASTING"
				? "Transcribing…"
				: "",
	);

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
				if (e.payload.text) {
					noteDraft = e.payload.text;
					if (autoEnter) addNote();
				} else {
					dictateError = "Nothing was heard. Try speaking clearly for at least a second.";
				}
			} else if (e.payload.state === "ERROR") {
				dictateError = e.payload.error ?? "Dictation failed. Check that a model is installed.";
			} else if (e.payload.state === "IDLE" && prev === "TRANSCRIBING") {
				// empty segments path — controller went TRANSCRIBING → IDLE without DONE
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
			<!-- Left: instructions + toggle -->
			<div class="flex flex-col gap-3 w-72 shrink-0 justify-between">
				<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2">
					<p class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">How to use</p>
					<ol class="space-y-1.5 text-body-md text-fg-dim list-decimal list-inside">
						<li>Click the text area on the right</li>
						<li>Double-tap <strong class="text-fg">{dictateModifierLabel}</strong></li>
						<li>Speak clearly for 2+ seconds</li>
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
				<div class="flex-1 min-h-0 relative">
					<div class="overflow-y-auto space-y-2">
						{#each notes as note (note.id)}
							<NoteCard {note} />
						{/each}
					</div>
					{#if notes.length === 0}
						<div class="flex items-center justify-center h-full">
							<p class="text-label-md text-fg/40 text-center px-3">
								{#if isActive}
									<span class="text-brand animate-pulse">{statusLabel}</span>
								{:else}
									Dictated notes will appear here
								{/if}
							</p>
						</div>
					{/if}
				</div>

				{#if dictateError}
					<p class="text-label-sm text-destructive px-1">{dictateError}</p>
				{/if}

				{#if isActive}
					<div
						class="flex items-center gap-2 rounded-md border border-fill bg-fill px-3 py-2 text-label-sm text-fg-dim"
					>
						<span class="size-2 rounded-full bg-brand animate-pulse shrink-0"></span>
						{statusLabel}
					</div>
				{/if}

				<!-- Always mounted so manual draft is preserved between dictations -->
				<div class={isActive ? "hidden" : ""}>
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
