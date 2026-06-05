<script lang="ts">
	import Button from "@lib/components/Button.svelte";
	import { Mic, AudioLines, History } from "lucide-svelte";

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
		<p class="text-body-md text-fg-dim max-w-sm">
			Private, on-device voice transcription. Everything stays on your computer — nothing sent to the cloud.
		</p>
	</div>

	<div class="flex gap-4">
		<div class="flex flex-col items-center gap-2 w-32">
			<div class="flex items-center justify-center w-10 h-10 rounded-md bg-card">
				<Mic class="size-5 text-brand" strokeWidth={1.5} />
			</div>
			<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Record & Transcribe</span>
		</div>
		<div class="flex flex-col items-center gap-2 w-32">
			<div class="flex items-center justify-center w-10 h-10 rounded-md bg-card">
				<AudioLines class="size-5 text-brand" strokeWidth={1.5} />
			</div>
			<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Voice Dictate</span>
		</div>
		<div class="flex flex-col items-center gap-2 w-32">
			<div class="flex items-center justify-center w-10 h-10 rounded-md bg-card">
				<History class="size-5 text-brand" strokeWidth={1.5} />
			</div>
			<span class="text-label-sm font-mono tracking-stamped uppercase text-fg/70">Browse History</span>
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
