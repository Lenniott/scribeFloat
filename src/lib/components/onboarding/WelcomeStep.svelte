<script lang="ts">
	import Button from "@lib/components/Button.svelte";

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
		<h1 class="font-mono text-display-lg tracking-stamped uppercase font-normal text-fg leading-tight">
			ScribeFloat
		</h1>
		<p class="text-body-lg text-fg max-w-sm font-medium">
			Your voice, transcribed. Privately, on your device.
		</p>
		<p class="text-body-md text-fg-dim max-w-sm">
			ScribeFloat runs Whisper AI locally — nothing leaves your computer.
		</p>
	</div>

	<div class="flex flex-col gap-2 w-full max-w-xs text-left">
		<div class="flex items-start gap-3 rounded-md bg-card border border-fill px-3 py-2.5">
			<span class="font-mono text-label-sm tracking-stamped uppercase text-brand shrink-0 mt-0.5">Scribe</span>
			<span class="text-body-md text-fg-dim">Turn meetings and voice memos into searchable transcripts</span>
		</div>
		<div class="flex items-start gap-3 rounded-md bg-card border border-fill px-3 py-2.5">
			<span class="font-mono text-label-sm tracking-stamped uppercase text-brand shrink-0 mt-0.5">Dictate</span>
			<span class="text-body-md text-fg-dim">Speak anywhere, text appears at your cursor</span>
		</div>
		<div class="flex items-start gap-3 rounded-md bg-card border border-fill px-3 py-2.5">
			<span class="font-mono text-label-sm tracking-stamped uppercase text-brand shrink-0 mt-0.5">History</span>
			<span class="text-body-md text-fg-dim">Every transcript saved, searchable, and exportable</span>
		</div>
	</div>

	<div class="flex flex-col items-center gap-2">
		<Button variant="primary" onclick={onStart}>Get started</Button>
		<button
			type="button"
			class="text-label-md text-fg-dim hover:text-fg transition-colors"
			onclick={onSkip}
		>
			Skip to Settings
		</button>
	</div>
</div>
