<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { X as Close } from "lucide-svelte";
	import Waveform from "@components/indicators/Waveform.svelte";
	import AnimatedEllipsis from "@primitives/display/AnimatedEllipsis.svelte";
	import RecordingStatusDot from "@primitives/display/StatusDot.svelte";
	import RecordingTimer from "@primitives/display/RecordingTimer.svelte";
	import IconButton from "@components/controls/IconButton.svelte";
	import ProgressBar from "@primitives/display/ProgressBar.svelte";
	import { DICTATE_STEPS, type ProcessingStage } from "@utils/processingFeedback";
	import { createCaptureProgress } from "@stores/captureProgress.svelte";

	type DictateState = "IDLE" | "RECORDING" | "TRANSCRIBING" | "PASTING" | "DONE" | "ERROR";

	type DictateStateEvent = {
		state: DictateState;
		progress?: number;
		/** Shared capture vocabulary; Dictate only ever receives the first two stages. */
		processing_stage?: ProcessingStage;
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
	let progress = 0;
	let processingStage: ProcessingStage = "LOADING_MODEL";

	let recordingStartedAt: number | null = null;
	let timerInterval: ReturnType<typeof setInterval> | undefined;
	let micLevelPending = 0;
	let micFlushRaf = 0;

	const capture = createCaptureProgress(DICTATE_STEPS, {
		estimateKey: "dictate-transcribe",
	});

	/** Rough transcribe+paste estimate (model load is a separate loading phase);
	 * the store corrects it from past runs. */
	function processingHintSeconds(recordedSeconds: number): number {
		return 0.8 + 0.2 * recordedSeconds;
	}

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

		if (ev.state === "TRANSCRIBING" && prev === "RECORDING") {
			capture.begin(processingHintSeconds(elapsedSeconds));
		}
		if (ev.progress != null) {
			progress = ev.progress;
		}
		if (ev.processing_stage) {
			processingStage = ev.processing_stage;
		}
		if (ev.progress != null || ev.processing_stage) {
			capture.update(processingStage, progress);
		}

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
				progress = 0;
				processingStage = "LOADING_MODEL";
				capture.reset();
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
		capture.reset();
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
			{#if capture.loading}
				<span class="sf-body-md text-fg-dim"
					>{capture.stageLabel}<AnimatedEllipsis /></span
				>
			{:else}
				<ProgressBar progress={capture.percentExact} fluid />
			{/if}
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
