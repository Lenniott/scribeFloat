import type { HistoryListItem } from '@services/historyActions';
import type { ToastState } from '@components/indicators/Toast.svelte';
import type { SettingsTab } from '@sections/settingsTypes';

class AppState {
	allItems = $state<HistoryListItem[]>([]);
	loading = $state(true);
	selectedItem = $state<HistoryListItem | null>(null);
	toastMessage = $state('');
	toastState = $state<ToastState>('normal');
	toastTimeout: ReturnType<typeof setTimeout> | null = null;
	deleteTarget = $state<HistoryListItem | null>(null);
	deleting = $state(false);
	skipDeleteConfirm = $state(false);
	settingsTab = $state<SettingsTab>('general');
	noteLeaveGuard = $state<((proceed: () => void, cancel: () => void) => void) | null>(null);
}

export const appState = new AppState();
