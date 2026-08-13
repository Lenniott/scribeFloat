<script lang="ts">
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { onDestroy, onMount } from "svelte";
	import { SvelteSet } from "svelte/reactivity";
	import { CircleCheckBig, Circle } from "lucide-svelte";
	import Button from "@components/controls/Button.svelte";
	import ToggleSwitch from "@components/controls/Toggle.svelte";
	import NoteComposer from "@patterns/NoteComposer.svelte";
	import NoteCard, { type Note } from "@components/cards/InlineNote.svelte";
	import StepShell from "@primitives/layout/StepFrame.svelte";
	import { dictateModifierLabel } from '@utils/platform';

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
	let notesScrollEl = $state<HTMLDivElement | null>(null);
	let unlisteners: (() => void)[] = [];

	type DictateState = "IDLE" | "RECORDING" | "TRANSCRIBING" | "PASTING" | "DONE" | "ERROR";
	type DictateGesture = "double_tap" | "hold";
	type DictateStateEvent = {
		state: DictateState;
		text?: string;
		error?: string;
		gesture?: DictateGesture;
	};

	let dictateState = $state<DictateState>("IDLE");
	let dictateError = $state("");
	const triedGestures = new SvelteSet<DictateGesture>();

	const MAX_NOTES = 2;

	function addNote() {
		const text = noteDraft.trim();
		if (!text) return;
		const next: Note[] = [
			...notes,
			{ id: crypto.randomUUID(), text, recordedAtMs: Date.now() },
		].slice(-MAX_NOTES);
		notes = next;
		noteDraft = "";
		dictateError = "";
		queueMicrotask(() => {
			if (notesScrollEl) {
				notesScrollEl.scrollTop = notesScrollEl.scrollHeight;
			}
		});
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

			if (e.payload.state === "RECORDING" && e.payload.gesture) {
				triedGestures.add(e.payload.gesture);
			}

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
	subtitle="Double-tap {dictateModifierLabel} to toggle, or press and hold to talk. Text is saved to history and copied to clipboard."
>
	{#snippet children()}
		<div class="flex gap-3 h-full min-h-0">
			<div class="flex flex-col gap-3 w-72 shrink-0 justify-between">
				<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2">
					<p class="sf-section-label text-fg-dim">How to use</p>
					<ol class="space-y-1.5 sf-body-md text-fg-dim list-decimal list-inside">
						<li>Click the text area on the right</li>
						<li>Double-tap, or press and hold, <strong class="text-fg">{dictateModifierLabel}</strong></li>
						<li>Speak clearly for 2+ seconds</li>
						<li>Release (or tap) to stop</li>
					</ol>
				</div>

				<div class="rounded-md bg-card border border-fill px-3 py-3 space-y-2">
					<p class="sf-section-label text-fg-dim">Gestures tried</p>
					<div class="space-y-1.5">
						<div class="flex items-center gap-1.5 {triedGestures.has('double_tap') ? 'text-success' : 'text-fg-muted'}">
							{#if triedGestures.has('double_tap')}
								<CircleCheckBig class="size-4" />
							{:else}
								<Circle class="size-4" />
							{/if}
							<span class="sf-label-sm">Double-tap</span>
						</div>
						<div class="flex items-center gap-1.5 {triedGestures.has('hold') ? 'text-success' : 'text-fg-muted'}">
							{#if triedGestures.has('hold')}
								<CircleCheckBig class="size-4" />
							{:else}
								<Circle class="size-4" />
							{/if}
							<span class="sf-label-sm">Press-and-hold</span>
						</div>
					</div>
				</div>

				<div class="rounded-md px-3 py-3 h-22 flex items-center justify-between">
					<div class="flex flex-col">
						<p class="sf-body-md-strong text-fg">Auto enter</p>
						<p class="sf-label-sm text-fg-dim">Manage in Settings.</p>
					</div>
					<ToggleSwitch checked={autoEnter} onchange={toggleEnter} aria-label="Auto enter after dictate" />
				</div>
			</div>

			<div class="flex min-h-0 flex-1 flex-col gap-2">
				<!-- List region fills leftover space; cards themselves stay content-sized. -->
				<div class="relative min-h-0 flex-1 overflow-y-auto" bind:this={notesScrollEl}>
					{#if notes.length === 0}
						<div class="flex h-full items-center justify-center">
							<p class="sf-label-md px-3 text-center text-fg-muted">
								Dictated notes will appear here
							</p>
						</div>
					{:else}
						<ul class="flex flex-col justify-start gap-2">
							{#each notes as note (note.id)}
								<li class="w-full shrink-0">
									<NoteCard {note} maxLines={2} />
								</li>
							{/each}
						</ul>
					{/if}
				</div>

				{#if dictateError}
					<p class="shrink-0 px-1 sf-label-sm text-destructive">{dictateError}</p>
				{/if}

				<div class="shrink-0">
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
