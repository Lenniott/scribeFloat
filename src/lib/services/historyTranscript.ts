import { invoke } from '@tauri-apps/api/core';

/** Rendered transcript body for preview (no YAML front matter). */
export async function loadTranscriptPreview(id: string): Promise<string> {
	return invoke<string>('history_render_markdown', { id });
}
