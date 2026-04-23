<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import Button from "@lib/components/Button.svelte";
	import type { PermissionStatus } from "$lib/types";

	let statuses = $state<PermissionStatus[]>([]);
	let requestingKind = $state<string | null>(null);
	let justGrantedByKind = $state<Record<string, boolean>>({});

	async function refresh() {
		const previous = statuses;
		statuses = await invoke<PermissionStatus[]>("settings_permissions_status").catch(() => []);
		const previousMap = new Map(previous.map((p) => [p.kind, p.granted]));
		for (const status of statuses) {
			if (!previousMap.get(status.kind) && status.granted) {
				justGrantedByKind = { ...justGrantedByKind, [status.kind]: true };
				setTimeout(() => {
					justGrantedByKind = { ...justGrantedByKind, [status.kind]: false };
				}, 3000);
			}
		}
	}

	async function grantPermission(kind: string) {
		requestingKind = kind;
		if (kind === "microphone") {
			try {
				const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
				stream.getTracks().forEach((t) => t.stop());
			} catch {
				// Denied or unavailable — status refresh below reflects reality.
			}
		} else {
			await invoke("settings_permissions_open", { kind }).catch(() => {});
		}
		await refresh();
		requestingKind = null;
	}

	let pollId: ReturnType<typeof setInterval>;
	let unlistenFocus: (() => void) | undefined;

	onMount(async () => {
		await refresh();
		pollId = setInterval(refresh, 3000);
		unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
			if (focused) refresh();
		});
	});

	onDestroy(() => {
		clearInterval(pollId);
		unlistenFocus?.();
	});
</script>

<section class="space-y-3">
	<h2 class="text-title-sm font-semibold">Permissions</h2>
	{#each statuses as status (status.kind)}
		<div
			class={`rounded-md border px-3 py-2.5 transition ${
				status.granted
					? "border-green-500/30 bg-green-500/5"
					: status.can_request
						? "border-amber-500/30 bg-amber-500/5"
						: "border-surface-container bg-surface"
			}`}
		>
			<div class="flex items-center justify-between gap-3">
				<div class="flex items-center gap-2">
					{#if status.granted}
						<svg class="size-4 shrink-0 text-green-500" viewBox="0 0 16 16" fill="currentColor">
							<path fill-rule="evenodd" d="M12.416 3.376a.75.75 0 0 1 .208 1.04l-5 7.5a.75.75 0 0 1-1.154.114l-3-3a.75.75 0 0 1 1.06-1.06l2.353 2.353 4.493-6.74a.75.75 0 0 1 1.04-.207Z" clip-rule="evenodd" />
						</svg>
					{:else if status.can_request}
						<span class="size-4 shrink-0 rounded-full border-2 border-amber-500/60 bg-amber-500/20"></span>
					{:else}
						<span class="size-4 shrink-0 rounded-full border-2 border-surface-container-high"></span>
					{/if}
					<div>
						<p class="text-body-sm capitalize">{status.kind.replace(/_/g, " ")}</p>
						{#if justGrantedByKind[status.kind]}
							<p class="text-label-sm text-green-500">Just granted</p>
						{/if}
					</div>
				</div>
				{#if status.granted}
					<span class="text-label-sm font-medium text-green-500">Granted</span>
				{:else if status.can_request}
					<Button
						variant="secondary"
						disabled={requestingKind === status.kind}
						onclick={() => grantPermission(status.kind)}
					>
						{requestingKind === status.kind ? "Requesting…" : "Grant permission"}
					</Button>
				{:else}
					<span class="text-label-sm text-on-surface/50">Not supported</span>
				{/if}
			</div>
			{#if !status.granted && status.can_request && status.kind !== "microphone"}
				<p class="mt-1.5 text-label-sm text-on-surface/50">
					{status.kind === "accessibility"
						? "System Settings will open → Privacy & Security → Accessibility. Enable the toggle next to this app."
						: "System Settings will open → Privacy & Security → Input Monitoring. Enable the toggle next to this app."}
				</p>
			{/if}
		</div>
	{/each}
</section>