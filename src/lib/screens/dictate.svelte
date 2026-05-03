<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { X as Close } from "lucide-svelte";
	import AudioWaveFormVisualizer from "@lib/components/audio/AudioWaveFormVisualizer.svelte";
	import RecordingStatusDot from "@lib/components/audio/RecordingStatusDot.svelte";
	import RecordingTimer from "@lib/components/audio/RecordingTimer.svelte";
	import IconButton from "@lib/components/IconButton.svelte";

	type DictateState = "IDLE" | "RECORDING" | "TRANSCRIBING" | "PASTING" | "DONE" | "ERROR";

	type DictateStateEvent = {
		state: DictateState;
		progress?: number;
		text?: string;
		error?: string;
	};

	let dictateState = $state<DictateState>("IDLE");
	let micLevel = $state(0);
	let elapsedSeconds = $state(0);
	let resultText = $state("");
	let errorText = $state("");

	let recordingStartedAt: number | null = null;
	let timerRaf = 0;

	const dotStatus = $derived(
		dictateState === "RECORDING"
			? "recording"
			: dictateState === "ERROR"
				? "error"
				: "idle",
	);

	const isProcessing = $derived(
		dictateState === "TRANSCRIBING" || dictateState === "PASTING",
	);

	function tickTimer() {
		if (recordingStartedAt !== null && dictateState === "RECORDING") {
			elapsedSeconds = (Date.now() - recordingStartedAt) / 1000;
			timerRaf = requestAnimationFrame(tickTimer);
		}
	}

	function handleStateEvent(ev: DictateStateEvent) {
		const prev = dictateState;
		dictateState = ev.state;

		if (ev.state === "DONE") {
			resultText = ev.text ?? "";
		} else if (ev.state === "ERROR") {
			errorText = ev.error ?? "Something went wrong.";
		}

		if (ev.state === "RECORDING" && prev !== "RECORDING") {
			recordingStartedAt = Date.now();
			elapsedSeconds = 0;
			cancelAnimationFrame(timerRaf);
			timerRaf = requestAnimationFrame(tickTimer);
		} else if (ev.state !== "RECORDING") {
			cancelAnimationFrame(timerRaf);
			if (ev.state === "IDLE") {
				elapsedSeconds = 0;
				recordingStartedAt = null;
				resultText = "";
				errorText = "";
			}
		}
	}

	async function handleClose() {
		try {
			if (dictateState === "DONE" || dictateState === "ERROR") {
				await invoke("dictate_dismiss");
			} else {
				await invoke("dictate_cancel");
			}
		} catch {
			// Backend hides the window; ignore errors.
		}
	}

	const unlisten: (() => void)[] = [];

	onMount(async () => {
		unlisten.push(
			await listen<DictateStateEvent>("dictate://state-changed", (e) => {
				handleStateEvent(e.payload);
			}),
		);
		unlisten.push(
			await listen<number>("dictate://audio-level", (e) => {
				micLevel = e.payload;
			}),
		);
	});

	onDestroy(() => {
		cancelAnimationFrame(timerRaf);
		unlisten.forEach((u) => u());
	});
</script>

<svelte:head>
	<style>
		html,
		body {
			background: transparent;
			margin: 0;
			overflow: hidden;
		}
	</style>
</svelte:head>

<div
	class="flex w-60 items-center justify-between gap-2 rounded-lg bg-panel py-2 pl-3 pr-2 shadow-lg"
>
	{#if dictateState === "DONE"}
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<span class="inline-block h-2 w-2 flex-shrink-0 rounded-full bg-success"></span>
			<span class="truncate text-label-md font-sans font-medium text-fg">{resultText}</span>
		</div>
	{:else if dictateState === "ERROR"}
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<RecordingStatusDot status="error" />
			<span class="truncate text-label-md font-sans font-medium text-destructive"
				>{errorText}</span
			>
		</div>
	{:else if isProcessing}
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<span class="text-label-sm font-sans uppercase tracking-stamped text-fg-dim">
				{dictateState === "PASTING" ? "Pasting…" : "Transcribing…"}
			</span>
		</div>
	{:else}
		<div class="flex items-center gap-4">
			<div class="flex items-center gap-2">
				<RecordingStatusDot status={dotStatus} />
				<RecordingTimer {elapsedSeconds} />
			</div>
			<AudioWaveFormVisualizer
				{micLevel}
				speakerLevel={0}
				speakerEnabled={false}
				size="small"
			/>
		</div>
	{/if}

	<IconButton
		variant="normal"
		size="small"
		icon={Close}
		aria-label={dictateState === "DONE" ? "Dismiss" : "Cancel dictation"}
		onclick={handleClose}
		disabled={isProcessing}
	/>
</div>
