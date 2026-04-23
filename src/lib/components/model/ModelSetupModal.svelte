<script lang="ts">
	import Button from '@components/Button.svelte';
	import type { ModelListItem } from '$lib/types';

	export type { ModelListItem };

	let {
		open = false,
		models = [],
		progressByModel = {},
		downloadingByModel = {},
		statusByModel = {},
		errorMessage = '',
		canClose = false,
		onDownload,
		onSelect,
		onClose
	}: {
		open?: boolean;
		models?: ModelListItem[];
		progressByModel?: Record<string, number>;
		downloadingByModel?: Record<string, boolean>;
		statusByModel?: Record<string, string>;
		errorMessage?: string;
		canClose?: boolean;
		onDownload?: (modelId: string) => void;
		onSelect?: (modelId: string) => void;
		onClose?: () => void;
	} = $props();
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
		<div class="w-full max-w-2xl rounded-xl border border-surface-container-high bg-surface-container-lowest px-5 py-4 shadow-xl">
			<div class="mb-2 flex items-center justify-between">
				<div>
					<h2 class="text-title-md font-semibold text-on-surface">Model setup</h2>
					<p class="text-body-sm text-on-surface/70">
						Download and choose a model. Downloaded models stay on this machine.
					</p>
				</div>
				<Button variant="secondary" disabled={!canClose} onclick={onClose}>Close</Button>
			</div>

			{#if errorMessage}
				<p class="mb-3 rounded-md bg-error-container/20 px-3 py-2 text-body-sm text-error">
					{errorMessage}
				</p>
			{/if}

			<div class="space-y-3">
				{#each models as model (model.id)}
					<div class="rounded-md border border-surface-container-high px-3 py-3">
						<div class="mb-2 flex items-center justify-between gap-2">
							<div class="min-w-0">
						<p class="text-label-md font-semibold text-on-surface">{model.label}</p>
							</div>
							<div class="flex items-center gap-2">
								{#if model.downloaded}
									<Button variant={model.selected ? 'primary' : 'secondary'} onclick={() => onSelect?.(model.id)}>
										{model.selected ? 'Selected' : 'Use model'}
									</Button>
								{:else}
									<Button
										variant="primary"
										disabled={!!downloadingByModel[model.id]}
										onclick={() => onDownload?.(model.id)}
									>
										{downloadingByModel[model.id] ? 'Installing…' : 'Install'}
									</Button>
								{/if}
							</div>
						</div>

						{#if !model.downloaded && (downloadingByModel[model.id] || (progressByModel[model.id] ?? 0) > 0)}
							<div class="h-2 w-full overflow-hidden rounded bg-surface-container-high">
								<div
									class="h-full bg-primary transition-all"
									style={`width:${Math.round((progressByModel[model.id] ?? 0) * 100)}%`}
								></div>
							</div>
							<p class="mt-1 text-label-sm text-on-surface/70">
								{Math.round((progressByModel[model.id] ?? 0) * 100)}%
							</p>
						{/if}
						{#if statusByModel[model.id]}
							<p class="mt-1 text-label-sm text-on-surface/70">{statusByModel[model.id]}</p>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}
