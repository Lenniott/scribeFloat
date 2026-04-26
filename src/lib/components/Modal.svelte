<script lang="ts">
	import type { Snippet } from 'svelte';
	import IconButton from '@lib/components/IconButton.svelte';
	import { X } from 'lucide-svelte';

	let {
		open = false,
		title,
		description = '',
		maxWidthClass = 'max-w-2xl',
		showCloseButton = true,
		closeDisabled = false,
		onClose,
		children,
		footer,
	}: {
		open?: boolean;
		title: string;
		description?: string;
		maxWidthClass?: string;
		showCloseButton?: boolean;
		closeDisabled?: boolean;
		onClose?: () => void;
		children?: Snippet;
		footer?: Snippet;
	} = $props();
</script>

{#if open}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
		<div
			role="dialog"
			aria-modal="true"
			aria-label={title}
			class={`flex max-h-full w-full ${maxWidthClass} flex-col rounded-xl border border-surface-container-high bg-surface-container-lowest px-5 py-4 shadow-xl`}
		>
			<header class="mb-3 flex items-start justify-between gap-4">
				<div>
					<h2 class="text-title-md font-semibold text-on-surface">{title}</h2>
					{#if description}
						<p class="text-body-sm text-on-surface/70">{description}</p>
					{/if}
				</div>
				{#if showCloseButton}
					<IconButton
						aria-label={`Close ${title}`}
						variant="normal"
						size="small"
						icon={X}
						disabled={closeDisabled}
						onclick={() => onClose?.()}
					/>
				{/if}
			</header>

			<div class="min-h-0 overflow-y-auto">
				{@render children?.()}
			</div>

			{#if footer}
				<footer class="mt-4 flex justify-end">
					{@render footer()}
				</footer>
			{/if}
		</div>
	</div>
{/if}
