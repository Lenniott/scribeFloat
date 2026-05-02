import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ModelListItem, ModelProgressPayload } from '$lib/types';

export type { ModelListItem };

export function createModelDownloadStore() {
	let models = $state<ModelListItem[]>([]);
	let progressByModel = $state<Record<string, number>>({});
	let downloadingByModel = $state<Record<string, boolean>>({});
	let statusByModel = $state<Record<string, string>>({});
	let error = $state('');
	let activeDownloadModelId = $state<string | null>(null);
	let autoSelectInFlight = false;

	async function refresh() {
		const list = await invoke<ModelListItem[]>('model_list').catch(() => [] as ModelListItem[]);
		models = list;

		const prevDownloading = { ...downloadingByModel };

		let nextProg: Record<string, number> = {};
		let nextDown: Record<string, boolean> = {};
		let nextStat: Record<string, string> = {};

		for (const m of list) {
			if (m.downloaded) {
				nextProg[m.id] = 1;
				nextDown[m.id] = false;
				nextStat[m.id] = m.selected ? 'Installed and selected' : 'Installed';
			} else if (prevDownloading[m.id]) {
				nextDown[m.id] = true;
				nextProg[m.id] = progressByModel[m.id] ?? 0;
				nextStat[m.id] =
					statusByModel[m.id] ??
					`Installing… ${Math.round((progressByModel[m.id] ?? 0) * 100)}%`;
			}
		}

		progressByModel = nextProg;
		downloadingByModel = nextDown;
		statusByModel = nextStat;

		if (!autoSelectInFlight) {
			const downloaded = list.filter((m) => m.downloaded);
			const hasSelected = list.some((m) => m.downloaded && m.selected);
			if (downloaded.length === 1 && !hasSelected) {
				autoSelectInFlight = true;
				await invoke('model_select', { modelId: downloaded[0].id }).catch(() => {});
				autoSelectInFlight = false;
				await refresh();
			}
		}
	}

	async function download(modelId: string) {
		error = '';
		activeDownloadModelId = modelId;
		downloadingByModel = { ...downloadingByModel, [modelId]: true };
		progressByModel = { ...progressByModel, [modelId]: 0 };
		statusByModel = { ...statusByModel, [modelId]: 'Starting install…' };
		await invoke('model_download', { modelId }).catch((e) => {
			error = String(e);
			downloadingByModel = { ...downloadingByModel, [modelId]: false };
			statusByModel = { ...statusByModel, [modelId]: 'Install failed' };
			activeDownloadModelId = null;
		});
	}

	async function select(modelId: string) {
		error = '';
		await invoke('model_select', { modelId }).catch((e) => {
			error = String(e);
		});
		await refresh();
	}

	async function remove(modelId: string) {
		error = '';
		await invoke('model_remove', { modelId }).catch((e) => {
			error = String(e);
		});
		await refresh();
	}

	async function subscribe(): Promise<UnlistenFn[]> {
		const ul1 = await listen<ModelProgressPayload>('model://download-progress', (e) => {
			const { model_id, progress } = e.payload;
			progressByModel = { ...progressByModel, [model_id]: progress };
			statusByModel = {
				...statusByModel,
				[model_id]: `Installing… ${Math.round(progress * 100)}%`,
			};
			if (progress >= 1) {
				downloadingByModel = { ...downloadingByModel, [model_id]: false };
				statusByModel = { ...statusByModel, [model_id]: 'Installed' };
				activeDownloadModelId = null;
				refresh();
			}
		});

		const ul2 = await listen<string>('model://download-error', (e) => {
			error = e.payload ?? 'Model install failed';
			for (const id of Object.keys(downloadingByModel)) {
				if (downloadingByModel[id]) {
					downloadingByModel = { ...downloadingByModel, [id]: false };
					statusByModel = { ...statusByModel, [id]: 'Install failed' };
				}
			}
			activeDownloadModelId = null;
		});

		return [ul1, ul2];
	}

	return {
		get models() {
			return models;
		},
		get progressByModel() {
			return progressByModel;
		},
		get downloadingByModel() {
			return downloadingByModel;
		},
		get statusByModel() {
			return statusByModel;
		},
		get error() {
			return error;
		},
		set error(v: string) {
			error = v;
		},
		get activeDownloadModelId() {
			return activeDownloadModelId;
		},
		refresh,
		download,
		select,
		remove,
		subscribe,
	};
}
