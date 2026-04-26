import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

export async function copyTranscript(path: string): Promise<void> {
	const content = await invoke<string>('scribe_read_transcript', { path });
	await writeText(content);
}
