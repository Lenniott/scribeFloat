<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { X, Copy, SquareArrowOutUpRight, FileDown, Trash2 } from 'lucide-svelte';
	import PanelHeader from '@lib/components/layout/PanelHeader.svelte';
	import FixedFooterBar from '@lib/components/layout/FixedFooterBar.svelte';
	import ScrollablePanel from '@lib/components/accordion/ScrollablePanel.svelte';
	import Button from '@lib/components/Button.svelte';
	import IconButton from '@lib/components/IconButton.svelte';
	import Chip from '@lib/components/Chip.svelte';
	import Modal from '@lib/components/Modal.svelte';
	import Toast from '@lib/components/Toast.svelte';
	import type { ToastState } from '@lib/components/Toast.svelte';

	export type HistoryListItem = {
		id: string;
		kind: string;
		created_at: string;
		title: string;
		model: string;
		word_count: number;
		duration_ms: number;
		has_markdown: boolean;
		markdown_path?: string;
		source: string;
	};

	let {
		item,
		onclose,
		onrefresh,
	}: {
		item: HistoryListItem;
		onclose: () => void;
		onrefresh: () => void;
	} = $props();

	let bodyText = $state('');
	let loadingBody = $state(true);
	let showDeleteModal = $state(false);
	let deleting = $state(false);

	let toastMessage = $state('');
	let toastState = $state<ToastState>('normal');
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;

	// Scribe and Transcribe use a full-height reader layout; Dictate uses the compact layout.
	const isReader = $derived(item.kind === 'scribe' || item.kind === 'transcribe');

	const showExport = $derived(
		item.source === 'store' && item.kind !== 'dictate' && !item.has_markdown,
	);
	const showOpenMd = $derived(item.has_markdown && !!item.markdown_path);
	const showDelete = $derived(item.source === 'store');

	function showToast(msg: string, state: ToastState = 'normal') {
		if (toastTimeout) clearTimeout(toastTimeout);
		toastMessage = msg;
		toastState = state;
		toastTimeout = setTimeout(() => {
			toastMessage = '';
			toastTimeout = null;
		}, 2500);
	}

	function formatDuration(ms: number): string {
		const totalSec = Math.round(ms / 1000);
		const mins = Math.floor(totalSec / 60);
		const secs = totalSec % 60;
		return `${mins}:${secs.toString().padStart(2, '0')}`;
	}

	async function loadContent() {
		loadingBody = true;
		try {
			bodyText = await invoke<string>('history_render_markdown', { id: item.id });
		} catch {
			bodyText = '';
		} finally {
			loadingBody = false;
		}
	}

	async function copyContent() {
		try {
			await writeText(bodyText);
			showToast('Copied', 'success');
		} catch {
			showToast('Copy failed', 'error');
		}
	}

	async function exportMarkdown() {
		try {
			await invoke<string>('history_export_markdown', { id: item.id });
			onrefresh();
			showToast('Exported', 'success');
		} catch (e) {
			showToast('Export failed: ' + String(e), 'error');
		}
	}

	async function openMarkdown() {
		if (!item.markdown_path) return;
		try {
			await invoke('settings_open_transcript', { filePath: item.markdown_path });
		} catch {
			showToast('Could not open file', 'error');
		}
	}

	async function confirmDelete() {
		deleting = true;
		try {
			await invoke('history_delete', { id: item.id });
			showDeleteModal = false;
			onrefresh();
			onclose();
		} catch (e) {
			showToast('Delete failed: ' + String(e), 'error');
			showDeleteModal = false;
		} finally {
			deleting = false;
		}
	}

	// Reload whenever the item changes
	$effect(() => {
		void item;
		void loadContent();
	});
</script>

<div class="flex min-h-0 flex-1 flex-col">
	{#if isReader}
		<!-- Reader layout: full-height transcript viewer for Scribe / Transcribe -->
		<PanelHeader>
			{#snippet left()}
				<p class="truncate font-mono text-label-md tracking-stamped text-fg/80 uppercase">
					{item.title || 'Detail'}
				</p>
			{/snippet}
			{#snippet right()}
				<div class="flex items-center gap-1">
					{#if showDelete}
						<IconButton
							aria-label="Delete recording"
							icon={Trash2}
							size="small"
							variant="normal"
							onclick={() => (showDeleteModal = true)}
						/>
					{/if}
					{#if showExport}
						<IconButton
							aria-label="Export to Markdown"
							icon={FileDown}
							size="small"
							variant="normal"
							onclick={exportMarkdown}
						/>
					{/if}
					{#if showOpenMd}
						<IconButton
							aria-label="Open Markdown file"
							icon={SquareArrowOutUpRight}
							size="small"
							variant="normal"
							onclick={openMarkdown}
						/>
					{/if}
					<IconButton
						aria-label="Copy transcript"
						icon={Copy}
						size="small"
						variant="normal"
						onclick={copyContent}
					/>
					<IconButton
						aria-label="Close detail"
						icon={X}
						size="small"
						variant="normal"
						onclick={onclose}
					/>
				</div>
			{/snippet}
		</PanelHeader>

		<ScrollablePanel class="px-4 py-3">
			{#if loadingBody}
				<p class="text-label-md text-fg/45">Loading…</p>
			{:else if bodyText}
				<p class="text-body-md whitespace-pre-wrap wrap-break-word text-fg/90">{bodyText}</p>
			{:else}
				<p class="text-label-md text-fg/45">No content available.</p>
			{/if}
		</ScrollablePanel>
	{:else}
		<!-- Compact layout: for Dictate items (short text, keep chips + footer) -->
		<PanelHeader>
			{#snippet left()}
				<p class="truncate font-mono text-label-md tracking-stamped text-fg/80 uppercase">
					{item.title || 'Detail'}
				</p>
			{/snippet}
			{#snippet right()}
				<IconButton aria-label="Close detail" icon={X} size="small" variant="normal" onclick={onclose} />
			{/snippet}
		</PanelHeader>

		<!-- Metadata chips -->
		<div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-card/60 px-4 py-2">
			{#if item.model}
				<Chip variant="brand">{item.model}</Chip>
			{/if}
			{#if item.duration_ms > 0}
				<Chip variant="brand">{formatDuration(item.duration_ms)}</Chip>
			{/if}
			{#if item.word_count > 0}
				<Chip variant="brand">{item.word_count} words</Chip>
			{/if}
			{#if item.source !== 'store'}
				<span class="font-mono text-label-sm tracking-stamped text-fg/45 uppercase">Legacy</span>
			{/if}
		</div>

		<ScrollablePanel class="px-4 py-3">
			{#if loadingBody}
				<p class="text-label-md text-fg/45">Loading…</p>
			{:else if bodyText}
				<p class="text-body-md whitespace-pre-wrap wrap-break-word text-fg/90">{bodyText}</p>
			{:else}
				<p class="text-label-md text-fg/45">No content available.</p>
			{/if}
		</ScrollablePanel>

		<FixedFooterBar>
			{#if showDelete}
				<Button variant="destructive" onclick={() => (showDeleteModal = true)}>
					<Trash2 class="size-4" />
					Delete
				</Button>
			{/if}
			<div class="flex-1"></div>
			{#if showExport}
				<Button variant="normal" onclick={exportMarkdown}>
					<FileDown class="size-4" />
					Export .md
				</Button>
			{/if}
			{#if showOpenMd}
				<Button variant="normal" onclick={openMarkdown}>
					<SquareArrowOutUpRight class="size-4" />
					Open .md
				</Button>
			{/if}
			<Button variant="normal" onclick={copyContent}>
				<Copy class="size-4" />
				Copy
			</Button>
		</FixedFooterBar>
	{/if}
</div>

<Modal
	open={showDeleteModal}
	title="Delete recording?"
	description="This will permanently delete the transcript and any associated audio. This cannot be undone."
	maxWidthClass="max-w-sm"
	onClose={() => (showDeleteModal = false)}
>
	{#snippet footer()}
		<div class="flex gap-3">
			<Button variant="normal" onclick={() => (showDeleteModal = false)}>Cancel</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{deleting ? 'Deleting…' : 'Delete'}
			</Button>
		</div>
	{/snippet}
</Modal>

<Toast message={toastMessage} state={toastState} position="bottom-center" />
