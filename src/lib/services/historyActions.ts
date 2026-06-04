import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { copyTranscript } from '$lib/services/clipboard';
import { loadTranscriptPreview } from '$lib/services/historyTranscript';

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

export async function copyHistoryItem(item: HistoryListItem): Promise<void> {
	if (item.has_markdown && item.markdown_path) {
		await copyTranscript(item.markdown_path);
	} else {
		const text = await loadTranscriptPreview(item.id);
		await writeText(text);
	}
}

export async function openHistoryMarkdown(path: string): Promise<void> {
	await invoke('settings_open_transcript', { filePath: path });
}

export async function deleteHistoryItem(id: string): Promise<void> {
	await invoke('history_delete', { id });
}

export async function exportHistoryMarkdown(id: string): Promise<string> {
	return invoke<string>('history_export_markdown', { id });
}
