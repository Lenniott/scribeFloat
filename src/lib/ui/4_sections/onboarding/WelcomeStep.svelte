<script lang="ts">
	import Button from "../../ui/controls/Button.svelte";

	let {
		onStart,
		onSkip,
	}: {
		onStart: () => void;
		onSkip: () => void;
	} = $props();
</script>

<div class="flex flex-col items-center justify-center h-full gap-8 text-center">
	<div class="flex flex-col items-center gap-3">
		<h1 class="sf-display-lg text-fg leading-tight">
			ScribeFloat
		</h1>
		<p class="sf-body-md-strong text-fg max-w-sm">
			Your voice, transcribed. Privately, on your device.
		</p>
		<p class="sf-body-md text-fg-dim max-w-sm">
			Runs Whisper AI locally.
		</p>
	</div>

	<div class="flex flex-col gap-2 w-full max-w-sm text-left">
		<div class="flex items-start gap-3 rounded-md bg-card border border-fill px-3 py-2.5">
			<span class="sf-label-sm text-fg-dim shrink-0 mt-0.5">Scribe</span>
			<span class="sf-body-md text-fg-dim">Long form transcription with notes</span>
		</div>
		<div class="flex items-start gap-3 rounded-md bg-card border border-fill px-3 py-2.5">
			<span class="sf-label-sm text-fg-dim shrink-0 mt-0.5">Dictate</span>
			<span class="sf-body-md text-fg-dim">Short form transcription that pastes at cursor</span>
		</div>
		<div class="flex items-start gap-3 rounded-md bg-card border border-fill px-3 py-2.5">
			<span class="sf-label-sm text-fg-dim shrink-0 mt-0.5">History</span>
			<span class="sf-body-md text-fg-dim">View past transcripts from scribe and dictate</span>
		</div>
	</div>

	<div class="flex flex-col items-center gap-2">
		<Button variant="primary" onclick={onStart}>Get started</Button>
		<button
			type="button"
			class="sf-label-md text-fg-dim hover:text-fg transition-colors"
			onclick={onSkip}
		>
			Skip to Settings
		</button>
	</div>
</div>
