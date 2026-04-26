import { invoke } from '@tauri-apps/api/core';

export async function copyTranscript(path: string): Promise<void> {
	const content = await invoke<string>('scribe_read_transcript', { path });
	await navigator.clipboard.writeText(content);
}
