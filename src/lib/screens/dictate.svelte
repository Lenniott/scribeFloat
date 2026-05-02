<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { listen } from "@tauri-apps/api/event";
	import { X as Close } from "lucide-svelte";
	import AudioWaveFormVisualizer from "@lib/components/audio/AudioWaveFormVisualizer.svelte";
	import RecordingStatusDot from "@lib/components/audio/RecordingStatusDot.svelte";
	import RecordingTimer from "@lib/components/audio/RecordingTimer.svelte";
	import IconButton from "@lib/components/IconButton.svelte";

	type DictateState = "IDLE" | "RECORDING" | "TRANSCRIBING" | "PASTING" | "ERROR";

	type DictateStateEvent = {
		state: DictateState;
		progress?: number;
		error?: string;
	};

	let dictateState = $state<DictateState>("IDLE");
	let micLevel = $state(0);
	let elapsedSeconds = $state(0);

	let recordingStartedAt: number | null = null;
	let timerRaf = 0;

	const dotStatus = $derived(
		dictateState === "RECORDING"
			? "recording"
			: dictateState === "ERROR"
				? "error"
				: dictateState === "TRANSCRIBING" || dictateState === "PASTING"
					? "paused"
					: "idle",
	);

	const isProcessing = $derived(dictateState === "TRANSCRIBING" || dictateState === "PASTING");

	function tickTimer() {
		if (recordingStartedAt !== null && dictateState === "RECORDING") {
			elapsedSeconds = (Date.now() - recordingStartedAt) / 1000;
			timerRaf = requestAnimationFrame(tickTimer);
		}
	}

	function handleStateEvent(ev: DictateStateEvent) {
		const prev = dictateState;
		dictateState = ev.state;

		if (ev.state === "RECORDING" && prev !== "RECORDING") {
			recordingStartedAt = Date.now();
			elapsedSeconds = 0;
			cancelAnimationFrame(timerRaf);
			timerRaf = requestAnimationFrame(tickTimer);
		} else if (dictateState !== "RECORDING") {
			cancelAnimationFrame(timerRaf);
			if (dictateState === "IDLE" || dictateState === "ERROR") {
				elapsedSeconds = 0;
				recordingStartedAt = null;
			}
		}
	}

	async function handleCancel() {
		try {
			await invoke("dictate_cancel");
		} catch {
			// Window will be hidden by backend after cancel; ignore errors.
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
	<div class="flex items-center gap-4">
		<div class="flex items-center gap-2">
			<RecordingStatusDot status={dotStatus} />
			<RecordingTimer {elapsedSeconds} />
		</div>
		<AudioWaveFormVisualizer {micLevel} speakerLevel={0} speakerEnabled={false} size="small" />
	</div>
	<IconButton
		variant="normal"
		size="small"
		icon={Close}
		aria-label="Cancel dictation"
		onclick={handleCancel}
		disabled={isProcessing}
	/>
</div>
