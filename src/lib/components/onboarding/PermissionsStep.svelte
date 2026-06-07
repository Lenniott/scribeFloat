<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import Button from "@lib/components/Button.svelte";
	import StepShell from "./StepShell.svelte";
	import { CircleCheckBig, Circle } from "lucide-svelte";
	import type { PermissionStatus } from "$lib/types";

	let {
		currentStep,
		onBack,
		onNext,
	}: {
		currentStep: number;
		onBack: () => void;
		onNext: () => void;
	} = $props();

	let statuses = $state<PermissionStatus[]>([]);
	let requestingKind = $state<string | null>(null);

	const mic = $derived(statuses.find((s) => s.kind === "microphone"));
	const accessibility = $derived(statuses.find((s) => s.kind === "accessibility"));
	const inputMonitoring = $derived(statuses.find((s) => s.kind === "input_monitoring"));
	const micGranted = $derived(mic?.granted ?? false);

	const PERMISSION_LABELS: Record<string, string> = {
		microphone: "Microphone",
		accessibility: "Accessibility",
		input_monitoring: "Input Monitoring",
	};

	// Shown before the user clicks Grant — explains the benefit, not just the requirement.
	const PERMISSION_PRIMERS: Record<string, string> = {
		microphone: "ScribeFloat records audio on your device only. Nothing is uploaded. Grant access so we can hear you.",
		accessibility: "This lets Dictate paste transcribed text at your cursor. Without it, text goes to your clipboard instead.",
		input_monitoring: "Required to listen for the Dictate hotkey while another app is in focus.",
	};

	async function refresh() {
		statuses = await invoke<PermissionStatus[]>("settings_permissions_status").catch(() => []);
	}

	async function grant(kind: string) {
		requestingKind = kind;
		await invoke("settings_permissions_request", { kind }).catch(() => {});
		await refresh();
		requestingKind = null;
	}

	let pollId: ReturnType<typeof setInterval>;
	let unlistenFocus: (() => void) | undefined;

	onMount(async () => {
		await refresh();
		pollId = setInterval(refresh, 5000);
		unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
			if (focused) refresh();
		});
	});

	onDestroy(() => {
		clearInterval(pollId);
		unlistenFocus?.();
	});
</script>

<StepShell {currentStep} title="Grant permissions" subtitle="ScribeFloat needs microphone access to record. Accessibility lets Dictate paste text automatically.">
	{#snippet children()}
		<div class="space-y-2">
			{#each statuses.filter(s => ["microphone", "accessibility", "input_monitoring"].includes(s.kind)) as status (status.kind)}
				<div class="rounded-md bg-card border border-fill px-3 py-3">
					<div class="flex items-start justify-between gap-3">
						<div class="flex-1 min-w-0">
							<p class="text-label-md font-mono tracking-stamped uppercase text-fg">
								{PERMISSION_LABELS[status.kind] ?? status.kind}
								{#if status.kind === "microphone"}
									<span class="text-destructive ml-1 normal-case font-sans tracking-normal">required</span>
								{:else}
									<span class="text-fg-muted ml-1 normal-case font-sans tracking-normal">optional</span>
								{/if}
							</p>
							{#if !status.granted && status.can_request}
								<p class="text-body-md text-fg-dim mt-0.5">{PERMISSION_PRIMERS[status.kind] ?? ""}</p>
							{:else if status.granted && status.hint}
								<p class="text-label-sm text-fg-dim mt-1">{status.hint}</p>
							{/if}
						</div>
						<div class="shrink-0">
							{#if status.granted}
								<div class="flex items-center gap-1.5 text-success">
									<CircleCheckBig class="size-4" />
									<span class="text-label-sm">Granted</span>
								</div>
							{:else if status.can_request}
								<Button
									variant="normal"
									size="small"
									disabled={requestingKind === status.kind}
									onclick={() => grant(status.kind)}
								>
									{requestingKind === status.kind ? "Requesting…" : "Grant"}
								</Button>
							{:else}
								<div class="flex items-center gap-1.5 text-fg-muted">
									<Circle class="size-4" />
									<span class="text-label-sm">Not available</span>
								</div>
							{/if}
						</div>
					</div>
				</div>
			{/each}

			{#if !micGranted}
				<p class="text-label-sm text-warning px-1">
					Microphone access is required to continue.
				</p>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<Button variant="ghost" onclick={onBack}>Back</Button>
		<Button variant="primary" disabled={!micGranted} onclick={onNext}>Continue</Button>
	{/snippet}
</StepShell>
