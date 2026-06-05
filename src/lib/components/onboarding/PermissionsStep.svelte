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

	const PERMISSION_HINTS: Record<string, string> = {
		microphone: "Required to record audio.",
		accessibility: "Required for Dictate to paste text automatically.",
		input_monitoring: "Required to listen for the Dictate hotkey.",
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
							<p class="text-body-md text-fg-dim mt-0.5">{PERMISSION_HINTS[status.kind] ?? ""}</p>
							{#if !status.granted && status.hint}
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
