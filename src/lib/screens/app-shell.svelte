<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import AppSidebar, { type AppRoute } from '@lib/components/regions/AppSidebar.svelte';
	import ShellTitleBar from '@lib/components/regions/TitleBar.svelte';
	import SettingsSidebar from '@lib/components/regions/SettingsSidebar.svelte';
	import SettingsPanel from '@lib/components/sections/SettingsPanel.svelte';
	import type { SettingsTab } from '@lib/components/sections/settingsTypes';
	import HomeScreen from '@lib/screens/home.svelte';
	import NotesScreen from '@lib/screens/notes.svelte';
	import UploadScreen from '@lib/screens/upload.svelte';
	import CaptureScreen from '@lib/screens/capture.svelte';
	import Toast from '@lib/components/ui/indicators/Toast.svelte';
	import type { ToastState } from '@lib/components/ui/indicators/Toast.svelte';
	import Button from '@lib/components/ui/controls/Button.svelte';
	import Modal from '@lib/components/primitives/layout/Modal.svelte';
	import {
		copyHistoryItem,
		deleteHistoryItem,
		openHistoryMarkdown,
		type HistoryListItem,
	} from '@lib/services/historyActions';

	type AppNavigateEvent = {
		route: AppRoute;
		settingsTab?: SettingsTab;
	};

	const ROUTE_LABELS: Record<AppRoute, string> = {
		home: 'Home',
		notes: 'Notes',
		upload: 'Upload',
		float: 'Float',
		settings: 'Settings',
	};

	let route = $state<AppRoute>('home');
	let previousRoute = $state<AppRoute>('home');
	let settingsTab = $state<SettingsTab>('general');
	let captureOpen = $state(false);
	let captureVisitKey = $state(0);
	let allItems = $state<HistoryListItem[]>([]);
	let loading = $state(true);
	let selectedItem = $state<HistoryListItem | null>(null);
	let toastMessage = $state('');
	let toastState = $state<ToastState>('normal');
	let toastTimeout: ReturnType<typeof setTimeout> | null = null;
	let deleteTarget = $state<HistoryListItem | null>(null);
	let deleting = $state(false);
	let skipDeleteConfirm = $state(false);

	let captureLeaveGuard: ((proceed: () => void) => void) | null = null;

	function showToast(msg: string, state: ToastState = 'normal') {
		if (toastTimeout) clearTimeout(toastTimeout);
		toastMessage = msg;
		toastState = state;
		toastTimeout = setTimeout(() => {
			toastMessage = '';
			toastTimeout = null;
		}, 2500);
	}

	async function loadNotes() {
		loading = true;
		try {
			const items = await invoke<HistoryListItem[]>('history_list');
			allItems = items.sort((a, b) => b.created_at.localeCompare(a.created_at));
		} catch {
			showToast('Failed to load notes', 'error');
		} finally {
			loading = false;
		}
	}

	function navigate(next: AppRoute, opts?: { settingsTab?: SettingsTab }) {
		if (next === 'settings' && route !== 'settings') {
			previousRoute = route;
		}
		route = next;
		selectedItem = null;
		if (opts?.settingsTab) {
			settingsTab = opts.settingsTab;
		}
	}

	function guardedNavigate(next: AppRoute) {
		if (captureOpen && next !== route && captureLeaveGuard) {
			captureLeaveGuard(() => {
				captureOpen = false;
				navigate(next);
			});
			return;
		}
		navigate(next);
	}

	function openCapture() {
		captureVisitKey += 1;
		captureOpen = true;
	}

	function openNoteDetail(item: HistoryListItem) {
		selectedItem = item;
		navigate('notes');
	}

	async function copyItem(item: HistoryListItem) {
		try {
			await copyHistoryItem(item);
			showToast('Copied', 'success');
		} catch {
			showToast('Copy failed', 'error');
		}
	}

	async function openItem(item: HistoryListItem) {
		if (!item.markdown_path) return;
		try {
			await openHistoryMarkdown(item.markdown_path);
		} catch {
			showToast('Could not open file', 'error');
		}
	}

	function requestDelete(item: HistoryListItem) {
		if (skipDeleteConfirm) {
			void executeDelete(item);
		} else {
			deleteTarget = item;
		}
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		const target = deleteTarget;
		deleteTarget = null;
		await executeDelete(target);
	}

	async function executeDelete(item: HistoryListItem) {
		const wasViewing = selectedItem?.id === item.id;
		deleting = true;
		try {
			await deleteHistoryItem(item.id);
			await loadNotes();
			if (wasViewing) {
				selectedItem = null;
			}
			showToast('Deleted', 'success');
		} catch (e) {
			showToast('Delete failed: ' + String(e), 'error');
		} finally {
			deleting = false;
		}
	}

	function applyNavigateEvent(payload: AppNavigateEvent) {
		const next = payload.route;
		if (next === 'settings' && payload.settingsTab) {
			navigate(next, { settingsTab: payload.settingsTab });
		} else {
			navigate(next);
		}
	}

	onMount(() => {
		void loadNotes();
		const unlistenNoteP = listen('note://item-added', () => {
			void loadNotes();
		});
		const unlistenNavP = listen<AppNavigateEvent>('app://navigate', (event) => {
			applyNavigateEvent(event.payload);
		});
		return async () => {
			await (await unlistenNoteP)();
			await (await unlistenNavP)();
		};
	});
</script>

<div class="flex h-screen flex-col overflow-hidden bg-canvas">
	<ShellTitleBar onNewNote={openCapture} />
	<div class="flex min-h-0 flex-1 overflow-hidden">
		{#if route === 'settings'}
			<SettingsSidebar
				activeTab={settingsTab}
				ontabchange={(tab) => (settingsTab = tab)}
				onback={() => navigate(previousRoute)}
				backLabel={ROUTE_LABELS[previousRoute]}
			/>
		{:else}
			<AppSidebar activeRoute={route} onnavigate={guardedNavigate} />
		{/if}
		<main class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-canvas">
			{#if captureOpen}
				<CaptureScreen
					visitKey={captureVisitKey}
					onclose={() => (captureOpen = false)}
					registerLeaveGuard={(handler) => {
						captureLeaveGuard = handler;
					}}
				/>
			{:else if route === 'home'}
				<HomeScreen
					items={allItems}
					{loading}
					onselect={openNoteDetail}
					onseeall={() => navigate('notes')}
				/>
			{:else if route === 'notes'}
				<NotesScreen
					{allItems}
					{loading}
					bind:selectedItem
					oncopy={copyItem}
					onopen={openItem}
					ondelete={requestDelete}
					onrefresh={() => void loadNotes()}
					{deleting}
				/>
			{:else if route === 'upload'}
				<UploadScreen />
			{:else if route === 'float'}
				<div class="flex h-full items-center justify-center">
					<p class="sf-body-md text-fg-muted">Float — coming soon</p>
				</div>
			{:else}
				<SettingsPanel bind:activeTab={settingsTab} />
			{/if}
		</main>
	</div>
</div>

<Modal
	open={deleteTarget !== null}
	title="Delete recording?"
	description="This will permanently delete the transcript and any associated audio. This cannot be undone."
	maxWidthClass="max-w-sm"
	onClose={() => (deleteTarget = null)}
>
	{#snippet footer()}
		<div class="flex w-full items-center gap-4">
			<label class="mr-auto flex cursor-pointer items-center gap-2 sf-label-sm text-fg-dim">
				<input type="checkbox" bind:checked={skipDeleteConfirm} />
				Don't ask again this session
			</label>
			<div class="flex gap-3">
				<Button variant="normal" disabled={deleting} onclick={() => (deleteTarget = null)}>
					Cancel
				</Button>
				<Button variant="destructive" disabled={deleting} onclick={() => void confirmDelete()}>
					{deleting ? 'Deleting…' : 'Delete'}
				</Button>
			</div>
		</div>
	{/snippet}
</Modal>

<Toast message={toastMessage} state={toastState} position="bottom-center" />
