<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { X as Close } from "lucide-svelte";
	import Waveform from "@lib/components/ui/indicators/Waveform.svelte";
	import RecordingStatusDot from "@lib/components/primitives/display/StatusDot.svelte";
	import RecordingTimer from "@lib/components/primitives/display/RecordingTimer.svelte";
	import IconButton from "@lib/components/ui/controls/IconButton.svelte";

	type DictateState = "IDLE" | "RECORDING" | "TRANSCRIBING" | "PASTING" | "DONE" | "ERROR";

	type DictateStateEvent = {
		state: DictateState;
		progress?: number;
		text?: string;
		paste_failed?: boolean;
		history_write_failed?: boolean;
		error?: string;
	};

	let dictateState = $state<DictateState>("IDLE");
	let micLevel = $state(0);
	let elapsedSeconds = $state(0);
	let resultText = $state("");
	let pasteFailed = $state(false);
	let historyWriteFailed = $state(false);
	let errorText = $state("");

	let recordingStartedAt: number | null = null;
	let timerInterval: ReturnType<typeof setInterval> | undefined;
	let micLevelPending = 0;
	let micFlushRaf = 0;

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

	function stopRecordingTimer() {
		if (timerInterval !== undefined) {
			clearInterval(timerInterval);
			timerInterval = undefined;
		}
	}

	function flushMicLevel() {
		micFlushRaf = 0;
		micLevel = micLevelPending;
	}

	function scheduleMicLevelFlush() {
		if (micFlushRaf) return;
		micFlushRaf = requestAnimationFrame(flushMicLevel);
	}

	function handleStateEvent(ev: DictateStateEvent) {
		const prev = dictateState;
		dictateState = ev.state;

		if (ev.state === "DONE") {
			resultText = ev.text ?? "";
			pasteFailed = Boolean(ev.paste_failed);
			historyWriteFailed = Boolean(ev.history_write_failed);
		} else if (ev.state === "ERROR") {
			errorText = ev.error ?? "Something went wrong.";
		}

		if (ev.state === "RECORDING" && prev !== "RECORDING") {
			recordingStartedAt = Date.now();
			elapsedSeconds = 0;
			stopRecordingTimer();
			timerInterval = setInterval(() => {
				if (recordingStartedAt !== null) {
					elapsedSeconds = (Date.now() - recordingStartedAt) / 1000;
				}
			}, 250);
		} else if (ev.state !== "RECORDING") {
			stopRecordingTimer();
			if (ev.state === "IDLE") {
				elapsedSeconds = 0;
				recordingStartedAt = null;
				resultText = "";
				pasteFailed = false;
				historyWriteFailed = false;
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
				micLevelPending = e.payload;
				scheduleMicLevelFlush();
			}),
		);
	});

	onDestroy(() => {
		stopRecordingTimer();
		if (micFlushRaf) {
			cancelAnimationFrame(micFlushRaf);
			micFlushRaf = 0;
		}
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
	class="flex w-60 items-center justify-between gap-2 rounded-md bg-panel py-2 pl-3 pr-2 shadow-ambient"
>
	{#if dictateState === "DONE"}
		<div class="flex min-w-0 flex-1 flex-col gap-0.5">
			<div class="flex min-w-0 items-center gap-2">
				<span class="inline-block h-2 w-2 shrink-0 rounded-full bg-success"></span>
				{#if pasteFailed}
					<span class="truncate sf-body-md-strong text-fg"
						>Copied to clipboard</span
					>
				{:else}
					<span class="truncate sf-body-md-strong text-fg">{resultText}</span>
				{/if}
			</div>
			{#if pasteFailed && resultText.trim()}
				<p class="line-clamp-2 pl-4 sf-label-sm text-fg-dim">{resultText}</p>
			{/if}
			{#if historyWriteFailed}
				<p class="pl-4 sf-label-sm text-fg-muted">History entry could not be saved — check save folder.</p>
			{/if}
		</div>
	{:else if dictateState === "ERROR"}
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<RecordingStatusDot status="error" />
			<span class="truncate sf-body-md-strong text-destructive"
				>{errorText}</span
			>
		</div>
	{:else if isProcessing}
		<div class="flex min-w-0 flex-1 items-center gap-2">
			<span class="sf-label-sm text-fg-dim">
				{dictateState === "PASTING" ? "Pasting…" : "Transcribing…"}
			</span>
		</div>
	{:else}
		<div class="flex items-center gap-4">
			<div class="flex items-center gap-2">
				<RecordingStatusDot status={dotStatus} pulseWhileRecording={false} />
				<RecordingTimer {elapsedSeconds} />
			</div>
			<Waveform
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
		aria-label={
			dictateState === "DONE" || dictateState === "ERROR"
				? "Dismiss"
				: "Cancel dictation"
		}
		onclick={handleClose}
	/>
</div>
