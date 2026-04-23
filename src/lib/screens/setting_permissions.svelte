<script lang="ts">
	import { onMount } from "svelte";
	import { invoke } from "@tauri-apps/api/core";
	import Button from "@lib/components/Button.svelte";

	type PermissionStatus = {
		kind: string;
		granted: boolean;
		can_request: boolean;
	};

	let statuses = $state<PermissionStatus[]>([]);

	async function refresh() {
		statuses = await invoke<PermissionStatus[]>("settings_permissions_status").catch(() => []);
	}

	async function openSettings(kind: string) {
		await invoke("settings_permissions_open", { kind }).catch(() => {});
		await refresh();
	}

	onMount(refresh);
</script>

<section class="space-y-3">
	<h2 class="text-title-sm font-semibold">Permissions</h2>
	{#each statuses as status (status.kind)}
		<div class="flex items-center justify-between rounded-md bg-surface-container-low p-3">
			<p class="text-body-sm capitalize">{status.kind.replace("_", " ")}</p>
			{#if status.granted}
				<span class="text-label-sm text-green-500">Granted</span>
			{:else if status.can_request}
				<Button variant="secondary" onclick={() => openSettings(status.kind)}>Open settings</Button>
			{:else}
				<span class="text-label-sm text-on-surface/60">Not supported</span>
			{/if}
		</div>
	{/each}
</section>