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
	duration_secs: number;
	excerpt?: string | null;
	tags?: string[];
	has_markdown: boolean;
	markdown_path?: string;
	source: string;
};

export type DashboardStats = {
	transcript_count: number;
	recorded_this_week_secs: number | null;
	float_layers: number | null;
	drafts_to_review: number | null;
};

export type TagVocabularyEntry = {
	name: string;
	count: number;
};

export async function fetchDashboardStats(): Promise<DashboardStats> {
	return invoke<DashboardStats>('get_dashboard_stats');
}

export async function fetchTagVocabulary(): Promise<TagVocabularyEntry[]> {
	return invoke<TagVocabularyEntry[]>('history_tag_vocabulary');
}

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
