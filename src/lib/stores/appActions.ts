import { invoke } from '@tauri-apps/api/core';
import { goto } from '$app/navigation';
import {
	copyHistoryItem,
	deleteHistoryItem,
	openHistoryMarkdown,
	type HistoryListItem,
} from '@services/historyActions';
import { appState } from './appState.svelte';
import type { ToastState } from '@components/indicators/Toast.svelte';

export function showToast(msg: string, state: ToastState = 'normal') {
	if (appState.toastTimeout) clearTimeout(appState.toastTimeout);
	appState.toastMessage = msg;
	appState.toastState = state;
	appState.toastTimeout = setTimeout(() => {
		appState.toastMessage = '';
		appState.toastTimeout = null;
	}, 2500);
}

export async function loadNotes() {
	appState.loading = true;
	try {
		const items = await invoke<HistoryListItem[]>('history_list');
		appState.allItems = items.sort((a, b) => b.created_at.localeCompare(a.created_at));
	} catch {
		showToast('Failed to load notes', 'error');
	} finally {
		appState.loading = false;
	}
}

export async function copyItem(item: HistoryListItem) {
	try {
		await copyHistoryItem(item);
		showToast('Copied', 'success');
	} catch {
		showToast('Copy failed', 'error');
	}
}

export async function openItem(item: HistoryListItem) {
	if (!item.markdown_path) return;
	try {
		await openHistoryMarkdown(item.markdown_path);
	} catch {
		showToast('Could not open file', 'error');
	}
}

export function requestDelete(item: HistoryListItem) {
	if (appState.skipDeleteConfirm) {
		void executeDelete(item);
	} else {
		appState.deleteTarget = item;
	}
}

export async function executeDelete(item: HistoryListItem) {
	const wasViewing = appState.selectedItem?.id === item.id;
	appState.deleting = true;
	try {
		await deleteHistoryItem(item.id);
		await loadNotes();
		if (wasViewing) {
			appState.selectedItem = null;
		}
		showToast('Deleted', 'success');
	} catch (e) {
		showToast('Delete failed: ' + String(e), 'error');
	} finally {
		appState.deleting = false;
	}
}

export function openNoteDetail(item: HistoryListItem) {
	appState.selectedItem = item;
	void goto('/notes');
}
